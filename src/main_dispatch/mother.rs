use anyhow::Result;

pub(crate) fn dispatch_mother(
    command: Option<crate::commands::mother::MotherCommands>,
) -> Result<()> {
    crate::commands::mother::execute_cli(command)?;
    Ok(())
}

pub(crate) fn dispatch_serve(host: Option<String>, port: u16, mcp: bool) -> Result<()> {
    // Deprecated: delegate to mother start with warning
    eprintln!("Warning: `patina serve` is deprecated, use `patina mother start` instead.");
    if mcp {
        anyhow::bail!("MCP server path has been retired; use `patina mother start`");
    } else {
        let options = crate::commands::mother::DaemonOptions {
            host,
            port,
            profile: crate::commands::mother::DaemonStartupProfile::Full,
            rivet: crate::commands::mother::RivetIntegrationProfile::Disabled,
        };
        crate::commands::mother::daemon::run_server(options)?;
    }

    Ok(())
}
