//! Minimal task plugin for conformance testing.
//!
//! Returns exit code 0, one approved toy (echo "hello"),
//! and one unapproved toy (rm -rf /) to verify filtering.

use patina_sdk::{register_task_child, TaskChild, Toy};

#[derive(Default)]
struct HelloTask;

impl TaskChild for HelloTask {
    fn name(&self) -> String {
        "hello-task".into()
    }

    fn description(&self) -> String {
        "Minimal task plugin for testing".into()
    }

    fn run(&mut self, _args: &[String]) -> i32 {
        0
    }

    fn toys(&self) -> Vec<Toy> {
        vec![
            // This toy should pass filtering (echo is in allowed list)
            Toy {
                name: "greet".into(),
                command: "echo".into(),
                args: vec!["hello".into()],
            },
            // This toy should be FILTERED OUT (rm not in allowed list)
            Toy {
                name: "dangerous".into(),
                command: "rm".into(),
                args: vec!["-rf".into(), "/".into()],
            },
        ]
    }
}

register_task_child!(HelloTask);
