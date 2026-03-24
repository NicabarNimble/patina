//! Patina typed protocol contracts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Protocol version envelope for control-plane contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const V0_1: Self = Self { major: 0, minor: 1 };
}

pub const CURRENT_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V0_1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltinChild {
    SpecManager,
    LakeManager,
    Doctor,
    SecretsAuthority,
}

impl BuiltinChild {
    pub fn from_route(name: &str) -> Option<Self> {
        match name {
            "spec-manager" => Some(Self::SpecManager),
            "lake-manager" => Some(Self::LakeManager),
            "doctor" => Some(Self::Doctor),
            "secrets-authority" => Some(Self::SecretsAuthority),
            _ => None,
        }
    }

    pub fn as_route(self) -> &'static str {
        match self {
            Self::SpecManager => "spec-manager",
            Self::LakeManager => "lake-manager",
            Self::Doctor => "doctor",
            Self::SecretsAuthority => "secrets-authority",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpecDispatchRequest {
    pub command: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LakeDispatchRequest {
    pub command: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DoctorRunRequest;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretsDispatchRequest {
    pub operation: SecretsAuthorityOperation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum SecretsAuthorityOperation {
    GetGlobalSecret {
        name: String,
    },
    AddSecret {
        name: String,
        value: String,
        env: Option<String>,
        global: bool,
        project_root: Option<String>,
    },
    RemoveSecret {
        name: String,
        global: bool,
        project_root: Option<String>,
    },
    AddRecipient {
        key: String,
        project_root: String,
    },
    RemoveRecipient {
        key: String,
        project_root: String,
    },
    ListRecipients {
        project_root: String,
    },
    Status {
        project_root: Option<String>,
    },
    LockSession,
    ExportIdentity,
    ImportIdentity {
        identity: String,
    },
    ResetIdentity,
    SetupClaudeToken {
        token: String,
        project_root: Option<String>,
    },
}

impl SecretsAuthorityOperation {
    pub fn from_payload(payload: Value) -> Result<Self, String> {
        let op = payload
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'op' for secrets-authority dispatch".to_string())?;

        let project_root = payload
            .get("project_root")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        let operation = match op {
            "get_global_secret" => Self::GetGlobalSecret {
                name: required_str_field(&payload, "name", op)?,
            },
            "add_secret" => Self::AddSecret {
                name: required_str_field(&payload, "name", op)?,
                value: required_str_field(&payload, "value", op)?,
                env: optional_str_field(&payload, "env"),
                global: payload
                    .get("global")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                project_root,
            },
            "remove_secret" => Self::RemoveSecret {
                name: required_str_field(&payload, "name", op)?,
                global: payload
                    .get("global")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                project_root,
            },
            "add_recipient" => Self::AddRecipient {
                key: required_str_field(&payload, "key", op)?,
                project_root: required_project_root(project_root, op)?,
            },
            "remove_recipient" => Self::RemoveRecipient {
                key: required_str_field(&payload, "key", op)?,
                project_root: required_project_root(project_root, op)?,
            },
            "list_recipients" => Self::ListRecipients {
                project_root: required_project_root(project_root, op)?,
            },
            "status" => Self::Status { project_root },
            "lock_session" => Self::LockSession,
            "export_identity" => Self::ExportIdentity,
            "import_identity" => Self::ImportIdentity {
                identity: required_str_field(&payload, "identity", op)?,
            },
            "reset_identity" => Self::ResetIdentity,
            "setup_claude_token" => Self::SetupClaudeToken {
                token: required_str_field(&payload, "token", op)?,
                project_root,
            },
            _ => return Err(format!("Unknown secrets-authority operation '{}'", op)),
        };

        Ok(operation)
    }

    pub fn into_payload(self) -> Value {
        match self {
            Self::GetGlobalSecret { name } => serde_json::json!({
                "op": "get_global_secret",
                "name": name,
            }),
            Self::AddSecret {
                name,
                value,
                env,
                global,
                project_root,
            } => {
                let mut payload = serde_json::json!({
                    "op": "add_secret",
                    "name": name,
                    "value": value,
                    "global": global,
                });
                if let Some(env) = env {
                    payload["env"] = Value::String(env);
                }
                if let Some(project_root) = project_root {
                    payload["project_root"] = Value::String(project_root);
                }
                payload
            }
            Self::RemoveSecret {
                name,
                global,
                project_root,
            } => {
                let mut payload = serde_json::json!({
                    "op": "remove_secret",
                    "name": name,
                    "global": global,
                });
                if let Some(project_root) = project_root {
                    payload["project_root"] = Value::String(project_root);
                }
                payload
            }
            Self::AddRecipient { key, project_root } => serde_json::json!({
                "op": "add_recipient",
                "key": key,
                "project_root": project_root,
            }),
            Self::RemoveRecipient { key, project_root } => serde_json::json!({
                "op": "remove_recipient",
                "key": key,
                "project_root": project_root,
            }),
            Self::ListRecipients { project_root } => serde_json::json!({
                "op": "list_recipients",
                "project_root": project_root,
            }),
            Self::Status { project_root } => {
                let mut payload = serde_json::json!({ "op": "status" });
                if let Some(project_root) = project_root {
                    payload["project_root"] = Value::String(project_root);
                }
                payload
            }
            Self::LockSession => serde_json::json!({ "op": "lock_session" }),
            Self::ExportIdentity => serde_json::json!({ "op": "export_identity" }),
            Self::ImportIdentity { identity } => serde_json::json!({
                "op": "import_identity",
                "identity": identity,
            }),
            Self::ResetIdentity => serde_json::json!({ "op": "reset_identity" }),
            Self::SetupClaudeToken {
                token,
                project_root,
            } => {
                let mut payload = serde_json::json!({
                    "op": "setup_claude_token",
                    "token": token,
                });
                if let Some(project_root) = project_root {
                    payload["project_root"] = Value::String(project_root);
                }
                payload
            }
        }
    }
}

fn required_str_field(payload: &Value, field: &str, op: &str) -> Result<String, String> {
    payload
        .get(field)
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .ok_or_else(|| format!("Missing '{}' for {}", field, op))
}

fn optional_str_field(payload: &Value, field: &str) -> Option<String> {
    payload
        .get(field)
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

fn required_project_root(project_root: Option<String>, op: &str) -> Result<String, String> {
    project_root.ok_or_else(|| format!("Missing 'project_root' for {}", op))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BuiltinChildAction {
    Health,
    SpecDispatch(SpecDispatchRequest),
    LakeDispatch(LakeDispatchRequest),
    DoctorRun(DoctorRunRequest),
    SecretsDispatch(SecretsDispatchRequest),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuiltinChildRequest {
    pub version: ProtocolVersion,
    pub child: BuiltinChild,
    pub action: BuiltinChildAction,
}

impl BuiltinChildRequest {
    pub fn new(child: BuiltinChild, action: BuiltinChildAction) -> Self {
        Self {
            version: CURRENT_PROTOCOL_VERSION,
            child,
            action,
        }
    }

    pub fn from_http_parts(child_name: &str, action: &str, body: &[u8]) -> Result<Self, String> {
        let Some(child) = BuiltinChild::from_route(child_name) else {
            return Err(format!("Unknown builtin child '{}'", child_name));
        };

        let value = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice::<Value>(body).map_err(|e| format!("Invalid JSON: {}", e))?
        };

        let parsed_action = match (child, action) {
            (_, "health") => BuiltinChildAction::Health,
            (BuiltinChild::SpecManager, "dispatch") => {
                let command = value.get("command").cloned().unwrap_or(Value::Null);
                BuiltinChildAction::SpecDispatch(SpecDispatchRequest { command })
            }
            (BuiltinChild::LakeManager, "dispatch") => {
                let command = value.get("command").cloned().unwrap_or(Value::Null);
                BuiltinChildAction::LakeDispatch(LakeDispatchRequest { command })
            }
            (BuiltinChild::Doctor, "run") => BuiltinChildAction::DoctorRun(DoctorRunRequest),
            (BuiltinChild::SecretsAuthority, "dispatch") => {
                let operation = SecretsAuthorityOperation::from_payload(value)?;
                BuiltinChildAction::SecretsDispatch(SecretsDispatchRequest { operation })
            }
            _ => {
                return Err(format!(
                    "Unsupported action '{}' for child '{}'",
                    action,
                    child.as_route()
                ));
            }
        };

        Ok(Self::new(child, parsed_action))
    }

    pub fn to_http_parts(&self) -> (&'static str, &'static str, Value) {
        let child = self.child.as_route();
        let (action, payload) = match &self.action {
            BuiltinChildAction::Health => ("health", Value::Null),
            BuiltinChildAction::SpecDispatch(request) => (
                "dispatch",
                serde_json::json!({ "command": request.command.clone() }),
            ),
            BuiltinChildAction::LakeDispatch(request) => (
                "dispatch",
                serde_json::json!({ "command": request.command.clone() }),
            ),
            BuiltinChildAction::DoctorRun(_) => ("run", Value::Null),
            BuiltinChildAction::SecretsDispatch(request) => {
                ("dispatch", request.operation.clone().into_payload())
            }
        };

        (child, action, payload)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoctorRunResult {
    pub data: Value,
    pub exit_code: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum BuiltinChildResult {
    Health { status: String },
    Dispatch { payload: Value },
    DoctorRun(DoctorRunResult),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuiltinChildResponse {
    pub version: ProtocolVersion,
    pub child: BuiltinChild,
    pub result: BuiltinChildResult,
}

impl BuiltinChildResponse {
    pub fn new(child: BuiltinChild, result: BuiltinChildResult) -> Self {
        Self {
            version: CURRENT_PROTOCOL_VERSION,
            child,
            result,
        }
    }

    pub fn into_http_payload(self) -> Value {
        match self.result {
            BuiltinChildResult::Health { status } => serde_json::json!({ "status": status }),
            BuiltinChildResult::Dispatch { payload } => payload,
            BuiltinChildResult::DoctorRun(result) => serde_json::json!({
                "child": self.child.as_route(),
                "text": "",
                "data": result.data,
                "exit_code": result.exit_code,
            }),
        }
    }
}
