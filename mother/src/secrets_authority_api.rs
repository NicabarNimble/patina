use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

use crate::http_daemon::{json_error, HttpResponse};

#[derive(Debug, Clone)]
pub struct AddSecretInput {
    pub name: String,
    pub value: String,
    pub env_name: Option<String>,
    pub global: bool,
    pub project_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct RemoveSecretInput {
    pub name: String,
    pub global: bool,
    pub project_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct RecipientInput {
    pub key: String,
    pub project_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AddSecretResult {
    pub env_var: String,
    pub created_vault: bool,
}

#[derive(Debug, Clone)]
pub struct StatusVault {
    pub exists: bool,
    pub secret_count: usize,
    pub recipient_count: usize,
    pub secret_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StatusResponse {
    pub identity_source: Option<String>,
    pub recipient_key: Option<String>,
    pub global: StatusVault,
    pub project: Option<StatusVault>,
}

#[derive(Debug, Clone)]
pub struct SetupClaudeTokenResult {
    pub replacing: bool,
    pub created_vault: bool,
    pub env_var: String,
}

pub trait SecretsAuthorityBackend {
    fn add_secret(&self, input: AddSecretInput) -> Result<AddSecretResult>;
    fn remove_secret(&self, input: RemoveSecretInput) -> Result<()>;
    fn add_recipient(&self, input: RecipientInput) -> Result<()>;
    fn remove_recipient(&self, input: RecipientInput) -> Result<()>;
    fn list_recipients(&self, project_root: PathBuf) -> Result<Vec<String>>;
    fn status(&self, project_root: Option<PathBuf>) -> Result<StatusResponse>;
    fn lock_session(&self) -> Result<()>;
    fn export_identity(&self) -> Result<String>;
    fn import_identity(&self, identity: String) -> Result<String>;
    fn reset_identity(&self) -> Result<()>;
    fn setup_claude_token(
        &self,
        token: String,
        project_root: Option<PathBuf>,
    ) -> Result<SetupClaudeTokenResult>;
}

pub fn dispatch(payload: Value, backend: &dyn SecretsAuthorityBackend) -> HttpResponse {
    let op = payload
        .get("op")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let project_root = payload
        .get("project_root")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    match op {
        "add_secret" => {
            let name = match payload.get("name").and_then(|v| v.as_str()) {
                Some(value) => value.to_string(),
                None => return json_error(400, "Missing 'name' for add_secret"),
            };
            let value = match payload.get("value").and_then(|v| v.as_str()) {
                Some(value) => value.to_string(),
                None => return json_error(400, "Missing 'value' for add_secret"),
            };
            let env_name = payload
                .get("env")
                .and_then(|v| v.as_str())
                .map(ToString::to_string);
            let global = payload
                .get("global")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            match backend.add_secret(AddSecretInput {
                name,
                value,
                env_name,
                global,
                project_root,
            }) {
                Ok(result) => HttpResponse::json(
                    200,
                    &serde_json::json!({
                        "status": "ok",
                        "env_var": result.env_var,
                        "created_vault": result.created_vault,
                    }),
                ),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        "remove_secret" => {
            let name = match payload.get("name").and_then(|v| v.as_str()) {
                Some(value) => value.to_string(),
                None => return json_error(400, "Missing 'name' for remove_secret"),
            };
            let global = payload
                .get("global")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            match backend.remove_secret(RemoveSecretInput {
                name,
                global,
                project_root,
            }) {
                Ok(()) => HttpResponse::json(200, &serde_json::json!({"status": "ok"})),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        "add_recipient" => {
            let key = match payload.get("key").and_then(|v| v.as_str()) {
                Some(value) => value.to_string(),
                None => return json_error(400, "Missing 'key' for add_recipient"),
            };
            let Some(project_root) = project_root else {
                return json_error(400, "Missing 'project_root' for add_recipient");
            };
            match backend.add_recipient(RecipientInput { key, project_root }) {
                Ok(()) => HttpResponse::json(200, &serde_json::json!({"status": "ok"})),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        "remove_recipient" => {
            let key = match payload.get("key").and_then(|v| v.as_str()) {
                Some(value) => value.to_string(),
                None => return json_error(400, "Missing 'key' for remove_recipient"),
            };
            let Some(project_root) = project_root else {
                return json_error(400, "Missing 'project_root' for remove_recipient");
            };
            match backend.remove_recipient(RecipientInput { key, project_root }) {
                Ok(()) => HttpResponse::json(200, &serde_json::json!({"status": "ok"})),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        "list_recipients" => {
            let Some(project_root) = project_root else {
                return json_error(400, "Missing 'project_root' for list_recipients");
            };
            match backend.list_recipients(project_root) {
                Ok(recipients) => HttpResponse::json(
                    200,
                    &serde_json::json!({
                        "status": "ok",
                        "recipients": recipients,
                    }),
                ),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        "status" => match backend.status(project_root) {
            Ok(status) => HttpResponse::json(
                200,
                &serde_json::json!({
                    "status": "ok",
                    "identity_source": status.identity_source,
                    "recipient_key": status.recipient_key,
                    "global": {
                        "exists": status.global.exists,
                        "secret_count": status.global.secret_count,
                        "recipient_count": status.global.recipient_count,
                        "secret_names": status.global.secret_names,
                    },
                    "project": status.project.map(|p| serde_json::json!({
                        "exists": p.exists,
                        "secret_count": p.secret_count,
                        "recipient_count": p.recipient_count,
                        "secret_names": p.secret_names,
                    })),
                }),
            ),
            Err(error) => json_error(400, &error.to_string()),
        },
        "lock_session" => match backend.lock_session() {
            Ok(()) => HttpResponse::json(200, &serde_json::json!({"status": "ok"})),
            Err(error) => json_error(400, &error.to_string()),
        },
        "export_identity" => match backend.export_identity() {
            Ok(identity) => HttpResponse::json(
                200,
                &serde_json::json!({
                    "status": "ok",
                    "identity": identity,
                }),
            ),
            Err(error) => json_error(400, &error.to_string()),
        },
        "import_identity" => {
            let identity = match payload.get("identity").and_then(|v| v.as_str()) {
                Some(value) => value.to_string(),
                None => return json_error(400, "Missing 'identity' for import_identity"),
            };

            match backend.import_identity(identity) {
                Ok(recipient) => HttpResponse::json(
                    200,
                    &serde_json::json!({
                        "status": "ok",
                        "recipient": recipient,
                    }),
                ),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        "reset_identity" => match backend.reset_identity() {
            Ok(()) => HttpResponse::json(200, &serde_json::json!({"status": "ok"})),
            Err(error) => json_error(400, &error.to_string()),
        },
        "setup_claude_token" => {
            let token = match payload.get("token").and_then(|v| v.as_str()) {
                Some(value) => value.to_string(),
                None => return json_error(400, "Missing 'token' for setup_claude_token"),
            };

            match backend.setup_claude_token(token, project_root) {
                Ok(result) => HttpResponse::json(
                    200,
                    &serde_json::json!({
                        "status": "ok",
                        "replacing": result.replacing,
                        "created_vault": result.created_vault,
                        "env_var": result.env_var,
                    }),
                ),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        _ => json_error(400, "Unknown secrets-authority operation"),
    }
}
