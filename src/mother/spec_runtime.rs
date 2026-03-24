#[allow(dead_code)]
#[path = "../commands/spec/mod.rs"]
mod shared_spec;

pub use shared_spec::SpecCommands;

pub fn execute_value(command: SpecCommands) -> anyhow::Result<serde_json::Value> {
    shared_spec::execute_value(command)
}
