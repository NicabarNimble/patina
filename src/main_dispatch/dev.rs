use anyhow::Result;

pub(crate) fn dispatch_upgrade(check: bool, json: bool) -> Result<()> {
    crate::commands::dev::upgrade::execute(check, json)?;
    Ok(())
}

pub(crate) fn dispatch_dev(command: crate::DevCommands) -> Result<()> {
    match command {
        crate::DevCommands::Validate { json } => {
            crate::commands::dev::validate::execute(json)?;
        }
        crate::DevCommands::Release { bump, dry_run } => {
            crate::commands::dev::release::execute(
                bump.map(|b| match b {
                    crate::BumpType::Major => "major",
                    crate::BumpType::Minor => "minor",
                    crate::BumpType::Patch => "patch",
                }),
                dry_run,
            )?;
        }
        crate::DevCommands::SyncInterfaces { interface, dry_run } => {
            crate::commands::dev::sync_adapters::execute(interface.as_deref(), dry_run)?;
        }
        crate::DevCommands::BumpVersion {
            component,
            bump_type,
            dry_run,
        } => {
            let bump_str = match bump_type {
                crate::BumpType::Major => "major",
                crate::BumpType::Minor => "minor",
                crate::BumpType::Patch => "patch",
            };
            crate::commands::dev::bump_version::execute(&component, bump_str, dry_run)?;
        }
        crate::DevCommands::UpdateFixtures { fixture } => {
            crate::commands::dev::update_fixtures::execute(fixture.as_deref())?;
        }
    }

    Ok(())
}
