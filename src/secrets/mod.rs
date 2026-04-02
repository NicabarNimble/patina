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
    let request = BuiltinChildRequest::new(
        BuiltinChild::SecretsAuthority,
        BuiltinChildAction::SecretsDispatch(SecretsDispatchRequest {
            operation: SecretsAuthorityOperation::LoadSecretsEnvMap {
                project_root: project_root.map(|root| root.display().to_string()),
            },
        }),
    );
    let response = crate::mother::control_plane_client()
        .child_action_typed(&request)
        .map_err(|error| {
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
    let value = console::Term::stderr()
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
    macro_rules! shell_case {
        ($name:ident, $in:expr, $out:expr) => {
            #[test]
            fn $name() {
                assert_eq!(super::shell_join(&[$in.to_string()]), $out);
            }
        };
    }

    #[test]
    fn shell_join_quotes_spaces() {
        assert_eq!(
            super::shell_join(&["echo".into(), "hello world".into()]),
            "echo 'hello world'"
        );
    }
    #[test]
    fn run_with_secrets_rejects_empty_command() {
        assert!(super::run_with_secrets(None, &[]).is_err());
    }
    #[test]
    fn run_with_secrets_ssh_rejects_empty_command() {
        assert!(super::run_with_secrets_ssh(None, "host", &[]).is_err());
    }

    shell_case!(secrets_shell_join_safe_token, "abc123", "abc123");
    shell_case!(secrets_shell_join_empty_token, "", "''");
    shell_case!(secrets_shell_join_escapes_single_quote, "a'b", "'a'\\''b'");
    shell_case!(secrets_shell_join_quotes_glob, "*.rs", "'*.rs'");
    shell_case!(secrets_shell_join_quotes_dollar, "$HOME", "'$HOME'");
    shell_case!(secrets_shell_join_quotes_pipe, "a|b", "'a|b'");
    shell_case!(secrets_shell_join_quotes_semicolon, "a;b", "'a;b'");
    shell_case!(secrets_shell_join_quotes_bang, "!x", "'!x'");
    shell_case!(secrets_shell_join_quotes_tilde, "~", "'~'");
    shell_case!(secrets_shell_join_quotes_brackets, "[x]", "'[x]'");
    shell_case!(secrets_shell_join_quotes_parens, "(x)", "'(x)'");
    shell_case!(secrets_shell_join_quotes_braces, "{x}", "'{x}'");
    shell_case!(secrets_shell_join_quotes_angle_brackets, "<x>", "'<x>'");
    shell_case!(secrets_shell_join_quotes_backslash, "a\\b", "'a\\b'");
    shell_case!(secrets_shell_join_quotes_double_quote, "a\"b", "'a\"b'");
    shell_case!(secrets_shell_join_quotes_tab, "a\tb", "'a\tb'");
    shell_case!(secrets_shell_join_quotes_newline, "a\nb", "'a\nb'");
}
