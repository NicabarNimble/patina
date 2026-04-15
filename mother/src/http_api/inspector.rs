use serde::Deserialize;

use super::*;

#[derive(Deserialize, Default)]
struct TypedCallHistoryRequest {
    #[serde(default = "default_typed_history_limit")]
    limit: usize,
    #[serde(default)]
    rivet_run_id: Option<String>,
    #[serde(default)]
    rivet_actor_id: Option<String>,
    #[serde(default)]
    rivet_workflow_id: Option<String>,
    #[serde(default)]
    rivet_job_id: Option<String>,
}

impl TypedCallHistoryRequest {
    fn has_correlation_filter(&self) -> bool {
        self.rivet_run_id.is_some()
            || self.rivet_actor_id.is_some()
            || self.rivet_workflow_id.is_some()
            || self.rivet_job_id.is_some()
    }
}

fn default_typed_history_limit() -> usize {
    100
}

pub fn handle_inspector_typed_calls(
    request: &HttpRequest,
    runtime: &(impl InspectorApi + ?Sized),
) -> HttpResponse {
    let body = if request.body.is_empty() {
        TypedCallHistoryRequest::default()
    } else {
        match serde_json::from_slice::<TypedCallHistoryRequest>(&request.body) {
            Ok(value) => value,
            Err(error) => {
                return json_error(400, &format!("Invalid JSON: {}", error));
            }
        }
    };

    let limit = body.limit.clamp(1, MAX_LIMIT);
    let query_limit = if body.has_correlation_filter() {
        MAX_LIMIT
    } else {
        limit
    };

    match runtime.typed_call_history(query_limit) {
        Ok(mut payload) => {
            if body.has_correlation_filter() {
                if let Some(object) = payload.as_object_mut() {
                    let mut filtered_count: Option<usize> = None;
                    if let Some(calls) = object.get_mut("calls").and_then(|v| v.as_array_mut()) {
                        calls.retain(|call| call_matches_correlation_filter(call, &body));
                        if calls.len() > limit {
                            calls.truncate(limit);
                        }
                        filtered_count = Some(calls.len());
                    }
                    if let Some(count) = filtered_count {
                        object.insert("count".to_string(), serde_json::json!(count));
                    }
                }
            }
            HttpResponse::json(200, &payload)
        }
        Err(error) => json_error(500, &format!("typed call history failed: {}", error)),
    }
}

fn call_matches_correlation_filter(
    call: &serde_json::Value,
    filter: &TypedCallHistoryRequest,
) -> bool {
    let correlation = call.get("correlation");

    let matches_string = |expected: &Option<String>, field: &str| {
        expected
            .as_ref()
            .map(|value| {
                correlation
                    .and_then(|c| c.get(field))
                    .and_then(|v| v.as_str())
                    .map(|actual| actual == value)
                    .unwrap_or(false)
            })
            .unwrap_or(true)
    };

    matches_string(&filter.rivet_run_id, "rivet_run_id")
        && matches_string(&filter.rivet_actor_id, "rivet_actor_id")
        && matches_string(&filter.rivet_workflow_id, "rivet_workflow_id")
        && matches_string(&filter.rivet_job_id, "rivet_job_id")
}
