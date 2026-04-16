use super::common::*;

#[test]
fn wasm_repos_child_handle_roundtrip() {
    let child = match load_repos_child() {
        Some(c) => c,
        None => return,
    };

    assert_eq!(child.name(), "repos");

    // Report a repo
    let request = ChildRequest {
        action: "report_repo".into(),
        payload: serde_json::json!({
            "name": "test-repo",
            "path": "/tmp/repos/test-repo",
            "last_indexed": 0
        }),
    };
    let response = child.handle(&request).expect("report_repo failed");
    assert_eq!(
        response.payload.get("status").and_then(|v| v.as_str()),
        Some("registered")
    );
    assert_eq!(
        response.payload.get("total_repos").and_then(|v| v.as_u64()),
        Some(1)
    );

    // Check freshness
    let request = ChildRequest {
        action: "check_freshness".into(),
        payload: serde_json::json!({}),
    };
    let response = child.handle(&request).expect("check_freshness failed");
    let stale_count = response.payload.get("stale_count").and_then(|v| v.as_u64());
    assert_eq!(
        stale_count,
        Some(1),
        "repo with last_indexed=0 should be stale"
    );
}

#[test]
fn wasm_repos_child_fresh_repo_no_toys() {
    let mut child = match load_repos_child() {
        Some(c) => c,
        None => return,
    };

    // Report a fresh repo (last_indexed = now)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let request = ChildRequest {
        action: "report_repo".into(),
        payload: serde_json::json!({
            "name": "fresh-repo",
            "path": "/tmp/repos/fresh-repo",
            "last_indexed": now
        }),
    };
    child.handle(&request).expect("report_repo failed");

    // tick() should return no toys — repo is fresh
    let toys = child.tick();
    assert!(
        toys.is_empty(),
        "expected no toys for fresh repo, got: {:?}",
        toys
    );
}

#[test]
fn wasm_repos_child_health_reflects_staleness() {
    let child = match load_repos_child() {
        Some(c) => c,
        None => return,
    };

    // No repos → Healthy
    match child.health() {
        ChildHealth::Healthy => {}
        other => panic!("expected Healthy with no repos, got: {:?}", other),
    }

    // Add stale repo → Degraded
    let request = ChildRequest {
        action: "report_repo".into(),
        payload: serde_json::json!({
            "name": "old-repo",
            "path": "/tmp/repos/old-repo",
            "last_indexed": 0
        }),
    };
    child.handle(&request).expect("report_repo failed");

    match child.health() {
        ChildHealth::Degraded(_) => {} // expected
        other => panic!("expected Degraded with stale repo, got: {:?}", other),
    }
}
