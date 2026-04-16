use crate::toys::{IngressBackend, IngressCatalog};

pub fn fetch_source<B: IngressBackend>(source_name: &str) -> Result<String, String> {
    let source = IngressCatalog::<B>::new().require(source_name)?;
    source.fetch()
}

pub fn fetch_all_sources<B: IngressBackend>() -> Result<Vec<(String, String)>, String> {
    IngressCatalog::<B>::new()
        .list()
        .into_iter()
        .map(|source| {
            let name = source.grant().name.clone();
            let body = source.fetch()?;
            Ok((name, body))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toys::GrantedIngressSource;
    use std::sync::{Mutex, OnceLock};

    fn calls() -> &'static Mutex<Vec<String>> {
        static CALLS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
        CALLS.get_or_init(|| Mutex::new(Vec::new()))
    }

    fn lock_calls() -> std::sync::MutexGuard<'static, Vec<String>> {
        calls().lock().unwrap_or_else(|error| error.into_inner())
    }

    fn test_mutex() -> &'static Mutex<()> {
        static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_MUTEX.get_or_init(|| Mutex::new(()))
    }

    struct MockIngress;

    impl IngressBackend for MockIngress {
        fn list_granted_sources() -> Vec<GrantedIngressSource> {
            vec![
                GrantedIngressSource {
                    name: "github".to_string(),
                    endpoint: "https://api.github.com".to_string(),
                },
                GrantedIngressSource {
                    name: "docs".to_string(),
                    endpoint: "https://docs.example.com".to_string(),
                },
            ]
        }

        fn fetch(source: &str) -> Result<String, String> {
            calls().lock().unwrap().push(format!("fetch:{source}"));
            Ok(format!("payload:{source}"))
        }
    }

    #[test]
    fn fetches_named_source() {
        let _guard = test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        lock_calls().clear();
        let body = fetch_source::<MockIngress>("github").unwrap();
        assert_eq!(body, "payload:github");
        assert_eq!(lock_calls().clone(), vec!["fetch:github"]);
    }

    #[test]
    fn rejects_missing_source_name() {
        let _guard = test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let err = fetch_source::<MockIngress>("missing").unwrap_err();
        assert!(err.contains("not granted"));
    }

    #[test]
    fn fetches_all_granted_sources() {
        let _guard = test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        lock_calls().clear();
        let results = fetch_all_sources::<MockIngress>().unwrap();
        assert_eq!(
            results,
            vec![
                ("github".to_string(), "payload:github".to_string()),
                ("docs".to_string(), "payload:docs".to_string()),
            ]
        );
        assert_eq!(lock_calls().clone(), vec!["fetch:github", "fetch:docs"]);
    }
}
