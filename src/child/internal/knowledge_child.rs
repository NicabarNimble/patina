//! Knowledge-child world — bindgen, engine, and WASM adapter.

use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::Store;

use super::{wasm_engine, ChildKind, ChildManifest, GrantedCapabilities, QueryDispatchFn};
use crate::mother::{
    ChildHealth, ChildRequest, ChildResponse, KnowledgeChild, MotherHost, PendingEvent, TaskIntent,
    TaskIntentKind,
};

mod bindings {
    use rayon::prelude::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[derive(Debug)]
    struct GithubPageResult {
        page: u32,
        raw_count: usize,
        rows: Vec<serde_json::Value>,
    }

    fn env_usize(name: &str, default: usize) -> usize {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(default)
    }

    fn backoff_seconds(headers: &reqwest::header::HeaderMap, attempt: usize) -> u64 {
        if let Some(retry_after) = headers
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            return retry_after.max(1);
        }

        if let (Some(remaining), Some(reset_epoch)) = (
            headers
                .get("x-ratelimit-remaining")
                .and_then(|h| h.to_str().ok()),
            headers
                .get("x-ratelimit-reset")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok()),
        ) {
            if remaining == "0" {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                if reset_epoch > now {
                    return (reset_epoch - now + 1).min(60);
                }
            }
        }

        (1_u64 << attempt.min(5)).min(30)
    }

    fn fetch_github_page(
        client: &reqwest::blocking::Client,
        credential: &crate::connect::ResolvedCredential,
        endpoint: &str,
        data_type: &str,
        since: Option<&str>,
        page: u32,
    ) -> Result<GithubPageResult, String> {
        let mut attempt = 0usize;
        loop {
            let mut url = reqwest::Url::parse(endpoint).map_err(|e| e.to_string())?;
            {
                let mut query = url.query_pairs_mut();
                query.append_pair("state", "all");
                query.append_pair("per_page", "100");
                query.append_pair("page", &page.to_string());
                query.append_pair("sort", "updated");
                query.append_pair("direction", "desc");
                if data_type == "issues" {
                    if let Some(cursor) = since {
                        if !cursor.trim().is_empty() {
                            query.append_pair("since", cursor);
                        }
                    }
                }
            }

            let mut request = client
                .get(url)
                .header("User-Agent", "patina-ducklake")
                .header("Accept", "application/vnd.github+json");
            request = match &credential.injection {
                crate::connect::InjectionStrategy::Bearer
                | crate::connect::InjectionStrategy::InProcess => request.header(
                    "Authorization",
                    format!("Bearer {}", credential.value.as_str()),
                ),
                crate::connect::InjectionStrategy::Header { name } => {
                    request.header(name, credential.value.as_str())
                }
            };

            let response = request.send().map_err(|e| e.to_string())?;
            let status = response.status().as_u16();
            let headers = response.headers().clone();
            let body = response.text().map_err(|e| e.to_string())?;

            if status == 401 {
                return Err("github auth denied (status 401)".into());
            }

            if status == 429 || status == 403 {
                let secondary = body.to_ascii_lowercase().contains("secondary rate limit");
                let primary_exhausted = headers
                    .get("x-ratelimit-remaining")
                    .and_then(|h| h.to_str().ok())
                    .is_some_and(|remaining| remaining == "0");
                if (secondary || primary_exhausted || status == 429) && attempt < 6 {
                    let sleep_secs = backoff_seconds(&headers, attempt);
                    std::thread::sleep(Duration::from_secs(sleep_secs));
                    attempt += 1;
                    continue;
                }
                return Err(format!("github auth/rate-limit denied (status {})", status));
            }

            if status >= 400 {
                return Err(format!("github sync failed (status {})", status));
            }

            let mut page_rows: Vec<serde_json::Value> = serde_json::from_str(&body)
                .map_err(|e| format!("invalid github payload: {}", e))?;
            let raw_count = page_rows.len();

            if data_type == "issues" {
                page_rows.retain(|row| row.get("pull_request").is_none());
            } else if let Some(cursor) = since {
                if !cursor.is_empty() {
                    page_rows.retain(|row| {
                        row.get("updated_at")
                            .and_then(|v| v.as_str())
                            .map(|updated| updated > cursor)
                            .unwrap_or(true)
                    });
                }
            }

            return Ok(GithubPageResult {
                page,
                raw_count,
                rows: page_rows,
            });
        }
    }

    pub struct HostState {
        pub plugin_name: String,
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_table: wasmtime::component::ResourceTable,
        pub project_root: Option<std::path::PathBuf>,
        pub grants: super::GrantedCapabilities,
        pub query_fn: Option<super::QueryDispatchFn>,
        pub http_client: reqwest::blocking::Client,
        pub runtime: crate::mother::KnowledgeRuntimeStore,
    }

    impl wasmtime_wasi::WasiView for HostState {
        fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
            wasmtime_wasi::WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.wasi_table,
            }
        }
    }

    wasmtime::component::bindgen!({
        path: "wit/knowledge-child/",
        world: "knowledge-child",
    });

    impl patina::host::log::Host for HostState {
        fn log(&mut self, level: patina::host::log::LogLevel, message: String) {
            let level_str = match level {
                patina::host::log::LogLevel::Debug => "DEBUG",
                patina::host::log::LogLevel::Info => "INFO",
                patina::host::log::LogLevel::Warn => "WARN",
                patina::host::log::LogLevel::Error => "ERROR",
            };
            crate::child::toy_host::v2::log_emit(&self.plugin_name, level_str, &message);
        }
    }

    impl patina::host::types::Host for HostState {}

    impl patina::host::query::Host for HostState {
        fn query(&mut self, kind: String, params: String) -> Result<String, String> {
            crate::child::toy_host::compat::query_dispatch(
                &self.plugin_name,
                &self.grants,
                &mut self.query_fn,
                &kind,
                &params,
            )
        }
    }

    impl patina::host::http::Host for HostState {
        fn http_post(
            &mut self,
            url: String,
            body: String,
            content_type: String,
        ) -> Result<patina::host::http::HttpResponse, String> {
            let r = crate::child::toy_host::compat::http_post(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &url,
                &body,
                &content_type,
            )?;
            Ok(patina::host::http::HttpResponse {
                status: r.status,
                body: r.body,
            })
        }

        fn http_get(&mut self, url: String) -> Result<patina::host::http::HttpResponse, String> {
            let r = crate::child::toy_host::compat::http_get(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &url,
            )?;
            Ok(patina::host::http::HttpResponse {
                status: r.status,
                body: r.body,
            })
        }
    }

    impl patina::host::emit::Host for HostState {
        fn emit_fact(
            &mut self,
            schema: String,
            fact_type: String,
            data: String,
        ) -> Result<u64, String> {
            if !self.grants.host_emit {
                return Err(format!(
                    "host_emit not granted for child '{}'",
                    self.plugin_name
                ));
            }
            super::super::host_support::emit_fact(
                &self.grants.schema_facts,
                &self.plugin_name,
                &schema,
                &fact_type,
                &data,
            )
        }
    }

    impl patina::host::measure::Host for HostState {
        fn record_measurement(
            &mut self,
            verb: String,
            tool: String,
            mode: String,
            metrics_json: String,
        ) -> Result<(), String> {
            super::super::host_support::record_measurement(
                &self.project_root,
                &self.plugin_name,
                &verb,
                &tool,
                &mode,
                &metrics_json,
            )
        }
    }

    impl patina::host::state::Host for HostState {
        fn get(&mut self, key: String) -> Option<String> {
            crate::child::toy_host::v2::state_get(&self.runtime, &self.plugin_name, &key)
                .ok()
                .flatten()
        }

        fn put(&mut self, key: String, value_json: String) -> Result<(), String> {
            if !self.grants.state_enabled {
                return Err(format!(
                    "state not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::toy_host::v2::state_set(
                &self.runtime,
                &self.plugin_name,
                &key,
                &value_json,
            )
        }

        fn delete(&mut self, key: String) -> Result<(), String> {
            if !self.grants.state_enabled {
                return Err(format!(
                    "state not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::toy_host::v2::state_delete(&self.runtime, &self.plugin_name, &key)
        }

        fn list_prefix(&mut self, prefix: String) -> Vec<String> {
            crate::child::toy_host::v2::state_list_prefix(&self.runtime, &self.plugin_name, &prefix)
                .unwrap_or_default()
        }
    }

    impl patina::host::checkpoint::Host for HostState {
        fn load(&mut self, stream: String) -> Option<String> {
            self.runtime
                .load_checkpoint(&self.plugin_name, &stream)
                .ok()
                .flatten()
        }

        fn save(&mut self, stream: String, checkpoint_json: String) -> Result<(), String> {
            if !self.grants.checkpoint_streams.contains(&stream) {
                return Err(format!(
                    "checkpoint stream '{}' not granted for child '{}'",
                    stream, self.plugin_name
                ));
            }
            self.runtime
                .save_checkpoint(&self.plugin_name, &stream, &checkpoint_json)
                .map_err(|e| e.to_string())
        }
    }

    impl patina::host::lake::Host for HostState {
        fn ensure_lake(&mut self, name: String) -> Result<String, String> {
            if !self.grants.lake_names.contains(&name) {
                return Err(format!(
                    "lake '{}' not granted for '{}'",
                    name, self.plugin_name
                ));
            }
            crate::child::toy_host::compat::lake_ensure_lake(&name)
        }

        fn list_granted_lakes(&mut self) -> Result<Vec<patina::host::lake::GrantedLake>, String> {
            self.grants
                .lake_names
                .iter()
                .cloned()
                .map(|name| {
                    let path = crate::child::toy_host::compat::lake_ensure_lake(&name)?;
                    Ok(patina::host::lake::GrantedLake { name, path })
                })
                .collect()
        }

        fn load_cursor(
            &mut self,
            lake: String,
            source: String,
            data_type: String,
        ) -> Option<String> {
            if !self.grants.lake_names.contains(&lake) {
                return None;
            }
            crate::child::toy_host::compat::lake_load_cursor(
                &self.runtime,
                &lake,
                &source,
                &data_type,
            )
            .ok()
            .flatten()
        }

        fn save_cursor(
            &mut self,
            lake: String,
            source: String,
            data_type: String,
            cursor: Option<String>,
            written: u64,
            status: String,
            last_error: Option<String>,
        ) -> Result<(), String> {
            if !self.grants.lake_names.contains(&lake) {
                return Err(format!(
                    "lake '{}' not granted for '{}'",
                    lake, self.plugin_name
                ));
            }
            crate::child::toy_host::compat::lake_save_cursor(
                &self.runtime,
                &crate::mother::state::LakeCursorUpdate {
                    lake_name: &lake,
                    source_name: &source,
                    data_type: &data_type,
                    cursor_value: cursor.as_deref(),
                    records_written: written,
                    status: &status,
                    last_error: last_error.as_deref(),
                },
            )
        }

        fn ensure_table(&mut self, lake: String, table: String) -> Result<(), String> {
            if !self.grants.lake_names.contains(&lake) {
                return Err(format!(
                    "lake '{}' not granted for '{}'",
                    lake, self.plugin_name
                ));
            }
            crate::child::toy_host::compat::lake_ensure_table(&lake, &table)
        }

        fn append_json_batch(
            &mut self,
            lake: String,
            table: String,
            source: String,
            rows_json: Vec<String>,
        ) -> Result<u64, String> {
            if !self.grants.lake_names.contains(&lake) {
                return Err(format!(
                    "lake '{}' not granted for '{}'",
                    lake, self.plugin_name
                ));
            }
            crate::child::toy_host::compat::lake_append_json_batch(
                &lake, &table, &source, &rows_json,
            )
        }

        fn query_json(&mut self, lake: String, sql: String) -> Result<String, String> {
            if !self.grants.lake_names.contains(&lake) {
                return Err(format!(
                    "lake '{}' not granted for '{}'",
                    lake, self.plugin_name
                ));
            }
            crate::child::toy_host::compat::lake_query_json(&lake, &sql)
        }
    }

    impl patina::host::ingress::Host for HostState {
        fn list_granted_sources(&mut self) -> Vec<patina::host::ingress::GrantedSource> {
            crate::child::toy_host::compat::ingress_list_granted_sources(&self.grants.toys)
                .into_iter()
                .map(|(name, endpoint)| patina::host::ingress::GrantedSource { name, endpoint })
                .collect()
        }

        fn fetch(&mut self, source_name: String) -> Result<String, String> {
            let endpoint = crate::child::toy_host::compat::ingress_resolve_source_endpoint(
                &self.grants.toys,
                &self.plugin_name,
                &source_name,
            )?;
            crate::child::toy_host::compat::ingress_fetch_via_http(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &endpoint,
            )
        }
    }

    impl patina::host::connector::Host for HostState {
        fn list_bindings(&mut self) -> Result<Vec<patina::host::connector::RepoBinding>, String> {
            crate::child::toy_host::compat::connector_ensure_granted(
                self.grants.toys.connector,
                &self.plugin_name,
            )?;
            crate::child::toy_host::compat::connector_list_bindings(
                &self.runtime,
                &self.plugin_name,
            )
            .map(|items| {
                items
                    .into_iter()
                    .map(|parsed| patina::host::connector::RepoBinding {
                        binding_id: parsed.binding_id,
                        connection: parsed.connection,
                        owner: parsed.owner,
                        repo: parsed.repo,
                        types: parsed.types,
                    })
                    .collect()
            })
        }

        fn upsert_binding(
            &mut self,
            binding: patina::host::connector::RepoBinding,
        ) -> Result<patina::host::connector::RepoBinding, String> {
            crate::child::toy_host::compat::connector_ensure_granted(
                self.grants.toys.connector,
                &self.plugin_name,
            )?;
            let record = crate::child::toy_host::connector::ConnectorBindingRecord {
                binding_id: binding.binding_id.clone(),
                connection: binding.connection.clone(),
                owner: binding.owner.clone(),
                repo: binding.repo.clone(),
                types: binding.types.clone(),
            };
            crate::child::toy_host::compat::connector_upsert_binding(
                &self.runtime,
                &self.plugin_name,
                record,
            )?;
            Ok(binding)
        }

        fn remove_binding(&mut self, binding_id: String) -> Result<(), String> {
            crate::child::toy_host::compat::connector_ensure_granted(
                self.grants.toys.connector,
                &self.plugin_name,
            )?;
            crate::child::toy_host::compat::connector_remove_binding(
                &self.runtime,
                &self.plugin_name,
                &binding_id,
            )
        }

        fn sync_binding(
            &mut self,
            binding_id: String,
            data_type: String,
            since: Option<String>,
        ) -> Result<patina::host::connector::SyncResult, String> {
            crate::child::toy_host::compat::connector_ensure_granted(
                self.grants.toys.connector,
                &self.plugin_name,
            )?;

            let binding = crate::child::toy_host::compat::connector_load_binding(
                &self.runtime,
                &self.plugin_name,
                &binding_id,
            )?;
            crate::child::toy_host::compat::connector_ensure_type_enabled(&binding, &data_type)?;

            let connection = crate::connect::load(&binding.connection)
                .map_err(|e| format!("connection '{}': {}", binding.connection, e))?;
            let credential =
                crate::connect::resolve_credential_for_domain(&connection, "api.github.com")?;

            let endpoint =
                crate::child::toy_host::compat::connector_endpoint_for(&binding, &data_type)?;

            let max_pages = env_usize("PATINA_GITHUB_MAX_PAGES", 500).clamp(1, 5_000) as u32;
            let max_parallel = env_usize("PATINA_GITHUB_MAX_PARALLEL", 4).clamp(1, 16);
            let mut rows: Vec<serde_json::Value> = Vec::new();
            let mut next_page = 1_u32;
            let since_ref = since.as_deref();

            while next_page <= max_pages {
                let upper = (next_page + max_parallel as u32 - 1).min(max_pages);
                let pages: Vec<u32> = (next_page..=upper).collect();
                let client = self.http_client.clone();
                let credential = credential.clone();
                let endpoint_cloned = endpoint.clone();
                let data_type_cloned = data_type.clone();

                let mut batch = pages
                    .into_par_iter()
                    .map(|page| {
                        fetch_github_page(
                            &client,
                            &credential,
                            &endpoint_cloned,
                            &data_type_cloned,
                            since_ref,
                            page,
                        )
                    })
                    .collect::<Vec<_>>();

                batch.sort_by_key(|item| item.as_ref().map(|p| p.page).unwrap_or(u32::MAX));
                let mut reached_end = false;
                for page in batch {
                    let page = page.map_err(|error| {
                        format!(
                            "github sync failed for {}/{} {} via connection '{}': {}",
                            binding.owner, binding.repo, data_type, binding.connection, error
                        )
                    })?;
                    let raw_count = page.raw_count;
                    rows.extend(page.rows);
                    if raw_count < 100 {
                        reached_end = true;
                        break;
                    }
                }

                if reached_end {
                    break;
                }
                next_page = upper + 1;
            }

            if next_page > max_pages {
                return Err(format!(
                    "github sync exceeded max pages ({}) for {}/{} {}. Increase PATINA_GITHUB_MAX_PAGES to continue.",
                    max_pages, binding.owner, binding.repo, data_type
                ));
            }

            let cursor = rows
                .iter()
                .filter_map(|row| row.get("updated_at").and_then(|v| v.as_str()))
                .max()
                .map(ToString::to_string)
                .or(since);
            let rows_json = rows
                .into_iter()
                .map(|row| serde_json::to_string(&row).map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(patina::host::connector::SyncResult {
                binding_id: binding.binding_id,
                data_type,
                cursor,
                rows_json,
            })
        }
    }

    impl patina::host::github::Host for HostState {
        fn list_issues(
            &mut self,
            owner: String,
            repo: String,
            params: patina::host::github::ListParams,
        ) -> Result<patina::host::github::Page, String> {
            crate::child::toy_host::compat::github_ensure_granted(
                self.grants.toys.github,
                &self.plugin_name,
            )?;
            let page = crate::child::toy_host::compat::github_list_issues(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &owner,
                &repo,
                &crate::child::toy_host::github::ListParams {
                    since: params.since,
                    state: params.state,
                    page: params.page,
                    per_page: params.per_page,
                },
            )?;
            Ok(patina::host::github::Page {
                items: page.items,
                has_next: page.has_next,
                next_page: page.next_page,
                rate_remaining: page.rate_remaining,
            })
        }

        fn list_pulls(
            &mut self,
            owner: String,
            repo: String,
            params: patina::host::github::ListParams,
        ) -> Result<patina::host::github::Page, String> {
            crate::child::toy_host::compat::github_ensure_granted(
                self.grants.toys.github,
                &self.plugin_name,
            )?;
            let page = crate::child::toy_host::compat::github_list_pulls(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &owner,
                &repo,
                &crate::child::toy_host::github::ListParams {
                    since: params.since,
                    state: params.state,
                    page: params.page,
                    per_page: params.per_page,
                },
            )?;
            Ok(patina::host::github::Page {
                items: page.items,
                has_next: page.has_next,
                next_page: page.next_page,
                rate_remaining: page.rate_remaining,
            })
        }

        fn list_issue_comments(
            &mut self,
            owner: String,
            repo: String,
            issue_number: u32,
        ) -> Result<patina::host::github::Page, String> {
            crate::child::toy_host::compat::github_ensure_granted(
                self.grants.toys.github,
                &self.plugin_name,
            )?;
            let page = crate::child::toy_host::compat::github_list_issue_comments(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &owner,
                &repo,
                issue_number,
            )?;
            Ok(patina::host::github::Page {
                items: page.items,
                has_next: page.has_next,
                next_page: page.next_page,
                rate_remaining: page.rate_remaining,
            })
        }

        fn list_issue_events(
            &mut self,
            owner: String,
            repo: String,
            issue_number: u32,
        ) -> Result<patina::host::github::Page, String> {
            crate::child::toy_host::compat::github_ensure_granted(
                self.grants.toys.github,
                &self.plugin_name,
            )?;
            let page = crate::child::toy_host::compat::github_list_issue_events(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &owner,
                &repo,
                issue_number,
            )?;
            Ok(patina::host::github::Page {
                items: page.items,
                has_next: page.has_next,
                next_page: page.next_page,
                rate_remaining: page.rate_remaining,
            })
        }

        fn list_pull_comments(
            &mut self,
            owner: String,
            repo: String,
            pull_number: u32,
        ) -> Result<patina::host::github::Page, String> {
            crate::child::toy_host::compat::github_ensure_granted(
                self.grants.toys.github,
                &self.plugin_name,
            )?;
            let page = crate::child::toy_host::compat::github_list_pull_comments(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &owner,
                &repo,
                pull_number,
            )?;
            Ok(patina::host::github::Page {
                items: page.items,
                has_next: page.has_next,
                next_page: page.next_page,
                rate_remaining: page.rate_remaining,
            })
        }

        fn list_reviews(
            &mut self,
            owner: String,
            repo: String,
            pull_number: u32,
        ) -> Result<patina::host::github::Page, String> {
            crate::child::toy_host::compat::github_ensure_granted(
                self.grants.toys.github,
                &self.plugin_name,
            )?;
            let page = crate::child::toy_host::compat::github_list_reviews(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &owner,
                &repo,
                pull_number,
            )?;
            Ok(patina::host::github::Page {
                items: page.items,
                has_next: page.has_next,
                next_page: page.next_page,
                rate_remaining: page.rate_remaining,
            })
        }

        fn list_review_comments(
            &mut self,
            owner: String,
            repo: String,
            pull_number: u32,
            review_id: u64,
        ) -> Result<patina::host::github::Page, String> {
            crate::child::toy_host::compat::github_ensure_granted(
                self.grants.toys.github,
                &self.plugin_name,
            )?;
            let page = crate::child::toy_host::compat::github_list_review_comments(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &owner,
                &repo,
                pull_number,
                review_id,
            )?;
            Ok(patina::host::github::Page {
                items: page.items,
                has_next: page.has_next,
                next_page: page.next_page,
                rate_remaining: page.rate_remaining,
            })
        }
    }

    impl patina::host::session::Host for HostState {
        fn get_session_id(&mut self) -> String {
            crate::child::toy_host::compat::session_get_session_id()
        }

        fn get_previous_session(&mut self) -> Option<String> {
            crate::child::toy_host::compat::session_get_previous_session()
        }

        fn get_previous_session_runtime_id(&mut self) -> Option<String> {
            crate::child::toy_host::compat::session_get_previous_session_runtime_id()
        }

        fn get_previous_session_handoff(&mut self) -> Option<String> {
            crate::child::toy_host::compat::session_get_previous_session_handoff()
        }

        fn write_artifact(&mut self, section: String, content: String) -> Result<(), String> {
            crate::child::toy_host::compat::session_ensure_granted(
                self.grants.toys.session,
                &self.plugin_name,
            )?;
            crate::child::toy_host::compat::session_write_artifact(&section, &content)
        }

        fn set_parent_session(&mut self, runtime_id: String) -> Result<(), String> {
            crate::child::toy_host::compat::session_ensure_granted(
                self.grants.toys.session,
                &self.plugin_name,
            )?;
            crate::child::toy_host::compat::session_set_parent_session(&runtime_id)
        }

        fn create_tag(&mut self, name: String) -> Result<(), String> {
            crate::child::toy_host::compat::session_ensure_granted(
                self.grants.toys.session,
                &self.plugin_name,
            )?;
            crate::child::toy_host::compat::session_create_tag(&name)
        }

        fn set_status(&mut self, status: String) -> Result<(), String> {
            crate::child::toy_host::compat::session_ensure_granted(
                self.grants.toys.session,
                &self.plugin_name,
            )?;
            crate::child::toy_host::compat::session_set_status(&status)
        }

        fn write_handoff(&mut self, modified_files: String, summary: String) -> Result<(), String> {
            crate::child::toy_host::compat::session_ensure_granted(
                self.grants.toys.session,
                &self.plugin_name,
            )?;
            crate::child::toy_host::compat::session_write_handoff(&modified_files, &summary)
        }
    }

    impl patina::host::events::Host for HostState {
        fn pull(
            &mut self,
            stream: String,
            after_offset: Option<u64>,
            limit: u32,
        ) -> Result<Vec<patina::host::events::PendingEvent>, String> {
            if !self.grants.subscribed_streams.contains(&stream) {
                return Err(format!(
                    "event stream '{}' not granted for child '{}'",
                    stream, self.plugin_name
                ));
            }
            let events =
                crate::child::toy_host::v2::events_subscribe(&stream, after_offset, limit)?;
            Ok(events
                .into_iter()
                .map(|event| patina::host::events::PendingEvent {
                    stream_name: event.stream_name,
                    offset: event.offset,
                    event_type: event.event_type,
                    payload_json: event.payload,
                    occurred_at: event.occurred_at,
                })
                .collect())
        }

        fn ack_through(&mut self, stream: String, offset: u64) -> Result<(), String> {
            if !self.grants.subscribed_streams.contains(&stream) {
                return Err(format!(
                    "event stream '{}' not granted for child '{}'",
                    stream, self.plugin_name
                ));
            }
            crate::child::toy_host::v2::events_ack(
                &self.runtime,
                &self.plugin_name,
                &stream,
                offset,
            )
        }

        fn list_streams(&mut self) -> Vec<String> {
            vec![
                "belief.changed".to_string(),
                "graph.changed".to_string(),
                "fact.ingested".to_string(),
                "session.completed".to_string(),
                "repo.synced".to_string(),
            ]
        }
    }

    impl patina::host::peer::Host for HostState {
        fn emit_event(&mut self, event_type: String, payload_json: String) -> Result<(), String> {
            let conn =
                crate::eventlog::open_events_db().map_err(|e| format!("open events.db: {}", e))?;
            let timestamp = chrono::Utc::now().to_rfc3339();
            let source_id = format!("plugin:{}", self.plugin_name);
            conn.execute(
                "INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data, provenance)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    &event_type,
                    &timestamp,
                    &source_id,
                    Option::<&str>::None,
                    &payload_json,
                    "external"
                ],
            )
            .map_err(|e| format!("insert peer event: {}", e))?;
            Ok(())
        }

        fn on_event(
            &mut self,
            stream_name: String,
            after_offset: Option<u64>,
            limit: u32,
        ) -> Result<Vec<patina::host::peer::PeerEvent>, String> {
            let events =
                crate::child::toy_host::compat::events_pull(&stream_name, after_offset, limit)
                    .map_err(|e| format!("peer on_event: {}", e))?;
            Ok(events
                .into_iter()
                .map(|event| patina::host::peer::PeerEvent {
                    stream_name: event.stream,
                    offset: event.offset,
                    event_type: event.event_type,
                    payload_json: serde_json::to_string(&event.payload)
                        .unwrap_or_else(|_| "null".into()),
                    occurred_at: event.occurred_at,
                })
                .collect())
        }
    }

    impl patina::host::task::Host for HostState {
        fn enqueue(&mut self, intent: patina::host::task::TaskIntent) -> Result<String, String> {
            let kind = match intent.kind {
                patina::host::task::TaskIntentKind::FetchSource => "fetch-source",
                patina::host::task::TaskIntentKind::RunQuery => "run-query",
                patina::host::task::TaskIntentKind::EmitFacts => "emit-facts",
                patina::host::task::TaskIntentKind::MaterializeIndex => "materialize-index",
                patina::host::task::TaskIntentKind::VerifyBelief => "verify-belief",
                patina::host::task::TaskIntentKind::SyncGraph => "sync-graph",
                patina::host::task::TaskIntentKind::RefreshCredential => "refresh-credential",
                patina::host::task::TaskIntentKind::NativeJob => "native-job",
            };
            let resolved = crate::mother::TaskIntentKind::parse(kind)
                .ok_or_else(|| format!("unknown task intent kind '{}'", kind))?;
            if !self.grants.task_intents.contains(&resolved) {
                return Err(format!(
                    "task intent '{}' not granted for '{}'",
                    resolved, self.plugin_name
                ));
            }
            crate::child::toy_host::v2::task_enqueue(
                &self.runtime,
                &self.plugin_name,
                kind,
                &intent.payload_json,
                intent.dedupe_key,
            )
        }
    }

    impl patina::host::graph::Host for HostState {
        fn query(&mut self, kind: String, params_json: String) -> Result<String, String> {
            if !self.grants.graph_read {
                return Err(format!("graph read not granted for '{}'", self.plugin_name));
            }
            crate::beliefs::graph_host::query(&kind, &params_json).map_err(|e| e.to_string())
        }

        fn mutate(&mut self, action: String, payload_json: String) -> Result<(), String> {
            if !self.grants.graph_write_actions.contains(&action) {
                return Err(format!(
                    "graph action '{}' not granted for '{}'",
                    action, self.plugin_name
                ));
            }
            crate::beliefs::graph_host::mutate(
                &self.runtime,
                &self.plugin_name,
                &action,
                &payload_json,
            )
            .map_err(|e| e.to_string())
        }
    }

    impl patina::host::belief::Host for HostState {
        fn query(&mut self, kind: String, params_json: String) -> Result<String, String> {
            if !self.grants.belief_read {
                return Err(format!(
                    "belief read not granted for '{}'",
                    self.plugin_name
                ));
            }
            crate::beliefs::belief_host::query(&kind, &params_json).map_err(|e| e.to_string())
        }

        fn mutate(&mut self, action: String, payload_json: String) -> Result<(), String> {
            if !self.grants.belief_write_actions.contains(&action) {
                return Err(format!(
                    "belief action '{}' not granted for '{}'",
                    action, self.plugin_name
                ));
            }
            crate::beliefs::belief_host::mutate(
                &self.runtime,
                &self.plugin_name,
                &action,
                &payload_json,
            )
            .map_err(|e| e.to_string())
        }
    }
}

use bindings::HostState;

pub struct KnowledgeChildEngine {
    _unit: (),
}

impl KnowledgeChildEngine {
    fn link_wasi(linker: &mut Linker<HostState>) -> Result<()> {
        wasmtime_wasi::p2::add_to_linker_sync(linker)?;
        Ok(())
    }

    fn link_log(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::log::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_measure(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::measure::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_query(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::query::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_http(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::http::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_ingress(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::ingress::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_connector(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::connector::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_github(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::github::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_emit(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::emit::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_state(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::state::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_checkpoint(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::checkpoint::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_lake(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::lake::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_events(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::events::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_peer(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::peer::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_task(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::task::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_graph(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::graph::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_belief(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::belief::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_types(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::types::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_session(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::host::session::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn build_linker(manifest: &ChildManifest) -> Result<Linker<HostState>> {
        let mut linker = Linker::new(wasm_engine());
        Self::link_wasi(&mut linker)?;
        Self::link_types(&mut linker)?;

        let wants_log = manifest.capabilities.iter().any(|cap| cap == "host_log");
        let wants_measure = manifest
            .capabilities
            .iter()
            .any(|cap| cap == "host_measure")
            || manifest.toys.measure;
        let wants_query = manifest.capabilities.iter().any(|cap| cap == "host_query")
            || !manifest.host_query_kinds.is_empty()
            || manifest.toys.query;
        let wants_http = manifest.capabilities.iter().any(|cap| cap == "host_http")
            || !manifest.host_http_domains.is_empty()
            || !manifest.ingress_sources.is_empty();
        let wants_emit = manifest.capabilities.iter().any(|cap| cap == "host_emit");
        let wants_state = manifest.state_enabled;
        let wants_checkpoint = !manifest.checkpoint_streams.is_empty();
        let wants_lake = !manifest.lake_names.is_empty();
        let wants_ingress = !manifest.ingress_sources.is_empty();
        let wants_connector = manifest.toys.connector;
        let wants_github = manifest.toys.github;
        let wants_events = !manifest.subscribed_streams.is_empty();
        let wants_peer = wants_events;
        let wants_task = !manifest.task_intent_names.is_empty();
        let wants_graph =
            manifest.graph_read || !manifest.graph_write_actions.is_empty() || manifest.toys.graph;
        let wants_belief = manifest.belief_read
            || !manifest.belief_write_actions.is_empty()
            || manifest.toys.belief;
        let wants_session = manifest.toys.session;

        if wants_log {
            Self::link_log(&mut linker)?;
        }
        if wants_measure {
            Self::link_measure(&mut linker)?;
        }
        if wants_query {
            Self::link_query(&mut linker)?;
        }
        if wants_http {
            Self::link_http(&mut linker)?;
        }
        if wants_ingress {
            Self::link_ingress(&mut linker)?;
        }
        if wants_connector {
            Self::link_connector(&mut linker)?;
        }
        if wants_github {
            Self::link_github(&mut linker)?;
        }
        if wants_emit {
            Self::link_emit(&mut linker)?;
        }
        if wants_state {
            Self::link_state(&mut linker)?;
        }
        if wants_checkpoint {
            Self::link_checkpoint(&mut linker)?;
        }
        if wants_lake {
            Self::link_lake(&mut linker)?;
        }
        if wants_events {
            Self::link_events(&mut linker)?;
        }
        if wants_peer {
            Self::link_peer(&mut linker)?;
        }
        if wants_task {
            Self::link_task(&mut linker)?;
        }
        if wants_graph {
            Self::link_graph(&mut linker)?;
        }
        if wants_belief {
            Self::link_belief(&mut linker)?;
        }
        if wants_session {
            Self::link_session(&mut linker)?;
        }

        Ok(linker)
    }

    pub fn new() -> Result<Self> {
        Ok(Self { _unit: () })
    }

    pub fn load_manifest(path: &Path) -> Result<ChildManifest> {
        ChildManifest::from_path(path)
    }

    pub fn load_component(&self, wasm: &[u8]) -> Result<Component> {
        ChildManifest::load_component(wasm)
    }

    pub fn check_capabilities(manifest: &ChildManifest) -> Result<()> {
        if manifest.world != ChildKind::KnowledgeChild {
            anyhow::bail!(
                "child '{}' has world '{}', expected 'knowledge-child'",
                manifest.name,
                manifest.world
            );
        }

        let allowed = manifest.world.allowed_capabilities();
        let world_denied: Vec<&str> = manifest
            .capabilities
            .iter()
            .filter(|cap| !allowed.contains(&cap.as_str()))
            .map(|s| s.as_str())
            .collect();
        if !world_denied.is_empty() {
            anyhow::bail!(
                "child '{}' (world '{}') requests capabilities not allowed for this world: {}",
                manifest.name,
                manifest.world,
                world_denied.join(", ")
            );
        }

        const AUTO_GRANTED: &[&str] = &[
            "host_log",
            "host_measure",
            "host_emit",
            "host_http",
            "host_query",
        ];
        let denied: Vec<&str> = manifest
            .capabilities
            .iter()
            .filter(|cap| !AUTO_GRANTED.contains(&cap.as_str()))
            .map(|s| s.as_str())
            .collect();
        if !denied.is_empty() {
            anyhow::bail!(
                "child '{}' requests capabilities not granted: {}",
                manifest.name,
                denied.join(", ")
            );
        }

        const KNOWN_STREAMS: &[&str] = &[
            "belief.changed",
            "graph.changed",
            "fact.ingested",
            "session.completed",
            "repo.synced",
        ];
        for stream in &manifest.subscribed_streams {
            if !KNOWN_STREAMS.contains(&stream.as_str()) {
                anyhow::bail!(
                    "child '{}' requests unknown event stream '{}'",
                    manifest.name,
                    stream
                );
            }
        }
        for source in manifest.ingress_sources.values() {
            super::host_support::validate_http_url(&source.endpoint).map_err(|error| {
                anyhow::anyhow!(
                    "child '{}' declares invalid ingress source '{}' endpoint '{}': {}",
                    manifest.name,
                    source.name,
                    source.endpoint,
                    error
                )
            })?;
        }
        let unknown_intents: Vec<&str> = manifest
            .task_intent_names
            .iter()
            .filter(|intent| TaskIntentKind::parse(intent).is_none())
            .map(|s| s.as_str())
            .collect();
        if !unknown_intents.is_empty() {
            anyhow::bail!(
                "child '{}' requests unknown task intents: {}",
                manifest.name,
                unknown_intents.join(", ")
            );
        }

        const GRAPH_ACTIONS: &[&str] = &["link", "unlink", "weight", "tag"];
        let unknown_graph: Vec<&str> = manifest
            .graph_write_actions
            .iter()
            .filter(|action| !GRAPH_ACTIONS.contains(&action.as_str()))
            .map(|s| s.as_str())
            .collect();
        if !unknown_graph.is_empty() {
            anyhow::bail!(
                "child '{}' requests unknown graph write actions: {}",
                manifest.name,
                unknown_graph.join(", ")
            );
        }

        const BELIEF_ACTIONS: &[&str] = &[
            "attach-evidence",
            "record-verification",
            "link-related",
            "supersede",
        ];
        let unknown_belief: Vec<&str> = manifest
            .belief_write_actions
            .iter()
            .filter(|action| !BELIEF_ACTIONS.contains(&action.as_str()))
            .map(|s| s.as_str())
            .collect();
        if !unknown_belief.is_empty() {
            anyhow::bail!(
                "child '{}' requests unknown belief write actions: {}",
                manifest.name,
                unknown_belief.join(", ")
            );
        }

        if manifest.capabilities.contains(&"host_emit".to_string()) && manifest.schemas.is_empty() {
            anyhow::bail!(
                "child '{}' declares host_emit but has no [schemas.*] entries",
                manifest.name
            );
        }

        Ok(())
    }

    pub fn instantiate_child(
        &self,
        component: &Component,
        manifest: &ChildManifest,
        query_fn: Option<QueryDispatchFn>,
    ) -> Result<Box<dyn KnowledgeChild>> {
        Self::check_capabilities(manifest)?;

        let linker = Self::build_linker(manifest)?;

        let grants = manifest.granted_capabilities();
        let http_client = super::host_support::build_http_client()?;
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build();
        let project_root = crate::session::SessionManager::find_project_root().ok();
        let host_state = HostState {
            plugin_name: manifest.name.clone(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            project_root,
            grants,
            query_fn,
            http_client,
            runtime: crate::mother::KnowledgeRuntimeStore::default(),
        };
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = bindings::KnowledgeChild::instantiate(&mut store, component, &linker)?;
        instance.call_init(&mut store)?;
        let name = instance.call_name(&mut store)?;
        Ok(Box::new(WasmKnowledgeChild {
            name,
            inner: Mutex::new(WasmKnowledgeChildInner { store, instance }),
        }))
    }
}

struct WasmKnowledgeChild {
    name: String,
    inner: Mutex<WasmKnowledgeChildInner>,
}

struct WasmKnowledgeChildInner {
    store: Store<HostState>,
    instance: bindings::KnowledgeChild,
}

impl KnowledgeChild for WasmKnowledgeChild {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        match instance.call_on_load(store)? {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("WASM on_load failed: {}", e)),
        }
    }

    fn on_unload(&mut self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        let _ = instance.call_on_unload(store);
    }

    fn health(&self) -> ChildHealth {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        match instance.call_health(store) {
            Ok(h) => {
                let reason = h.reason.unwrap_or_default();
                match h.status {
                    bindings::patina::host::types::HealthStatus::Healthy => ChildHealth::Healthy,
                    bindings::patina::host::types::HealthStatus::Degraded => {
                        ChildHealth::Degraded(if reason.is_empty() {
                            "degraded".into()
                        } else {
                            reason
                        })
                    }
                    bindings::patina::host::types::HealthStatus::Unhealthy => {
                        ChildHealth::Unhealthy(if reason.is_empty() {
                            "unhealthy".into()
                        } else {
                            reason
                        })
                    }
                }
            }
            Err(e) => ChildHealth::Unhealthy(format!("WASM call failed: {}", e)),
        }
    }

    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        let payload_json = serde_json::to_string(&request.payload)?;
        match instance.call_handle(store, &request.action, &payload_json)? {
            Ok(json) => Ok(ChildResponse {
                payload: serde_json::from_str(&json)?,
            }),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }

    fn drain(&mut self, limit: u32) -> Result<Vec<PendingEvent>> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        match instance.call_drain(store, limit)? {
            Ok(events) => Ok(events
                .into_iter()
                .map(|event| PendingEvent {
                    stream: event.stream_name,
                    offset: event.offset,
                    event_type: event.event_type,
                    payload: serde_json::from_str(&event.payload_json)
                        .unwrap_or(serde_json::Value::Null),
                    occurred_at: event.occurred_at,
                })
                .collect()),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }

    fn tick(&mut self) -> Vec<TaskIntent> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        match instance.call_tick(store) {
            Ok(intents) => intents
                .into_iter()
                .filter_map(|intent| {
                    Some(TaskIntent {
                        kind: match intent.kind {
                            bindings::patina::host::task::TaskIntentKind::FetchSource => {
                                crate::mother::TaskIntentKind::FetchSource
                            }
                            bindings::patina::host::task::TaskIntentKind::RunQuery => {
                                crate::mother::TaskIntentKind::RunQuery
                            }
                            bindings::patina::host::task::TaskIntentKind::EmitFacts => {
                                crate::mother::TaskIntentKind::EmitFacts
                            }
                            bindings::patina::host::task::TaskIntentKind::MaterializeIndex => {
                                crate::mother::TaskIntentKind::MaterializeIndex
                            }
                            bindings::patina::host::task::TaskIntentKind::VerifyBelief => {
                                crate::mother::TaskIntentKind::VerifyBelief
                            }
                            bindings::patina::host::task::TaskIntentKind::SyncGraph => {
                                crate::mother::TaskIntentKind::SyncGraph
                            }
                            bindings::patina::host::task::TaskIntentKind::RefreshCredential => {
                                crate::mother::TaskIntentKind::RefreshCredential
                            }
                            bindings::patina::host::task::TaskIntentKind::NativeJob => {
                                crate::mother::TaskIntentKind::NativeJob
                            }
                        },
                        payload: serde_json::from_str(&intent.payload_json).ok()?,
                        dedupe_key: intent.dedupe_key,
                    })
                })
                .collect(),
            Err(e) => {
                eprintln!("[child:{}] tick failed: {}", self.name, e);
                vec![]
            }
        }
    }
}
