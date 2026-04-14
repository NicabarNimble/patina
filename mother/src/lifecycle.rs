use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct ChildHealthInfo {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectDatabasesInfo {
    #[serde(default)]
    pub events_db_bytes: Option<u64>,
    #[serde(default)]
    pub patina_db_bytes: Option<u64>,
    #[serde(default)]
    pub runtime_db_bytes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DegradedChildInfo {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChildWarmupInfo {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryInfo {
    #[serde(default)]
    pub rss_bytes: Option<u64>,
    #[serde(default)]
    pub max_rss_bytes: Option<u64>,
    #[serde(default)]
    pub soft_limit_bytes: Option<u64>,
    #[serde(default)]
    pub pressure: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HealthInfo {
    pub version: String,
    pub uptime_secs: u64,
    #[serde(default)]
    pub children: Vec<ChildHealthInfo>,
    #[serde(default)]
    pub child_count: usize,
    #[serde(default)]
    pub registered_projects: usize,
    #[serde(default)]
    pub active_project_uid: Option<String>,
    #[serde(default)]
    pub active_project_databases: Option<ProjectDatabasesInfo>,
    #[serde(default)]
    pub state_db_bytes: Option<u64>,
    #[serde(default)]
    pub control_plane_ready: bool,
    #[serde(default)]
    pub children_ready_count: usize,
    #[serde(default)]
    pub children_total: usize,
    #[serde(default)]
    pub children_degraded: Vec<DegradedChildInfo>,
    #[serde(default)]
    pub startup_profile: Option<String>,
    #[serde(default)]
    pub rivet_integration: Option<String>,
    #[serde(default)]
    pub child_warmup: Option<ChildWarmupInfo>,
    #[serde(default)]
    pub memory: Option<MemoryInfo>,
}

#[derive(Debug, Clone)]
pub struct StatusReport {
    pub running: bool,
    pub pid: Option<i32>,
    pub stale_pid_file: bool,
    pub health: Option<HealthInfo>,
    pub health_error: Option<String>,
    pub startup_failure: Option<crate::StartupAttemptRecord>,
}

pub fn probe_status(pid_path: &Path, socket_path: &Path) -> Result<StatusReport> {
    let pid = if pid_path.exists() {
        match std::fs::read_to_string(pid_path) {
            Ok(s) => s.trim().parse::<i32>().ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let running = pid
        .map(|p| unsafe { libc::kill(p, 0) == 0 })
        .unwrap_or(false);
    if !running {
        let startup_failure = crate::MotherRuntimeStore::default()
            .last_startup_failure()
            .ok()
            .flatten();
        return Ok(StatusReport {
            running: false,
            pid,
            stale_pid_file: pid.is_some(),
            health: None,
            health_error: None,
            startup_failure,
        });
    }

    match query_health(socket_path) {
        Ok(health) => Ok(StatusReport {
            running: true,
            pid,
            stale_pid_file: false,
            health: Some(health),
            health_error: None,
            startup_failure: None,
        }),
        Err(error) => Ok(StatusReport {
            running: true,
            pid,
            stale_pid_file: false,
            health: None,
            health_error: Some(error.to_string()),
            startup_failure: None,
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopResult {
    NotRunningNoPid,
    NotRunningStalePid,
    Stopped,
    TimedOut,
}

pub fn stop_daemon(pid_path: &Path, socket_path: &Path) -> Result<StopResult> {
    if !pid_path.exists() {
        return Ok(StopResult::NotRunningNoPid);
    }

    let pid_str = std::fs::read_to_string(pid_path)
        .with_context(|| format!("reading PID file {}", pid_path.display()))?;
    let pid: i32 = pid_str
        .trim()
        .parse()
        .with_context(|| format!("parsing PID from '{}'", pid_str.trim()))?;

    let is_running = unsafe { libc::kill(pid, 0) == 0 };
    if !is_running {
        let _ = std::fs::remove_file(pid_path);
        return Ok(StopResult::NotRunningStalePid);
    }

    let result = unsafe { libc::kill(pid, libc::SIGTERM) };
    if result != 0 {
        let err = std::io::Error::last_os_error();
        anyhow::bail!("Failed to send SIGTERM to PID {}: {}", pid, err);
    }

    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let still_running = unsafe { libc::kill(pid, 0) == 0 };
        if !still_running {
            let _ = std::fs::remove_file(pid_path);
            let _ = std::fs::remove_file(socket_path);
            return Ok(StopResult::Stopped);
        }
    }

    Ok(StopResult::TimedOut)
}

fn query_health(socket_path: &Path) -> Result<HealthInfo> {
    let mut stream = UnixStream::connect(socket_path)
        .with_context(|| format!("connecting to {}", socket_path.display()))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;

    let request = "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let response_str = String::from_utf8_lossy(&response);
    let body_start = response_str.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &response_str[body_start..];

    serde_json::from_str(body).with_context(|| "parsing health response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_info_parses_legacy_shape() {
        let json = r#"{"version":"0.1.0","uptime_secs":10,"children":[]}"#;
        let info: HealthInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.version, "0.1.0");
        assert_eq!(info.uptime_secs, 10);
        assert_eq!(info.child_count, 0);
        assert_eq!(info.registered_projects, 0);
        assert!(info.active_project_uid.is_none());
        assert!(info.active_project_databases.is_none());
        assert!(!info.control_plane_ready);
        assert_eq!(info.children_ready_count, 0);
        assert_eq!(info.children_total, 0);
        assert!(info.children_degraded.is_empty());
        assert!(info.startup_profile.is_none());
        assert!(info.rivet_integration.is_none());
        assert!(info.child_warmup.is_none());
        assert!(info.memory.is_none());
    }

    #[test]
    fn health_info_parses_deep_shape() {
        let json = r#"{
            "version":"0.2.0",
            "uptime_secs":20,
            "children":[{"name":"ducklake","status":"healthy"}],
            "child_count":1,
            "registered_projects":3,
            "active_project_uid":"2bdc808e",
            "active_project_databases":{
                "events_db_bytes":100,
                "patina_db_bytes":200,
                "runtime_db_bytes":50
            },
            "state_db_bytes":400,
            "control_plane_ready":true,
            "children_ready_count":1,
            "children_total":2,
            "children_degraded":[{"name":"catalog","reason":"on_load failed"}],
            "startup_profile":"core",
            "rivet_integration":"disabled",
            "child_warmup":{"mode":"manual","state":"pending"},
            "memory":{"max_rss_bytes":1024,"soft_limit_bytes":2048,"pressure":"ok"}
        }"#;
        let info: HealthInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.child_count, 1);
        assert_eq!(info.registered_projects, 3);
        assert_eq!(info.active_project_uid.as_deref(), Some("2bdc808e"));
        assert_eq!(
            info.active_project_databases
                .as_ref()
                .and_then(|d| d.events_db_bytes),
            Some(100)
        );
        assert_eq!(info.state_db_bytes, Some(400));
        assert!(info.control_plane_ready);
        assert_eq!(info.children_ready_count, 1);
        assert_eq!(info.children_total, 2);
        assert_eq!(
            info.children_degraded
                .first()
                .map(|entry| entry.name.as_str()),
            Some("catalog")
        );
        assert_eq!(info.startup_profile.as_deref(), Some("core"));
        assert_eq!(info.rivet_integration.as_deref(), Some("disabled"));
        assert_eq!(
            info.child_warmup
                .as_ref()
                .map(|warmup| warmup.state.as_str()),
            Some("pending")
        );
        assert_eq!(
            info.memory.as_ref().and_then(|memory| memory.max_rss_bytes),
            Some(1024)
        );
    }
}
