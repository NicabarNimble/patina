//! Secrets execution helpers for Patina CLI.
//!
//! Vault authority lives in Mother. This module is a thin CLI-side layer for
//! command execution and session cache integration.

mod session;

use anyhow::{bail, Result};
use patina_protocol::{
    BuiltinChild, BuiltinChildAction, BuiltinChildRequest, BuiltinChildResult,
    SecretsAuthorityOperation, SecretsDispatchRequest,
};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run_with_secrets(project_root: Option<&Path>, command: &[String]) -> Result<i32> {
    if command.is_empty() {
        bail!("No command provided");
    }

    let secrets = session::get_secrets_with_cache(|| load_secrets_via_ipc(project_root))?;
    if secrets.is_empty() {
        println!("No secrets to inject.");
        let status = Command::new(&command[0]).args(&command[1..]).status()?;
        return Ok(status.code().unwrap_or(1));
    }

    println!("✓ Injecting {} secrets", secrets.len());

    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..]);
    cmd.envs(std::env::vars());
    for (env_var, value) in &secrets {
        cmd.env(env_var, value);
    }
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status()?;
    Ok(status.code().unwrap_or(1))
}

pub fn run_with_secrets_ssh(
    project_root: Option<&Path>,
    host: &str,
    command: &[String],
) -> Result<i32> {
    if command.is_empty() {
        bail!("No command provided");
    }

    let secrets = session::get_secrets_with_cache(|| load_secrets_via_ipc(project_root))?;

    let mut stdin_script = String::new();
    for (env_var, value) in &secrets {
        let escaped_value = value.replace('\'', "'\\''");
        stdin_script.push_str(&format!("export {}='{}'\n", env_var, escaped_value));
    }
    stdin_script.push_str(&format!("exec {}\n", shell_join(command)));

    println!("✓ Injecting {} secrets via SSH (stdin)", secrets.len());

    let mut child = Command::new("ssh")
        .arg(host)
        .arg("bash")
        .arg("-s")
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_script.as_bytes())?;
    }

    let status = child.wait()?;
    Ok(status.code().unwrap_or(1))
}

fn load_secrets_via_ipc(project_root: Option<&Path>) -> Result<HashMap<String, String>> {
    let client = crate::mother::control_plane_client();
    let request = BuiltinChildRequest::new(
        BuiltinChild::SecretsAuthority,
        BuiltinChildAction::SecretsDispatch(SecretsDispatchRequest {
            operation: SecretsAuthorityOperation::LoadSecretsEnvMap {
                project_root: project_root.map(|root| root.display().to_string()),
            },
        }),
    );
    let response = client.child_action_typed(&request).map_err(|error| {
        anyhow::anyhow!(
            "secrets-authority unavailable via Mother (start with `patina mother start`): {}",
            error
        )
    })?;
    let payload = match response.result {
        BuiltinChildResult::Dispatch { payload } => payload,
        other => bail!(
            "Unexpected typed response from secrets-authority: {:?}",
            other
        ),
    };

    let secrets = payload
        .get("secrets")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    Ok(serde_json::from_value(secrets).unwrap_or_default())
}

pub fn prompt_for_value(name: &str) -> Result<String> {
    let term = console::Term::stderr();
    let value = term
        .read_secure_line()
        .map_err(|e| anyhow::anyhow!("Failed to read secret for '{}': {}", name, e))?;
    Ok(value.trim().to_string())
}

fn shell_join(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg.is_empty()
                || arg
                    .contains(|c: char| c.is_whitespace() || "\"'\\$`!#&|;(){}[]<>?*~".contains(c))
            {
                format!("'{}'", arg.replace('\'', "'\\''"))
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    #[test]
    fn shell_join_quotes_spaces() {
        let cmd = vec!["echo".to_string(), "hello world".to_string()];
        assert_eq!(super::shell_join(&cmd), "echo 'hello world'");
    }

    #[test]
    fn run_with_secrets_rejects_empty_command() {
        assert!(super::run_with_secrets(None, &[]).is_err());
    }

    #[test]
    fn secrets_shell_join_safe_token() {
        assert_eq!(super::shell_join(&["abc123".to_string()]), "abc123");
    }

    #[test]
    fn secrets_shell_join_empty_token() {
        assert_eq!(super::shell_join(&["".to_string()]), "''");
    }

    #[test]
    fn secrets_shell_join_escapes_single_quote() {
        assert_eq!(super::shell_join(&["a'b".to_string()]), "'a'\\''b'");
    }

    #[test]
    fn secrets_shell_join_quotes_glob() {
        assert_eq!(super::shell_join(&["*.rs".to_string()]), "'*.rs'");
    }

    #[test]
    fn secrets_shell_join_quotes_dollar() {
        assert_eq!(super::shell_join(&["$HOME".to_string()]), "'$HOME'");
    }

    #[test]
    fn secrets_shell_join_quotes_pipe() {
        assert_eq!(super::shell_join(&["a|b".to_string()]), "'a|b'");
    }

    #[test]
    fn secrets_shell_join_quotes_semicolon() {
        assert_eq!(super::shell_join(&["a;b".to_string()]), "'a;b'");
    }

    #[test]
    fn secrets_shell_join_quotes_bang() {
        assert_eq!(super::shell_join(&["!x".to_string()]), "'!x'");
    }

    #[test]
    fn secrets_shell_join_quotes_tilde() {
        assert_eq!(super::shell_join(&["~".to_string()]), "'~'");
    }

    #[test]
    fn secrets_shell_join_quotes_brackets() {
        assert_eq!(super::shell_join(&["[x]".to_string()]), "'[x]'");
    }

    #[test]
    fn secrets_shell_join_quotes_parens() {
        assert_eq!(super::shell_join(&["(x)".to_string()]), "'(x)'");
    }

    #[test]
    fn secrets_shell_join_quotes_braces() {
        assert_eq!(super::shell_join(&["{x}".to_string()]), "'{x}'");
    }

    #[test]
    fn secrets_shell_join_quotes_angle_brackets() {
        assert_eq!(super::shell_join(&["<x>".to_string()]), "'<x>'");
    }

    #[test]
    fn secrets_shell_join_quotes_backslash() {
        assert_eq!(super::shell_join(&["a\\b".to_string()]), "'a\\b'");
    }

    #[test]
    fn secrets_shell_join_quotes_double_quote() {
        assert_eq!(super::shell_join(&["a\"b".to_string()]), "'a\"b'");
    }

    #[test]
    fn secrets_shell_join_quotes_tab() {
        assert_eq!(super::shell_join(&["a\tb".to_string()]), "'a\tb'");
    }

    #[test]
    fn secrets_shell_join_quotes_newline() {
        assert_eq!(super::shell_join(&["a\nb".to_string()]), "'a\nb'");
    }

    #[test]
    fn secrets_run_with_secrets_ssh_rejects_empty_command() {
        assert!(super::run_with_secrets_ssh(None, "host", &[]).is_err());
    }
}
