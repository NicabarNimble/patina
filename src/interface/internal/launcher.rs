use anyhow::{bail, Result};
use std::env;
use std::path::Path;
use std::process::Command;

pub fn launch_adapter_cli(
    adapter_name: &str,
    project_root: &Path,
    extra_env: &[(String, String)],
) -> Result<()> {
    let claude_token = if adapter_name == "claude" {
        try_get_claude_token()
    } else {
        None
    };

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        let mut cmd = Command::new(adapter_name);
        cmd.current_dir(project_root);
        for (key, value) in extra_env {
            cmd.env(key, value);
        }
        if let Some(token) = &claude_token {
            cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token);
        }
        let err = cmd.exec();
        bail!("Failed to exec {}: {}", adapter_name, err);
    }

    #[cfg(not(unix))]
    {
        use anyhow::Context as _;

        let mut cmd = Command::new(adapter_name);
        cmd.current_dir(project_root);
        for (key, value) in extra_env {
            cmd.env(key, value);
        }
        if let Some(token) = &claude_token {
            cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token);
        }
        let status = cmd
            .status()
            .with_context(|| format!("Failed to run {}", adapter_name))?;
        if !status.success() {
            bail!("{} exited with status: {}", adapter_name, status);
        }
        Ok(())
    }
}

fn try_get_claude_token() -> Option<String> {
    if env::var("ANTHROPIC_API_KEY").is_ok() {
        eprintln!(
            "patina: ANTHROPIC_API_KEY set - skipping vault token injection (API key takes priority)"
        );
        return None;
    }
    if env::var("CLAUDE_CODE_OAUTH_TOKEN").is_ok() {
        eprintln!("patina: CLAUDE_CODE_OAUTH_TOKEN already set - skipping vault token injection");
        return None;
    }
    match crate::secrets::get_global_secret("claude-oauth") {
        Ok(Some(token)) => Some(token),
        Ok(None) => None,
        Err(error) => {
            eprintln!("patina: failed to read claude-oauth from vault - {}", error);
            None
        }
    }
}
