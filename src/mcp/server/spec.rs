//! Spec lifecycle + schema introspection MCP handlers

use super::super::protocol::{Request, Response};

pub(super) fn handle(req: &Request, name: &str, args: &serde_json::Value) -> Response {
    match name {
        // Spec query tools
        "spec.list" => {
            let status = args
                .get("status")
                .and_then(|v| v.as_str())
                .map(String::from);
            let target = args
                .get("target")
                .and_then(|v| v.as_str())
                .map(String::from);
            let filters = crate::commands::spec::ListFilters { status, target };
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
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
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
            Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
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
            Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
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
            Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
        },
        "spec.show" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.show requires 'id' parameter",
                );
            }
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
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        "spec.check" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.check requires 'id' parameter",
                );
            }
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
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        // Spec mutation tools
        "spec.promote" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.promote requires 'id' parameter",
                );
            }
            match crate::commands::spec::promote_spec_value(id) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        "spec.complete" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let major = args.get("major").and_then(|v| v.as_bool()).unwrap_or(false);
            if id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.complete requires 'id' parameter",
                );
            }
            match crate::commands::spec::complete_spec_value(id, major) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        "spec.abandon" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let reason = args.get("reason").and_then(|v| v.as_str());
            if id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.abandon requires 'id' parameter",
                );
            }
            match crate::commands::spec::abandon_spec_value(id, reason) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        "spec.pause" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.pause requires 'id' parameter",
                );
            }
            if reason.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.pause requires 'reason' parameter",
                );
            }
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
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        "spec.resume" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
            if id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.resume requires 'id' parameter",
                );
            }
            match crate::commands::spec::resume_spec_value(id, force) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        "spec.block" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let by = args.get("by").and_then(|v| v.as_str()).unwrap_or("");
            let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.block requires 'id' parameter",
                );
            }
            if by.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.block requires 'by' parameter",
                );
            }
            if reason.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.block requires 'reason' parameter",
                );
            }
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
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        "spec.split" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let new_id = args.get("new_id").and_then(|v| v.as_str());
            let description = args.get("description").and_then(|v| v.as_str());
            if id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.split requires 'id' parameter",
                );
            }
            match crate::commands::spec::split_spec_value(id, new_id, description) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        "spec.set" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let field = args.get("field").and_then(|v| v.as_str()).unwrap_or("");
            let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                return Response::error(req.id.clone(), -32602, "spec.set requires 'id' parameter");
            }
            if field.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.set requires 'field' parameter",
                );
            }
            if value.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.set requires 'value' parameter",
                );
            }
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
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        "spec.create" => {
            let spec_type = args.get("spec_type").and_then(|v| v.as_str()).unwrap_or("");
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let title = args.get("title").and_then(|v| v.as_str());
            let description = args.get("description").and_then(|v| v.as_str());
            let blocked_by: Vec<String> = args
                .get("blocked_by")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let related: Vec<String> = args
                .get("related")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            if spec_type.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.create requires 'spec_type' parameter",
                );
            }
            if id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "spec.create requires 'id' parameter",
                );
            }
            match crate::commands::spec::create_spec_value(
                spec_type,
                id,
                title,
                description,
                blocked_by,
                related,
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
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
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
            Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
        },
        "schemas.show" => {
            let schema_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if schema_name.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "schemas.show requires 'name' parameter",
                );
            }
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
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        _ => Response::error(req.id.clone(), -32602, &format!("Unknown tool: {}", name)),
    }
}
