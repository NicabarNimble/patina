use crate::connect::{ConnectionRecord, InjectionStrategy};
use crate::mother::KnowledgeRuntimeStore;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ConnectionHandle {
    pub name: String,
    #[allow(dead_code)]
    pub base_url: String,
    pub(crate) record: ConnectionRecord,
}

pub fn connect_resolve(name: &str) -> Result<ConnectionHandle, String> {
    let record = crate::connect::load(name).map_err(|e| e.to_string())?;
    let domain = record
        .auth
        .allowed_domains
        .first()
        .cloned()
        .ok_or_else(|| format!("connection '{}' has no allowed domains", name))?;
    Ok(ConnectionHandle {
        name: record.identity.name.clone(),
        base_url: format!("https://{}", domain),
        record,
    })
}

#[allow(dead_code)]
pub fn connect_base_url(conn: &ConnectionHandle) -> String {
    conn.base_url.clone()
}

pub fn connect_matches_domain(conn: &ConnectionHandle, domain: &str) -> bool {
    conn.record
        .auth
        .allowed_domains
        .iter()
        .any(|allowed| normalize_domain(allowed) == normalize_domain(domain))
}

pub fn connect_auth_header_for_domain(
    conn: &ConnectionHandle,
    domain: &str,
) -> Result<Option<(String, String)>, String> {
    let credential = crate::connect::resolve_credential_for_domain(&conn.record, domain)?;
    Ok(match credential.injection {
        InjectionStrategy::Bearer => Some((
            "Authorization".to_string(),
            format!("Bearer {}", credential.value),
        )),
        InjectionStrategy::Header { name } => Some((name, credential.value)),
        InjectionStrategy::InProcess => None,
    })
}

pub fn normalize_domain(domain: &str) -> String {
    domain.trim().trim_end_matches('.').to_ascii_lowercase()
}

#[allow(dead_code)]
pub fn connect_request(
    http_client: &reqwest::blocking::Client,
    conn: &ConnectionHandle,
    method: &str,
    path: &str,
    headers: &[Header],
    body: Option<&[u8]>,
) -> Result<Response, String> {
    let domain = reqwest::Url::parse(&conn.base_url)
        .map_err(|e| format!("invalid base url '{}': {}", conn.base_url, e))?
        .host_str()
        .ok_or_else(|| format!("base url '{}' has no host", conn.base_url))?
        .to_string();
    let credential = crate::connect::resolve_credential_for_domain(&conn.record, &domain)?;

    let url = if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else {
        format!(
            "{}/{}",
            conn.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    };
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| format!("invalid method '{}': {}", method, e))?;
    let mut builder = http_client.request(method, &url);
    for header in headers {
        builder = builder.header(&header.name, &header.value);
    }
    builder = match credential.injection {
        InjectionStrategy::Bearer => {
            builder.header("Authorization", format!("Bearer {}", credential.value))
        }
        InjectionStrategy::Header { ref name } => builder.header(name, credential.value),
        InjectionStrategy::InProcess => {
            return Err(format!(
                "connection '{}' uses inprocess injection which is not valid for connect::request",
                conn.name
            ))
        }
    };
    if let Some(payload) = body {
        builder = builder.body(payload.to_vec());
    }
    let response = builder.send().map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| Header {
            name: name.to_string(),
            value: value.to_str().unwrap_or_default().to_string(),
        })
        .collect::<Vec<_>>();
    let body = response.bytes().map_err(|e| e.to_string())?.to_vec();
    Ok(Response {
        status,
        headers,
        body,
    })
}

pub fn store_query(conn: &ConnectionHandle, query: &str) -> Result<String, String> {
    crate::child::toy_host::lake::query_json(&conn.name, query).map_err(|e| e.to_string())
}

pub fn store_mutate(
    conn: &ConnectionHandle,
    action: &str,
    payload: &str,
) -> Result<String, String> {
    match action {
        "append-json-batch" => {
            #[derive(serde::Deserialize)]
            struct AppendJsonBatch {
                table: String,
                source: String,
                rows_json: Vec<String>,
            }
            let payload: AppendJsonBatch =
                serde_json::from_str(payload).map_err(|e| format!("invalid payload: {}", e))?;
            let inserted = crate::child::toy_host::lake::append_json_batch(
                &conn.name,
                &payload.table,
                &payload.source,
                &payload.rows_json,
            )
            .map_err(|e| e.to_string())?;
            Ok(inserted.to_string())
        }
        other => Err(format!("unsupported store mutate action '{}'", other)),
    }
}

pub fn state_get(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    key: &str,
) -> Result<Option<String>, String> {
    runtime
        .get_state(plugin_name, key)
        .map_err(|e| e.to_string())
}

pub fn state_set(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    key: &str,
    value: &str,
) -> Result<(), String> {
    runtime
        .put_state(plugin_name, key, value)
        .map_err(|e| e.to_string())
}

