use crate::toys::SessionBackend;
use crate::toys::SessionToy;

pub fn checkpoint<B: SessionBackend>(
    section: &str,
    content: &str,
    status: &str,
) -> Result<(), String> {
    let session = SessionToy::<B>::new();
    session.write(section, content)?;
    session.set_status(status)
}

pub fn handoff<B: SessionBackend>(modified_files: &str, summary: &str) -> Result<(), String> {
    SessionToy::<B>::new().write_handoff(modified_files, summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn calls() -> &'static Mutex<Vec<String>> {
        static CALLS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
        CALLS.get_or_init(|| Mutex::new(Vec::new()))
    }

    struct MockSession;

    impl SessionBackend for MockSession {
        fn get_session_id() -> String {
            "runtime".to_string()
        }
        fn get_previous_session() -> Option<String> {
            None
        }
        fn get_previous_session_runtime_id() -> Option<String> {
            None
        }
        fn get_previous_session_handoff() -> Option<String> {
            None
        }
        fn write(section: &str, content: &str) -> Result<(), String> {
            calls()
                .lock()
                .unwrap()
                .push(format!("write:{section}:{content}"));
            Ok(())
        }
        fn set_parent_session(_runtime_id: &str) -> Result<(), String> {
            Ok(())
        }
        fn create_tag(_name: &str) -> Result<(), String> {
            Ok(())
        }
        fn set_status(status: &str) -> Result<(), String> {
            calls().lock().unwrap().push(format!("status:{status}"));
            Ok(())
        }
        fn write_handoff(modified_files: &str, summary: &str) -> Result<(), String> {
            calls()
                .lock()
                .unwrap()
                .push(format!("handoff:{modified_files}:{summary}"));
            Ok(())
        }
    }

    #[test]
    fn checkpoint_writes_then_sets_status() {
        calls().lock().unwrap().clear();
        checkpoint::<MockSession>("Progress", "done", "active").unwrap();
        let captured = calls().lock().unwrap().clone();
        assert_eq!(captured, vec!["write:Progress:done", "status:active"]);
    }

    #[test]
    fn handoff_writes_payload() {
        calls().lock().unwrap().clear();
        handoff::<MockSession>("a.rs,b.rs", "summary").unwrap();
        let captured = calls().lock().unwrap().clone();
        assert_eq!(captured, vec!["handoff:a.rs,b.rs:summary"]);
    }
}
