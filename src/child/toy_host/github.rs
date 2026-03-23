use crate::child::engine::GrantedCapabilities;

#[derive(Debug, Clone, Default)]
pub struct ListParams {
    pub since: Option<String>,
    pub state: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub items: String,
    pub has_next: bool,
    pub next_page: Option<u32>,
    pub rate_remaining: u32,
}

pub fn ensure_granted(github_granted: bool, plugin_name: &str) -> Result<(), String> {
    if github_granted {
        Ok(())
    } else {
        Err(format!(
            "github toy not granted for plugin '{}'",
            plugin_name
        ))
    }
}

pub fn list_issues(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    params: &ListParams,
) -> Result<Page, String> {
    list(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        "issues",
        params,
    )
}

pub fn list_pulls(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    params: &ListParams,
) -> Result<Page, String> {
    list(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        "pulls",
        params,
    )
}

pub fn list_issue_comments(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    issue_number: u32,
) -> Result<Page, String> {
    list_raw_path(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        &format!("issues/{}/comments", issue_number),
        &ListParams::default(),
    )
}

pub fn list_issue_events(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    issue_number: u32,
) -> Result<Page, String> {
    list_raw_path(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        &format!("issues/{}/events", issue_number),
        &ListParams::default(),
    )
}

pub fn list_pull_comments(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    pull_number: u32,
) -> Result<Page, String> {
    list_raw_path(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        &format!("pulls/{}/comments", pull_number),
        &ListParams::default(),
    )
}

pub fn list_reviews(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    pull_number: u32,
) -> Result<Page, String> {
    list_raw_path(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        &format!("pulls/{}/reviews", pull_number),
        &ListParams::default(),
    )
}

pub fn list_review_comments(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    pull_number: u32,
    review_id: u64,
) -> Result<Page, String> {
    list_raw_path(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        &format!("pulls/{}/reviews/{}/comments", pull_number, review_id),
        &ListParams::default(),
    )
}

fn list(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    path: &str,
    params: &ListParams,
) -> Result<Page, String> {
    list_raw_path(http_client, grants, plugin_name, owner, repo, path, params)
}

fn list_raw_path(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    path: &str,
    params: &ListParams,
) -> Result<Page, String> {
    if !grants.http_domains.contains("api.github.com") {
        return Err(format!(
            "domain 'api.github.com' not granted for plugin '{}'",
            plugin_name
        ));
    }

    let url = build_url(owner, repo, path, params)?;

    let mut request = http_client
        .get(url)
        .header("User-Agent", "patina-toy-github")
        .header("Accept", "application/vnd.github+json");

    if let Some(token) = token_from_grants(grants) {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request.send().map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let body = response.text().map_err(|e| e.to_string())?;
    page_from_response(status, &headers, body)
}

fn build_url(
    owner: &str,
    repo: &str,
    path: &str,
    params: &ListParams,
) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(&format!(
        "https://api.github.com/repos/{owner}/{repo}/{path}"
    ))
    .map_err(|e| e.to_string())?;
    {
        let mut q = url.query_pairs_mut();
        if let Some(since) = params.since.as_deref() {
            if !since.trim().is_empty() {
                q.append_pair("since", since);
            }
        }
        if let Some(state) = params.state.as_deref() {
            if !state.trim().is_empty() {
                q.append_pair("state", state);
            }
        }
        if let Some(page) = params.page {
            q.append_pair("page", &page.to_string());
        }
        if let Some(per_page) = params.per_page {
            q.append_pair("per_page", &per_page.to_string());
        }
    }
    Ok(url)
}

fn page_from_response(
    status: u16,
    headers: &reqwest::header::HeaderMap,
    body: String,
) -> Result<Page, String> {
    if status >= 400 {
        return Err(format!("github api request failed with status {}", status));
    }

    let rate_remaining = headers
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let (has_next, next_page) = parse_next_page(headers.get("link"));

    let parsed: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("github response was not valid json: {}", e))?;
    if !parsed.is_array() {
        return Err("github list endpoint expected array response".into());
    }

    Ok(Page {
        items: body,
        has_next,
        next_page,
        rate_remaining,
    })
}

