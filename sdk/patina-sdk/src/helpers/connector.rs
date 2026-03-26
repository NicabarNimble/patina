use crate::toys::{
    ConnectorBackend, ConnectorCatalog, ConnectorSyncResult, GrantedConnectorBinding,
};

pub fn upsert_and_sync<B: ConnectorBackend>(
    binding: &GrantedConnectorBinding,
    data_type: &str,
    since: Option<&str>,
) -> Result<ConnectorSyncResult, String> {
    let catalog = ConnectorCatalog::<B>::new();
    let saved = catalog.upsert(binding)?;
    saved.sync(data_type, since)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn calls() -> &'static Mutex<Vec<String>> {
        static CALLS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
        CALLS.get_or_init(|| Mutex::new(Vec::new()))
    }

    struct MockConnector;

    impl ConnectorBackend for MockConnector {
        fn list_bindings() -> Result<Vec<GrantedConnectorBinding>, String> {
            Ok(vec![GrantedConnectorBinding {
                binding_id: "gh-main".to_string(),
                connection: "github".to_string(),
                owner: "nicabar".to_string(),
                repo: "patina".to_string(),
                types: vec!["issue".to_string()],
            }])
        }

        fn upsert_binding(
            binding: &GrantedConnectorBinding,
        ) -> Result<GrantedConnectorBinding, String> {
            calls()
                .lock()
                .unwrap()
                .push(format!("upsert:{}", binding.binding_id));
            Ok(binding.clone())
        }

        fn remove_binding(_binding_id: &str) -> Result<(), String> {
            Ok(())
        }

        fn sync_binding(
            binding_id: &str,
            data_type: &str,
            since: Option<&str>,
        ) -> Result<ConnectorSyncResult, String> {
            calls().lock().unwrap().push(format!(
                "sync:{binding_id}:{data_type}:{}",
                since.unwrap_or("none")
            ));
            Ok(ConnectorSyncResult {
                binding_id: binding_id.to_string(),
                data_type: data_type.to_string(),
                cursor: Some("c-2".to_string()),
                rows_json: vec!["{\"id\":1}".to_string()],
            })
        }
    }

    #[test]
    fn upsert_then_sync_binding() {
        calls().lock().unwrap().clear();
        let result = upsert_and_sync::<MockConnector>(
            &GrantedConnectorBinding {
                binding_id: "gh-main".to_string(),
                connection: "github".to_string(),
                owner: "nicabar".to_string(),
                repo: "patina".to_string(),
                types: vec!["issue".to_string()],
            },
            "issue",
            Some("2026-03-01T00:00:00Z"),
        )
        .unwrap();

        assert_eq!(result.binding_id, "gh-main");
        assert_eq!(result.data_type, "issue");
        let captured = calls().lock().unwrap().clone();
        assert_eq!(
            captured,
            vec!["upsert:gh-main", "sync:gh-main:issue:2026-03-01T00:00:00Z"]
        );
    }
}
