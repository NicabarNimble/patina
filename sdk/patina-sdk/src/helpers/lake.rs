use crate::toys::{LakeBackend, LakeCatalog, LakeCursorRecord};

pub fn append_batch_with_cursor<B: LakeBackend>(
    lake_name: &str,
    table: &str,
    source: &str,
    data_type: &str,
    rows_json: &[String],
    cursor: Option<String>,
) -> Result<u64, String> {
    let lake = LakeCatalog::<B>::new().require(lake_name)?;
    lake.ensure_table(table)?;
    let written = lake.append_json_batch(table, source, rows_json)?;
    lake.save_cursor(&LakeCursorRecord {
        source: source.to_string(),
        data_type: data_type.to_string(),
        cursor,
        written,
        status: "ok".to_string(),
        last_error: None,
    })?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toys::GrantedLake;
    use std::sync::{Mutex, OnceLock};

    fn calls() -> &'static Mutex<Vec<String>> {
        static CALLS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
        CALLS.get_or_init(|| Mutex::new(Vec::new()))
    }

    struct MockLake;

    impl LakeBackend for MockLake {
        fn list_granted_lakes() -> Result<Vec<GrantedLake>, String> {
            Ok(vec![GrantedLake {
                name: "default".to_string(),
                path: "/tmp/default".to_string(),
            }])
        }

        fn load_cursor(_lake: &str, _source: &str, _data_type: &str) -> Option<String> {
            None
        }

        fn save_cursor(
            _lake: &str,
            source: &str,
            data_type: &str,
            cursor: Option<&str>,
            written: u64,
            status: &str,
            _last_error: Option<&str>,
        ) -> Result<(), String> {
            calls().lock().unwrap().push(format!(
                "save:{source}:{data_type}:{written}:{status}:{}",
                cursor.unwrap_or("none")
            ));
            Ok(())
        }

        fn ensure_table(lake: &str, table: &str) -> Result<(), String> {
            calls()
                .lock()
                .unwrap()
                .push(format!("ensure:{lake}:{table}"));
            Ok(())
        }

        fn append_json_batch(
            lake: &str,
            table: &str,
            source: &str,
            rows_json: &[String],
        ) -> Result<u64, String> {
            calls().lock().unwrap().push(format!(
                "append:{lake}:{table}:{source}:{}",
                rows_json.len()
            ));
            Ok(rows_json.len() as u64)
        }

        fn query_json(_lake: &str, _sql: &str) -> Result<String, String> {
            Err("unused in test".to_string())
        }
    }

    #[test]
    fn appends_then_saves_cursor() {
        calls().lock().unwrap().clear();
        let rows = vec!["{\"id\":1}".to_string(), "{\"id\":2}".to_string()];
        let written = append_batch_with_cursor::<MockLake>(
            "default",
            "issues",
            "github",
            "issue",
            &rows,
            Some("cursor-2".to_string()),
        )
        .unwrap();

        assert_eq!(written, 2);
        let captured = calls().lock().unwrap().clone();
        assert_eq!(captured[0], "ensure:default:issues");
        assert_eq!(captured[1], "append:default:issues:github:2");
        assert_eq!(captured[2], "save:github:issue:2:ok:cursor-2");
    }
}