fn token_from_grants(grants: &GrantedCapabilities) -> Option<String> {
    if let Some(mapping) = grants.credential_mappings.get("api.github.com") {
        if let Ok(Some(secret)) = crate::mother::get_global_secret(&mapping.secret_name) {
            if !secret.trim().is_empty() {
                return Some(secret);
            }
        }
    }
    std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

fn parse_next_page(link_header: Option<&reqwest::header::HeaderValue>) -> (bool, Option<u32>) {
    let Some(raw) = link_header.and_then(|v| v.to_str().ok()) else {
        return (false, None);
    };

    for part in raw.split(',') {
        let part = part.trim();
        if !part.contains("rel=\"next\"") {
            continue;
        }
        let start = part.find('<');
        let end = part.find('>');
        let Some(start) = start else {
            return (true, None);
        };
        let Some(end) = end else {
            return (true, None);
        };
        if end <= start + 1 {
            return (true, None);
        }
        let url = &part[start + 1..end];
        let next_page = reqwest::Url::parse(url).ok().and_then(|u| {
            u.query_pairs()
                .find(|(k, _)| k == "page")
                .and_then(|(_, v)| v.parse::<u32>().ok())
        });
        return (true, next_page);
    }

    (false, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::child::engine::{GrantedCapabilities, QueryScope};

    fn fixture(path: &str) -> String {
        std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/github-api")
                .join(path),
        )
        .unwrap()
    }

    fn grants() -> GrantedCapabilities {
        let mut g = GrantedCapabilities {
            query_kinds: Default::default(),
            http_domains: Default::default(),
            credential_mappings: Default::default(),
            query_scope: QueryScope::CurrentProject,
            host_emit: false,
            schema_facts: Default::default(),
            state_enabled: false,
            checkpoint_streams: Default::default(),
            lake_names: Default::default(),
            subscribed_streams: Default::default(),
            task_intents: Default::default(),
            graph_read: false,
            graph_write_actions: Default::default(),
            belief_read: false,
            belief_write_actions: Default::default(),
            toys: Default::default(),
        };
        g.http_domains.insert("api.github.com".to_string());
        g
    }

    #[test]
    fn parses_next_page_from_link_header() {
        let header = reqwest::header::HeaderValue::from_static(
            "<https://api.github.com/repos/a/b/issues?page=3>; rel=\"next\", <https://api.github.com/repos/a/b/issues?page=8>; rel=\"last\"",
        );
        let (has_next, next_page) = parse_next_page(Some(&header));
        assert!(has_next);
        assert_eq!(next_page, Some(3));
    }

    #[test]
    fn builds_github_url_with_params() {
        let url = build_url(
            "NicabarNimble",
            "patina",
            "issues",
            &ListParams {
                since: Some("2026-03-20T00:00:00Z".into()),
                state: Some("all".into()),
                page: Some(2),
                per_page: Some(50),
            },
        )
        .unwrap();
        let text = url.to_string();
        assert!(text.contains("since=2026-03-20T00%3A00%3A00Z"));
        assert!(text.contains("state=all"));
        assert!(text.contains("page=2"));
        assert!(text.contains("per_page=50"));
    }

    #[test]
    fn parses_all_fixture_arrays_into_pages() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-ratelimit-remaining",
            reqwest::header::HeaderValue::from_static("4999"),
        );

        for file in [
            "issues.json",
            "pulls.json",
            "issue-comments.json",
            "issue-events.json",
            "pull-comments.json",
            "reviews.json",
            "review-comments.json",
        ] {
            let page = page_from_response(200, &headers, fixture(file)).unwrap();
            let value: serde_json::Value = serde_json::from_str(&page.items).unwrap();
            assert!(value.is_array(), "fixture {} should parse as array", file);
            assert_eq!(page.rate_remaining, 4999);
        }
    }

    #[test]
    fn uses_env_token_when_no_credential_mapping_exists() {
        let old = std::env::var("GITHUB_TOKEN").ok();
        unsafe {
            std::env::set_var("GITHUB_TOKEN", "ghp_fixture_token");
        }
        let token = token_from_grants(&grants());
        match old {
            Some(v) => unsafe { std::env::set_var("GITHUB_TOKEN", v) },
            None => unsafe { std::env::remove_var("GITHUB_TOKEN") },
        }
        assert_eq!(token.as_deref(), Some("ghp_fixture_token"));
    }

    #[test]
    #[ignore]
    fn live_list_issues_when_token_present() {
        if std::env::var("GITHUB_TOKEN").is_err() {
            return;
        }

        let client = reqwest::blocking::Client::new();
        let page = list_issues(
            &client,
            &grants(),
            "test-plugin",
            "rust-lang",
            "rust",
            &ListParams {
                per_page: Some(1),
                ..Default::default()
            },
        )
        .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&page.items).unwrap();
        assert!(parsed.is_array());
    }
}
