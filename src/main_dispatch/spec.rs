use anyhow::Result;

pub(crate) fn dispatch_spec(
    project: Option<String>,
    command: crate::commands::spec::SpecCommands,
) -> Result<()> {
    crate::commands::spec::execute(command, project)?;
    Ok(())
}
