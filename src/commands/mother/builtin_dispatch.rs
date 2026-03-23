use mother_crate::http_daemon::{json_error, HttpResponse};
use mother_crate::secrets_authority_api as secrets_api;
use patina::secrets;

pub(super) fn handle_builtin_child_request(
    child_name: &str,
    action: &str,
    body: &[u8],
) -> Option<HttpResponse> {
    match (child_name, action) {
        ("spec-manager", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("spec-manager", "dispatch") => {
            let payload = if body.is_empty() {
                serde_json::Value::Null
            } else {
                match serde_json::from_slice::<serde_json::Value>(body) {
                    Ok(v) => v,
                    Err(e) => return Some(json_error(400, &format!("Invalid JSON: {}", e))),
                }
            };
            let command_value = payload
                .get("command")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let command: crate::commands::spec::SpecCommands =
                match serde_json::from_value(command_value) {
                    Ok(command) => command,
                    Err(e) => {
                        return Some(json_error(
                            400,
                            &format!("Invalid spec-manager command payload: {}", e),
                        ));
                    }
                };

            match crate::commands::spec::execute_value(command) {
                Ok(value) => Some(HttpResponse::json(200, &value)),
                Err(e) => Some(json_error(400, &e.to_string())),
            }
        }
        ("lake-manager", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("lake-manager", "dispatch") => {
            let payload = if body.is_empty() {
                serde_json::Value::Null
            } else {
                match serde_json::from_slice::<serde_json::Value>(body) {
                    Ok(v) => v,
                    Err(e) => return Some(json_error(400, &format!("Invalid JSON: {}", e))),
                }
            };
            let command_value = payload
                .get("command")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let command: crate::commands::lake::LakeCommands =
                match serde_json::from_value(command_value) {
                    Ok(command) => command,
                    Err(e) => {
                        return Some(json_error(
                            400,
                            &format!("Invalid lake-manager command payload: {}", e),
                        ));
                    }
                };

            match crate::commands::lake::execute_value(command) {
                Ok(value) => Some(HttpResponse::json(200, &value)),
                Err(e) => Some(json_error(400, &e.to_string())),
            }
        }
        ("doctor", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("doctor", "run") => match crate::commands::doctor::execute_value() {
            Ok(value) => {
                let exit_code = value.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
                Some(HttpResponse::json(
                    200,
                    &serde_json::json!({
                        "child": "doctor",
                        "text": "",
                        "data": value,
                        "exit_code": exit_code,
                    }),
                ))
            }
            Err(e) => Some(json_error(400, &e.to_string())),
        },
        ("secrets-authority", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("secrets-authority", "dispatch") => {
            let payload = if body.is_empty() {
                serde_json::Value::Null
            } else {
                match serde_json::from_slice::<serde_json::Value>(body) {
                    Ok(v) => v,
                    Err(e) => return Some(json_error(400, &format!("Invalid JSON: {}", e))),
                }
            };
            Some(handle_secrets_authority_dispatch(payload))
        }
        _ => None,
    }
}

fn handle_secrets_authority_dispatch(payload: serde_json::Value) -> HttpResponse {
    secrets_api::dispatch(payload, &PatinaSecretsAuthorityBackend)
}

struct PatinaSecretsAuthorityBackend;

impl secrets_api::SecretsAuthorityBackend for PatinaSecretsAuthorityBackend {
    fn add_secret(
        &self,
        input: secrets_api::AddSecretInput,
    ) -> anyhow::Result<secrets_api::AddSecretResult> {
        let result = secrets::add_secret(
            &input.name,
            &input.value,
            input.env_name.as_deref(),
            input.global,
            input.project_root.as_deref(),
        )?;
        Ok(secrets_api::AddSecretResult {
            env_var: result.env_var,
            created_vault: result.created_vault,
        })
    }

    fn remove_secret(&self, input: secrets_api::RemoveSecretInput) -> anyhow::Result<()> {
        secrets::remove_secret(&input.name, input.global, input.project_root.as_deref())
    }

    fn add_recipient(&self, input: secrets_api::RecipientInput) -> anyhow::Result<()> {
        secrets::add_recipient(&input.project_root, &input.key)
    }

    fn remove_recipient(&self, input: secrets_api::RecipientInput) -> anyhow::Result<()> {
        secrets::remove_recipient(&input.project_root, &input.key)
    }

    fn list_recipients(&self, project_root: std::path::PathBuf) -> anyhow::Result<Vec<String>> {
        secrets::list_recipients(&project_root)
    }

    fn status(
        &self,
        project_root: Option<std::path::PathBuf>,
    ) -> anyhow::Result<secrets_api::StatusResponse> {
        let status = secrets::check_status(project_root.as_deref())?;
        Ok(secrets_api::StatusResponse {
            identity_source: status.identity_source.map(|s| s.to_string()),
            recipient_key: status.recipient_key,
            global: secrets_api::StatusVault {
                exists: status.global.exists,
                secret_count: status.global.secret_count,
                recipient_count: status.global.recipient_count,
                secret_names: status.global.secret_names,
            },
            project: status.project.map(|p| secrets_api::StatusVault {
                exists: p.exists,
                secret_count: p.secret_count,
                recipient_count: p.recipient_count,
                secret_names: p.secret_names,
            }),
        })
    }

    fn lock_session(&self) -> anyhow::Result<()> {
        secrets::lock_session()
    }

    fn export_identity(&self) -> anyhow::Result<String> {
        Ok(secrets::export_identity()?.to_string())
    }

    fn import_identity(&self, identity: String) -> anyhow::Result<String> {
        secrets::import_identity(&identity)
    }

    fn reset_identity(&self) -> anyhow::Result<()> {
        secrets::reset_identity()
    }

    fn setup_claude_token(
        &self,
        token: String,
        project_root: Option<std::path::PathBuf>,
    ) -> anyhow::Result<secrets_api::SetupClaudeTokenResult> {
        let replacing = matches!(secrets::get_global_secret("claude-oauth"), Ok(Some(_)));
        let result = secrets::add_secret(
            "claude-oauth",
            &token,
            Some("CLAUDE_CODE_OAUTH_TOKEN"),
            true,
            project_root.as_deref(),
        )?;
        if let Ok(key) = secrets::export_identity() {
            let _ = secrets::import_identity(&key);
        }
        Ok(secrets_api::SetupClaudeTokenResult {
            replacing,
            created_vault: result.created_vault,
            env_var: result.env_var,
        })
    }
}
