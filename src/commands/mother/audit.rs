use chrono::Utc;

pub(crate) struct CapabilityDecision {
    pub outcome: &'static str,
    pub reason: String,
    pub capability: String,
}

pub(crate) fn evaluate_outside_capability_decisions(
    world: &patina::child::engine::ChildKind,
    capabilities: &[String],
) -> Vec<CapabilityDecision> {
    let allowed = world.allowed_capabilities();
    capabilities
        .iter()
        .map(|capability| {
            if allowed.contains(&capability.as_str()) {
                CapabilityDecision {
                    outcome: "GRANT",
                    capability: capability.clone(),
                    reason: format!("allowed for world '{}'", world),
                }
            } else {
                CapabilityDecision {
                    outcome: "DENY",
                    capability: capability.clone(),
                    reason: format!(
                        "capability '{}' is not allowed for world '{}'",
                        capability, world
                    ),
                }
            }
        })
        .collect()
}

pub(crate) fn emit_outside_capability_audit(
    child: &str,
    capability: &str,
    outcome: &str,
    reason: &str,
) {
    emit_grant_audit_event(
        &format!("mother:grant:outside:{}", child),
        serde_json::json!({
            "scope": "outside",
            "outcome": outcome,
            "child": child,
            "toy_or_capability": capability,
            "reason": reason,
        }),
    );
}

pub(crate) fn emit_typed_wiring_audit(
    from: &str,
    to: &str,
    toy: &str,
    outcome: &str,
    reason: &str,
) {
    emit_grant_audit_event(
        &format!("mother:grant:inside:{}->{}", from, to),
        serde_json::json!({
            "scope": "inside-typed",
            "outcome": outcome,
            "from": from,
            "to": to,
            "toy_or_capability": toy,
            "reason": reason,
        }),
    );
}

fn emit_grant_audit_event(source_id: &str, payload: serde_json::Value) {
    let conn = match patina::eventlog::open_events_db() {
        Ok(conn) => conn,
        Err(error) => {
            tracing::warn!(%error, source_id, "failed to open events db for grant audit");
            return;
        }
    };

    if let Err(error) = patina::eventlog::insert_event(
        &conn,
        "mother.grant",
        &Utc::now().to_rfc3339(),
        source_id,
        None,
        &payload.to_string(),
    ) {
        tracing::warn!(%error, source_id, "failed to write grant audit event");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use serde_json::Value;
    use std::path::Path;

    fn with_temp_patina_home<T>(f: impl FnOnce(&Path) -> T) -> T {
        let _guard = patina::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::TempDir::new().unwrap();
        let home = temp.path().join("patina-home");
        std::fs::create_dir_all(&home).unwrap();
        let old_home = std::env::var_os("PATINA_HOME");
        unsafe {
            std::env::set_var("PATINA_HOME", &home);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&home)));
        match old_home {
            Some(value) => unsafe { std::env::set_var("PATINA_HOME", value) },
            None => unsafe { std::env::remove_var("PATINA_HOME") },
        }
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    fn latest_grant_payload() -> Value {
        let conn: Connection = patina::eventlog::open_events_db().expect("open events db");
        let data: String = conn
            .query_row(
                "SELECT data FROM eventlog WHERE event_type = 'mother.grant' ORDER BY seq DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("query mother.grant event");
        serde_json::from_str(&data).expect("parse mother.grant payload")
    }

    #[test]
    fn outside_capability_decisions_include_grant_and_deny() {
        let decisions = evaluate_outside_capability_decisions(
            &patina::child::engine::ChildKind::Child,
            &["host_log".to_string(), "host_unknown".to_string()],
        );

        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].outcome, "GRANT");
        assert_eq!(decisions[0].capability, "host_log");
        assert_eq!(decisions[1].outcome, "DENY");
        assert_eq!(decisions[1].capability, "host_unknown");
        assert!(decisions[1].reason.contains("not allowed"));
    }

    #[test]
    fn emit_outside_capability_writes_structured_event() {
        with_temp_patina_home(|_| {
            emit_outside_capability_audit(
                "schema-enforcer",
                "host_log",
                "GRANT",
                "allowed for world 'child'",
            );

            let payload = latest_grant_payload();
            assert_eq!(payload["scope"], "outside");
            assert_eq!(payload["outcome"], "GRANT");
            assert_eq!(payload["child"], "schema-enforcer");
            assert_eq!(payload["toy_or_capability"], "host_log");
            assert_eq!(payload["reason"], "allowed for world 'child'");
        });
    }

    #[test]
    fn emit_typed_wiring_writes_structured_event() {
        with_temp_patina_home(|_| {
            emit_typed_wiring_audit(
                "content-extractor",
                "schema-enforcer",
                "patina:records/transform",
                "DENY",
                "no compatible export/import interface found",
            );

            let payload = latest_grant_payload();
            assert_eq!(payload["scope"], "inside-typed");
            assert_eq!(payload["outcome"], "DENY");
            assert_eq!(payload["from"], "content-extractor");
            assert_eq!(payload["to"], "schema-enforcer");
            assert_eq!(payload["toy_or_capability"], "patina:records/transform");
            assert_eq!(
                payload["reason"],
                "no compatible export/import interface found"
            );
        });
    }
}
