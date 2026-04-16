use serde::{Deserialize, Serialize};

use super::*;

#[derive(Deserialize)]
struct ScryRequest {
    query: String,
    repo: Option<String>,
    #[serde(default)]
    all_repos: bool,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    10
}

#[derive(Serialize)]
struct ScryResponse {
    results: Vec<ScryResultJson>,
    count: usize,
}

#[derive(Serialize)]
struct ScryResultJson {
    id: i64,
    content: String,
    score: f32,
    event_type: String,
    source_id: String,
    timestamp: String,
}

pub fn handle_scry(request: &HttpRequest, runtime: &(impl ScryApi + ?Sized)) -> HttpResponse {
    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let mut body: ScryRequest = match serde_json::from_slice(&request.body) {
        Ok(req) => req,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    body.limit = body.limit.min(MAX_LIMIT);

    match runtime.scry_query(&body.query, body.limit, body.repo, body.all_repos) {
        Ok(results) => {
            let json_results: Vec<ScryResultJson> = results
                .into_iter()
                .map(|r| ScryResultJson {
                    id: 0,
                    content: r.content,
                    score: r.score,
                    event_type: r.event_type,
                    source_id: r.source_id,
                    timestamp: r.timestamp,
                })
                .collect();

            let response = ScryResponse {
                count: json_results.len(),
                results: json_results,
            };

            HttpResponse::json(200, &response)
        }
        Err(e) => json_error(500, &format!("Scry failed: {}", e)),
    }
}