pub fn state_delete(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    key: &str,
) -> Result<(), String> {
    runtime
        .delete_state(plugin_name, key)
        .map_err(|e| e.to_string())
}

pub fn state_list_prefix(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    prefix: &str,
) -> Result<Vec<String>, String> {
    runtime
        .list_state_prefix(plugin_name, prefix)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRecord {
    pub stream_name: String,
    pub offset: u64,
    pub event_type: String,
    pub payload: String,
    pub occurred_at: String,
}

pub fn events_publish(
    _runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    stream_name: &str,
    event_type: &str,
    payload: &str,
) -> Result<u64, String> {
    let conn = crate::eventlog::open_events_db().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let source_id = format!("stream:{}:{}", plugin_name, stream_name);
    conn.execute(
        "INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data, provenance)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            event_type,
            now,
            source_id,
            Option::<&str>::None,
            payload,
            "external"
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as u64)
}

pub fn events_subscribe(
    stream_name: &str,
    after: Option<u64>,
    limit: u32,
) -> Result<Vec<EventRecord>, String> {
    let conn = crate::eventlog::open_events_db().map_err(|e| e.to_string())?;
    let after = after.unwrap_or(0) as i64;
    let mut stmt = conn
        .prepare(
            "SELECT seq, event_type, timestamp, data, source_id
             FROM eventlog
             WHERE seq > ?1 AND source_id LIKE ?2
             ORDER BY seq
             LIMIT ?3",
        )
        .map_err(|e| e.to_string())?;
    let pattern = format!("%:{}", stream_name);
    let rows = stmt
        .query_map(rusqlite::params![after, pattern, limit as i64], |row| {
            Ok(EventRecord {
                stream_name: stream_name.to_string(),
                offset: row.get::<_, i64>(0)? as u64,
                event_type: row.get(1)?,
                occurred_at: row.get(2)?,
                payload: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn events_ack(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    stream_name: &str,
    offset: u64,
) -> Result<(), String> {
    match stream_name {
        "belief.changed" | "graph.changed" | "fact.ingested" | "session.completed"
        | "repo.synced" => {
            crate::child::toy_host::events::ack_through(runtime, plugin_name, stream_name, offset)
                .map_err(|e| e.to_string())
        }
        _ => Ok(()),
    }
}

pub fn task_enqueue(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    kind: &str,
    payload: &str,
    dedupe_key: Option<String>,
) -> Result<String, String> {
    let kind = crate::mother::TaskIntentKind::parse(kind)
        .ok_or_else(|| format!("unknown task kind '{}'", kind))?;
    let payload = serde_json::from_str(payload).map_err(|e| format!("invalid payload: {}", e))?;
    runtime
        .enqueue_task(
            plugin_name,
            &crate::mother::TaskIntent {
                kind,
                payload,
                dedupe_key,
            },
        )
        .map_err(|e| e.to_string())
}

pub fn peer_call(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    child: &str,
    action: &str,
    payload: &str,
) -> Result<String, String> {
    let envelope = serde_json::json!({
        "child": child,
        "action": action,
        "payload": payload,
    });
    task_enqueue(
        runtime,
        plugin_name,
        "native-job",
        &envelope.to_string(),
        None,
    )
}

pub fn git_create_tag(name: &str) -> Result<(), String> {
    crate::git::create_tag(name, &format!("v2 tag: {}", name)).map_err(|e| e.to_string())
}

pub fn git_delete_tag(name: &str) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .args(["tag", "-d", name])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn git_tag_exists(name: &str) -> Result<bool, String> {
    crate::git::tag_exists(name).map_err(|e| e.to_string())
}

pub fn git_commit(message: &str) -> Result<String, String> {
    crate::git::commit(message).map_err(|e| e.to_string())?;
    crate::git::head_sha().map_err(|e| e.to_string())
}

pub fn git_log_oneline(limit: u32) -> Result<Vec<String>, String> {
    let lines = crate::git::log_oneline(limit as usize).map_err(|e| e.to_string())?;
    Ok(lines
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect())
}

pub fn git_diff_stat() -> Result<String, String> {
    crate::git::diff_stat_summary().map_err(|e| e.to_string())
}

pub fn log_emit(plugin_name: &str, level: &str, message: &str) {
    eprintln!("[{}:{}] {}", plugin_name, level, message);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::with_temp_patina_home;

    #[test]
    fn v2_connect_resolve_exposes_base_url() {
        with_temp_patina_home(|home| {
            let connections_dir = home.join("connections");
            std::fs::create_dir_all(&connections_dir).unwrap();
            std::fs::write(
                connections_dir.join("github.toml"),
                r#"
schema_version = 0

[identity]
name = "github"
provider = "github"
auth_method = "manual"
created_at = "2026-03-26T00:00:00Z"
updated_at = "2026-03-26T00:00:00Z"

[auth]
secret_ref = "github:default"
allowed_domains = ["api.github.com"]
refresh_capable = false

[auth.injection]
type = "bearer"
"#,
            )
            .unwrap();

            let conn = connect_resolve("github").unwrap();
            assert_eq!(conn.name, "github");
            assert_eq!(connect_base_url(&conn), "https://api.github.com");
        });
    }

    #[test]
    fn v2_state_round_trip() {
        with_temp_patina_home(|home| {
            let runtime = KnowledgeRuntimeStore::new_with_project(
                home.join("mother/state.db"),
                crate::mother::ProjectUid::new("2bdc808e").unwrap(),
            );
            state_set(&runtime, "phase4", "k1", r#"{"ok":true}"#).unwrap();
            let loaded = state_get(&runtime, "phase4", "k1").unwrap();
            assert_eq!(loaded.as_deref(), Some(r#"{"ok":true}"#));

            let keys = state_list_prefix(&runtime, "phase4", "k").unwrap();
            assert_eq!(keys, vec!["k1".to_string()]);

            state_delete(&runtime, "phase4", "k1").unwrap();
            assert!(state_get(&runtime, "phase4", "k1").unwrap().is_none());
        });
    }

    #[test]
    fn v2_store_append_and_query_via_connection_handle() {
        with_temp_patina_home(|home| {
            let connections_dir = home.join("connections");
            std::fs::create_dir_all(&connections_dir).unwrap();
            std::fs::write(
                connections_dir.join("default.toml"),
                r#"
schema_version = 0

[identity]
name = "default"
provider = "duckdb"
auth_method = "manual"
created_at = "2026-03-26T00:00:00Z"
updated_at = "2026-03-26T00:00:00Z"

[auth]
secret_ref = "default:none"
allowed_domains = ["localhost"]
refresh_capable = false

[auth.injection]
type = "bearer"
"#,
            )
            .unwrap();

            let conn = connect_resolve("default").unwrap();
            let inserted = store_mutate(
                &conn,
                "append-json-batch",
                r#"{"table":"rows","source":"smoke","rows_json":["{\"id\":1}","{\"id\":2}"]}"#,
            )
            .unwrap();
            assert_eq!(inserted, "2");

            let rows =
                store_query(&conn, "SELECT _source_id FROM rows ORDER BY _source_id").unwrap();
            assert!(rows.contains("smoke"));
        });
    }

    #[test]
    fn v2_events_publish_subscribe_and_ack() {
        with_temp_patina_home(|home| {
            let runtime = KnowledgeRuntimeStore::new_with_project(
                home.join("mother/state.db"),
                crate::mother::ProjectUid::new("2bdc808e").unwrap(),
            );
            let _ = events_publish(
                &runtime,
                "phase4",
                "belief.changed",
                "belief.updated",
                r#"{"id":"b1"}"#,
            )
            .unwrap();
            let events = events_subscribe("belief.changed", None, 10).unwrap();
            assert!(!events.is_empty());
            let last = events.last().unwrap();
            assert_eq!(last.stream_name, "belief.changed");
            events_ack(&runtime, "phase4", "belief.changed", last.offset).unwrap();
        });
    }

    #[test]
    fn v2_task_and_peer_enqueue_native_jobs() {
        with_temp_patina_home(|home| {
            let runtime = KnowledgeRuntimeStore::new_with_project(
                home.join("mother/state.db"),
                crate::mother::ProjectUid::new("2bdc808e").unwrap(),
            );
            let task_id = task_enqueue(
                &runtime,
                "phase4",
                "fetch-source",
                r#"{"source":"github"}"#,
                Some("k1".to_string()),
            )
            .unwrap();
            assert!(!task_id.is_empty());

            let peer_id = peer_call(&runtime, "phase4", "source-router", "sync", "{}").unwrap();
            assert!(!peer_id.is_empty());
        });
    }

    #[test]
    fn v2_git_basic_calls_work() {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .arg("init")
            .status()
            .unwrap()
            .success());
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["config", "user.email", "phase4@test.local"])
            .status()
            .unwrap()
            .success());
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["config", "user.name", "Phase4"])
            .status()
            .unwrap()
            .success());

        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo).unwrap();
        std::fs::write(repo.join("README.md"), "hello\n").unwrap();
        assert!(std::process::Command::new("git")
            .args(["add", "README.md"])
            .status()
            .unwrap()
            .success());
        let _sha = git_commit("init").unwrap();
        assert!(git_tag_exists("phase4-tag").unwrap() == false);
        git_create_tag("phase4-tag").unwrap();
        assert!(git_tag_exists("phase4-tag").unwrap());
        let lines = git_log_oneline(5).unwrap();
        assert!(!lines.is_empty());
        let _ = git_diff_stat().unwrap();
        git_delete_tag("phase4-tag").unwrap();
        assert!(!git_tag_exists("phase4-tag").unwrap());
        std::env::set_current_dir(old_dir).unwrap();
    }
}
