//! Spec lifecycle + schema introspection MCP handlers

use serde::Deserialize;

use super::super::protocol::{Request, Response};

/// Flat args struct covering all spec.* and schemas.* tools.
/// Each subcommand uses a subset of fields; required fields are validated at runtime.
#[derive(Deserialize)]
pub(super) struct SpecArgs {
    // Common
    pub id: Option<String>,
    // spec.list
    pub status: Option<String>,
    pub target: Option<String>,
    // spec.complete / spec.resume
    #[serde(default)]
    pub major: bool,
    #[serde(default)]
    pub force: bool,
    // spec.pause / spec.block / spec.abandon
    pub reason: Option<String>,
    // spec.block
    pub by: Option<String>,
    // spec.split
    pub new_id: Option<String>,
    pub description: Option<String>,
    // spec.set
    pub field: Option<String>,
    pub value: Option<String>,
    // spec.create
    pub spec_type: Option<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub blocked_by: Vec<String>,
    #[serde(default)]
    pub related: Vec<String>,
    // schemas.show
    pub name: Option<String>,
}

/// Require a string field, returning -32602 if missing or empty.
macro_rules! require {
    ($req:expr, $field:expr, $tool:expr, $name:expr) => {
        match $field.as_deref() {
            Some(v) if !v.is_empty() => v,
            _ => {
                return Response::error(
                    $req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    &format!("{} requires '{}' parameter", $tool, $name),
                );
            }
        }
    };
}

pub(super) fn handle(req: &Request, name: &str, args: SpecArgs) -> Response {
    match name {
        // Spec query tools
        "spec.list" => {
            // Parse status at MCP boundary — unknown values return validation error
            let parsed_status = match args.status.as_deref() {
                Some(s) => match s.parse::<patina::spec::SpecStatus>() {
                    Ok(st) => Some(st),
                    Err(e) => {
                        return Response::error(
                            req.id.clone(),
                            super::ERR_INVALID_PARAMS,
                            &e.to_string(),
                        );
                    }
                },
                None => None,
            };
            let filters = crate::commands::spec::ListFilters {
                status: parsed_status,
                target: args.target,
            };
            match crate::commands::spec::get_all_specs(&filters) {
                Ok(specs) => {
                    let text = serde_json::to_string_pretty(&specs).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.ready" => match crate::commands::spec::get_ready_specs() {
            Ok(specs) => {
                let text = serde_json::to_string_pretty(&specs).unwrap_or_default();
                Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                )
            }
            Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
        },
        "spec.blocked" => match crate::commands::spec::get_blocked_specs() {
            Ok(specs) => {
                let text = serde_json::to_string_pretty(&specs).unwrap_or_default();
                Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                )
            }
            Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
        },
        "spec.next" => match crate::commands::spec::next_spec_value() {
            Ok(result) => {
                let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                )
            }
            Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
        },
        "spec.show" => {
            let id = require!(req, args.id, "spec.show", "id");
            match crate::commands::spec::show_spec_value(id) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.prompt" => {
            let id = require!(req, args.id, "spec.prompt", "id");
            match crate::commands::spec::prompt_spec_value(id) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.handoff" => {
            let id = require!(req, args.id, "spec.handoff", "id");
            match crate::commands::spec::handoff_spec_value(id) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.packet" => {
            let id = require!(req, args.id, "spec.packet", "id");
            match crate::commands::spec::packet_spec_value(id) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.check" => {
            let id = require!(req, args.id, "spec.check", "id");
            match crate::commands::spec::check_spec_value(id) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        // Spec mutation tools
        "spec.promote" => {
            let id = require!(req, args.id, "spec.promote", "id");
            match crate::commands::spec::promote_spec_value(id, false) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.complete" => {
            let id = require!(req, args.id, "spec.complete", "id");
            match crate::commands::spec::complete_spec_value(id, args.major, args.force) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.abandon" => {
            let id = require!(req, args.id, "spec.abandon", "id");
            match crate::commands::spec::abandon_spec_value(id, args.reason.as_deref()) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.pause" => {
            let id = require!(req, args.id, "spec.pause", "id");
            let reason = require!(req, args.reason, "spec.pause", "reason");
            match crate::commands::spec::pause_spec_value(id, reason) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.resume" => {
            let id = require!(req, args.id, "spec.resume", "id");
            match crate::commands::spec::resume_spec_value(id, args.force) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.block" => {
            let id = require!(req, args.id, "spec.block", "id");
            let by = require!(req, args.by, "spec.block", "by");
            let reason = require!(req, args.reason, "spec.block", "reason");
            match crate::commands::spec::block_spec_value(id, by, reason) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.split" => {
            let id = require!(req, args.id, "spec.split", "id");
            match crate::commands::spec::split_spec_value(
                id,
                args.new_id.as_deref(),
                args.description.as_deref(),
            ) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.set" => {
            let id = require!(req, args.id, "spec.set", "id");
            let field = require!(req, args.field, "spec.set", "field");
            let value = require!(req, args.value, "spec.set", "value");
            match crate::commands::spec::set_spec_value(id, field, value) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.create" => {
            let spec_type = require!(req, args.spec_type, "spec.create", "spec_type");
            let id = require!(req, args.id, "spec.create", "id");
            match crate::commands::spec::create_spec_value(
                spec_type,
                id,
                args.title.as_deref(),
                args.description.as_deref(),
                args.blocked_by,
                args.related,
            ) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        "spec.history" => {
            let id = require!(req, args.id, "spec.history", "id");
            match crate::commands::spec::history_spec_value(id) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        // Schema introspection tools
        "schemas.list" => match crate::commands::schema::list_value() {
            Ok(schemas) => {
                let text = serde_json::to_string_pretty(&schemas).unwrap_or_default();
                Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                )
            }
            Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
        },
        "schemas.show" => {
            let schema_name = require!(req, args.name, "schemas.show", "name");
            match crate::commands::schema::show_value(schema_name) {
                Ok(schema) => {
                    let text = serde_json::to_string_pretty(&schema).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            }
        }
        _ => Response::error(
            req.id.clone(),
            super::ERR_INVALID_PARAMS,
            &format!("Unknown tool: {}", name),
        ),
    }
}
