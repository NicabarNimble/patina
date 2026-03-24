pub use crate::commands::spec::SpecCommands;

pub fn execute_value(command: SpecCommands) -> anyhow::Result<serde_json::Value> {
    crate::commands::spec::execute_value(command)
}
