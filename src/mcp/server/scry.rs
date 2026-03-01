//! Retrieval MCP handlers — scry, context, mother
//!
//! Also contains formatting helpers, orient/recent/why/use/detail mode handlers,
//! and query logging — all retrieval-domain code.

use anyhow::Result;
use serde::Deserialize;

use super::super::protocol::{Request, Response};
use crate::commands::context::get_project_context;
use crate::commands::scry::internal::enrichment::find_belief_impact;
use crate::commands::scry::ScryResult;
use crate::retrieval::{snippet, FusedResult, QueryEngine, QueryOptions};

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
pub(super) struct ScryArgs {
    pub query: Option<String>,
    pub mode: Option<String>,
    pub query_id: Option<String>,
    pub rank: Option<usize>,
    pub path: Option<String>,
    pub days: Option<u32>,
    pub doc_id: Option<String>,
    pub limit: Option<usize>,
    pub repo: Option<String>,
    #[serde(default)]
    pub all_repos: bool,
    /// In MCP schema but not yet wired to scry handler (ScryOptions has it)
    #[serde(default, rename = "include_issues")]
    pub _include_issues: bool,
    #[serde(default)]
    pub expanded_terms: Vec<String>,
    pub belief: Option<String>,
    pub content_type: Option<String>,
    #[serde(default = "default_true")]
    pub impact: bool,
}

#[derive(Deserialize)]
pub(super) struct ContextArgs {
    pub topic: Option<String>,
    /// In MCP schema but get_project_context doesn't take repo yet
    #[serde(rename = "repo")]
    pub _repo: Option<String>,
    /// In MCP schema but get_project_context doesn't take all_repos yet
    #[serde(default, rename = "all_repos")]
    pub _all_repos: bool,
}

#[derive(Deserialize)]
pub(super) struct MotherArgs {
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub mode: Option<String>,
    pub belief_id: Option<String>,
}

pub(super) fn handle_scry(
    req: &Request,
    args: ScryArgs,
    engine: &QueryEngine,
    conn: &rusqlite::Connection,
) -> Response {
    let mode = args.mode.as_deref().unwrap_or("find");
    let limit = args.limit.unwrap_or(10);

    // Handle modes
    match mode {
        "orient" => {
            let path = args.path.as_deref().unwrap_or("");
            if path.is_empty() {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    "orient mode requires 'path' parameter",
                );
            }

            match crate::commands::scry::internal::orient_json(conn, path, limit) {
                Ok(text) => Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                ),
                Err(e) => Response::error(req.id.clone(), super::ERR_DATABASE, &e.to_string()),
            }
        }
        "recent" => {
            let query = args.query.as_deref();
            let days = args.days.unwrap_or(7);

            match crate::commands::scry::internal::recent_json(conn, query, days, limit) {
                Ok(text) => Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                ),
                Err(e) => Response::error(req.id.clone(), super::ERR_DATABASE, &e.to_string()),
            }
        }
        "why" => {
            let doc_id = args.doc_id.as_deref().unwrap_or("");
            let query = args.query.as_deref().unwrap_or("");

            if doc_id.is_empty() || query.is_empty() {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    "why mode requires 'doc_id' and 'query' parameters",
                );
            }

            match handle_why(doc_id, query, engine) {
                Ok(text) => Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                ),
                Err(e) => Response::error(req.id.clone(), super::ERR_MISSING_INDEX, &e.to_string()),
            }
        }
        "belief" => {
            // E4.6a: Belief grounding — find nearest code/commits/sessions
            let belief_id = args.belief.as_deref().unwrap_or("");

            if belief_id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    "belief mode requires 'belief' parameter",
                );
            }

            let options = crate::commands::scry::ScryOptions {
                limit,
                belief: Some(belief_id.to_string()),
                content_type: args.content_type,
                ..Default::default()
            };

            match crate::commands::scry::scry_belief_fn(belief_id, &options) {
                Ok(results) => {
                    let mut text = format!(
                        "Belief grounding for '{}' ({} results):\n\n",
                        belief_id,
                        results.len()
                    );
                    for (i, r) in results.iter().enumerate() {
                        text.push_str(&format!(
                            "[{}] Score: {:.3} | {} | {}\n    {}\n\n",
                            i + 1,
                            r.score,
                            r.event_type,
                            r.source_id,
                            r.content
                        ));
                    }
                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_MISSING_INDEX, &e.to_string()),
            }
        }
        "use" => {
            let query_id = args.query_id.as_deref().unwrap_or("");
            let rank = args.rank.unwrap_or(0);

            if query_id.is_empty() || rank == 0 {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    "use mode requires 'query_id' and 'rank' parameters",
                );
            }

            match crate::commands::scry::internal::use_json(query_id, rank) {
                Ok(text) => Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                ),
                Err(e) => Response::error(req.id.clone(), super::ERR_DATABASE, &e.to_string()),
            }
        }
        "detail" => {
            let query_id = args.query_id.as_deref().unwrap_or("");
            let rank = args.rank.unwrap_or(0);

            if query_id.is_empty() || rank == 0 {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    "detail mode requires 'query_id' and 'rank' (1-indexed) parameters",
                );
            }

            match crate::commands::scry::detail_json(query_id, rank) {
                Ok(text) => Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                ),
                Err(e) => Response::error(req.id.clone(), super::ERR_DATABASE, &e.to_string()),
            }
        }
        "full" => {
            // D3: Escape hatch — full content for all results (deprecated)
            let query = args.query.as_deref().unwrap_or("");
            let expanded_terms: Vec<&str> =
                args.expanded_terms.iter().map(|s| s.as_str()).collect();

            if query.is_empty() {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    "full mode requires 'query' parameter",
                );
            }

            let full_query = if expanded_terms.is_empty() {
                query.to_string()
            } else {
                format!("{} {}", query, expanded_terms.join(" "))
            };

            let options = QueryOptions {
                repo: args.repo,
                all_repos: args.all_repos,
            };

            match engine.query_with_options(&full_query, limit, &options) {
                Ok(results) => {
                    let query_id = log_mcp_query(query, "full", &results);
                    let mut text = format_results_full_with_query_id(&results, query_id.as_deref());

                    if args.impact {
                        text = annotate_impact(&results, text);
                    }

                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_MISSING_INDEX, &e.to_string()),
            }
        }
        _ => {
            // Default find mode
            let query = args.query.as_deref().unwrap_or("");
            let expanded_terms: Vec<&str> =
                args.expanded_terms.iter().map(|s| s.as_str()).collect();

            if query.is_empty() {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    "find mode requires 'query' parameter",
                );
            }

            // Combine query with expanded terms for better FTS5 matching
            let full_query = if expanded_terms.is_empty() {
                query.to_string()
            } else {
                format!("{} {}", query, expanded_terms.join(" "))
            };

            let options = QueryOptions {
                repo: args.repo,
                all_repos: args.all_repos,
            };

            match engine.query_with_options(&full_query, limit, &options) {
                Ok(results) => {
                    // Log query and get query_id for feedback loop (Phase 3)
                    let query_id = log_mcp_query(query, "find", &results);
                    let mut text = format_results_with_query_id(&results, query_id.as_deref());

                    // E4.6a: Compute belief impact for code results
                    if args.impact {
                        text = annotate_impact(&results, text);
                    }

                    Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }]
                        }),
                    )
                }
                Err(e) => Response::error(req.id.clone(), super::ERR_MISSING_INDEX, &e.to_string()),
            }
        }
    }
}

pub(super) fn handle_context(req: &Request, args: ContextArgs) -> Response {
    let start = std::time::Instant::now();
    let topic = args.topic.as_deref();
    let result = get_project_context(topic);

    // Emit usage event (best-effort)
    emit_usage_event(
        "context.query",
        topic.unwrap_or("(none)"),
        &serde_json::json!({
            "topic": topic,
            "duration_ms": start.elapsed().as_millis() as u64,
            "source": "mcp",
        }),
    );

    match result {
        Ok(text) => Response::success(
            req.id.clone(),
            serde_json::json!({
                "content": [{ "type": "text", "text": text }]
            }),
        ),
        Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
    }
}

pub(super) fn handle_mother(req: &Request, args: MotherArgs) -> Response {
    let mode = args.mode.as_deref().unwrap_or("search");
    let query = args.query.as_deref().unwrap_or("");
    let limit = args.limit.unwrap_or(10);
    let belief_id = args.belief_id.as_deref().unwrap_or("");

    match mode {
        "search" => {
            if query.is_empty() {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    "mother tool requires 'query' parameter",
                );
            }

            match handle_mother_search(query, limit) {
                Ok(text) => Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                ),
                Err(e) => Response::error(req.id.clone(), super::ERR_DATABASE, &e.to_string()),
            }
        }
        "supports" | "attacks" | "projects" => {
            if belief_id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    &format!("mode '{}' requires 'belief_id' parameter", mode),
                );
            }

            match handle_mother_query(mode, belief_id) {
                Ok(text) => Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                ),
                Err(e) => Response::error(req.id.clone(), super::ERR_DATABASE, &e.to_string()),
            }
        }
        _ => Response::error(
            req.id.clone(),
            super::ERR_INVALID_PARAMS,
            &format!("unknown mode '{}'", mode),
        ),
    }
}

// ============================================================================
// Query logging
// ============================================================================

/// Log an MCP query to events.db and return query_id (Phase 3)
fn log_mcp_query(query: &str, mode: &str, results: &[FusedResult]) -> Option<String> {
    // Get session_id from active session
    let session_id = crate::commands::scry::internal::logging::get_active_session_id();

    // Generate query_id
    let now = chrono::Utc::now();
    let random_suffix: String = (0..3)
        .map(|_| (b'a' + fastrand::u8(0..26)) as char)
        .collect();
    let query_id = format!("q_{}_{}", now.format("%Y%m%d_%H%M%S"), random_suffix);

    // Build results array for logging
    let results_json: Vec<serde_json::Value> = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            serde_json::json!({
                "doc_id": r.doc_id,
                "score": r.fused_score,
                "rank": i + 1,
                "event_type": r.metadata.event_type
            })
        })
        .collect();

    let query_data = serde_json::json!({
        "query": query,
        "query_id": query_id,
        "mode": mode,
        "session_id": session_id,
        "results": results_json
    });

    // Best-effort insert into events.db
    let conn = patina::eventlog::open_events_db()
        .map_err(|e| tracing::warn!(error = %e, "failed to open events DB for query logging"))
        .ok()?;
    let timestamp = now.to_rfc3339();
    patina::eventlog::insert_event(
        &conn,
        "scry.query",
        &timestamp,
        &query_id,
        None,
        &query_data.to_string(),
    )
    .map_err(|e| tracing::warn!(error = %e, "failed to log scry query event"))
    .ok()?;

    Some(query_id)
}

// ============================================================================
// Result formatting
// ============================================================================

/// Format snippet results with query_id (default D3 behavior)
fn format_results_with_query_id(results: &[FusedResult], query_id: Option<&str>) -> String {
    let mut output = format_results(results);
    if let Some(qid) = query_id {
        output.push_str(&format!(
            "\n---\nQuery ID: {} (use scry mode='detail' with query_id and rank to fetch full content)\n",
            qid
        ));
    }
    output
}

/// Format full-content results with query_id (mode="full" escape hatch)
fn format_results_full_with_query_id(results: &[FusedResult], query_id: Option<&str>) -> String {
    let mut output = format_results_full(results);
    if let Some(qid) = query_id {
        output.push_str(&format!("\n---\nQuery ID: {} (mode='full')\n", qid));
    }
    output
}

/// Annotate formatted results text with belief impact (E4.6a)
///
/// Converts FusedResults to ScryResults, computes belief impact, and appends
/// a belief impact section to the output.
fn annotate_impact(results: &[FusedResult], mut text: String) -> String {
    // Convert FusedResults to ScryResults for find_belief_impact
    let scry_results: Vec<ScryResult> = results
        .iter()
        .map(|r| {
            let event_type = r.metadata.event_type.clone().unwrap_or_default();
            ScryResult {
                id: 0, // MCP results don't carry usearch keys; resolved via source_id
                content: r.content.clone(),
                score: r.fused_score,
                event_type,
                source_id: r.doc_id.clone(),
                timestamp: r.metadata.timestamp.clone().unwrap_or_default(),
            }
        })
        .collect();

    if let Ok(impact_map) = find_belief_impact(&scry_results) {
        if !impact_map.is_empty() {
            text.push_str("\n--- Belief Impact ---\n");
            for (i, r) in results.iter().enumerate() {
                if let Some(beliefs) = impact_map.get(&r.doc_id) {
                    let belief_strs: Vec<String> = beliefs
                        .iter()
                        .map(|(id, score)| format!("{} ({:.2})", id, score))
                        .collect();
                    text.push_str(&format!(
                        "{}. {} → {}\n",
                        i + 1,
                        r.doc_id,
                        belief_strs.join(", ")
                    ));
                }
            }
        }
    }
    text
}

/// Format result header (shared by snippet and full formatters)
fn format_result_header(i: usize, result: &FusedResult) -> String {
    let mut contributions_str: String = result
        .contributions
        .iter()
        .map(|(name, c)| {
            let score_display = match c.score_type {
                "co_change_count" => format!("co-changes: {}", c.raw_score as i32),
                "bm25" => format!("{:.1} BM25", c.raw_score),
                _ => format!("{:.2}", c.raw_score),
            };
            format!("{} #{} ({})", name, c.rank, score_display)
        })
        .collect::<Vec<_>>()
        .join(" | ");

    // Add structural annotations if available
    let ann = &result.annotations;
    if let Some(count) = ann.importer_count {
        if count > 0 {
            contributions_str.push_str(&format!(" | imp {}", count));
        }
    }
    if let Some(true) = ann.is_entry_point {
        contributions_str.push_str(" | entry");
    }

    let source_tag = if result.sources.contains(&"persona") {
        "[PERSONA] "
    } else {
        ""
    };
    let mut header = format!(
        "{}. {}[{}] (score: {:.3})",
        i + 1,
        source_tag,
        contributions_str,
        result.fused_score
    );

    // Location: file path or doc_id for non-file results
    if let Some(ref path) = result.metadata.file_path {
        header.push_str(&format!(" {}", path));
    } else {
        header.push_str(&format!(" {}", result.doc_id));
    }

    // Event type
    if let Some(ref event_type) = result.metadata.event_type {
        header.push_str(&format!(" ({})", event_type));
    }

    // Timestamp if available
    if let Some(ref ts) = result.metadata.timestamp {
        if !ts.is_empty() {
            header.push_str(&format!(" @{}", ts));
        }
    }

    header
}

/// Format results with type-aware snippets (default D3 behavior)
fn format_results(results: &[FusedResult]) -> String {
    if results.is_empty() {
        return "No results found.".to_string();
    }

    let mut output = String::new();
    for (i, result) in results.iter().enumerate() {
        output.push_str(&format_result_header(i, result));
        output.push('\n');
        output.push_str(&snippet(result));
        output.push_str("\n\n");
    }
    output
}

/// Format results with full content (mode="full" escape hatch)
fn format_results_full(results: &[FusedResult]) -> String {
    if results.is_empty() {
        return "No results found.".to_string();
    }

    let mut output = String::new();
    for (i, result) in results.iter().enumerate() {
        output.push_str(&format_result_header(i, result));
        output.push('\n');
        output.push_str(&result.content);
        output.push_str("\n\n");
    }
    output
}

// ============================================================================
// Mode-specific handlers
// ============================================================================

/// Handle why mode - explain a specific result
fn handle_why(doc_id: &str, query: &str, engine: &QueryEngine) -> Result<String> {
    let options = QueryOptions::default();
    let results = engine.query_with_options(query, 50, &options)?;

    let matching = results
        .iter()
        .find(|r| r.doc_id == doc_id || r.doc_id.ends_with(doc_id) || doc_id.ends_with(&r.doc_id));

    match matching {
        Some(result) => {
            let rank = results
                .iter()
                .position(|r| r.doc_id == result.doc_id)
                .unwrap_or(0)
                + 1;

            let mut output = format!(
                "# Why: {}\n\nQuery: \"{}\"\nRank: #{}\nFused Score: {:.4}\n\n## Oracle Contributions\n\n",
                result.doc_id, query, rank, result.fused_score
            );

            for (oracle_name, contrib) in &result.contributions {
                let score_display = match contrib.score_type {
                    "co_change_count" => format!("{} co-changes", contrib.raw_score as i32),
                    "bm25" => format!("{:.2} BM25", contrib.raw_score),
                    "cosine" => format!("{:.3} cosine", contrib.raw_score),
                    _ => format!("{:.3} {}", contrib.raw_score, contrib.score_type),
                };

                output.push_str(&format!(
                    "- **{}**: rank #{} ({})\n",
                    oracle_name, contrib.rank, score_display
                ));
            }

            let ann = &result.annotations;
            if ann.importer_count.is_some() || ann.activity_level.is_some() {
                output.push_str("\n## Structural Signals\n\n");
                if let Some(count) = ann.importer_count {
                    output.push_str(&format!("- Importers: {}\n", count));
                }
                if let Some(ref level) = ann.activity_level {
                    output.push_str(&format!("- Activity: {}\n", level));
                }
            }

            Ok(output)
        }
        None => {
            let mut output = format!(
                "'{}' not found in top 50 results for query \"{}\".\n\nTop 5 results:\n",
                doc_id, query
            );
            for (i, r) in results.iter().take(5).enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, r.doc_id));
            }
            Ok(output)
        }
    }
}

/// Handle mother query — supports/attacks/projects modes
fn handle_mother_query(mode: &str, belief_id: &str) -> Result<String> {
    use patina::mother::Graph;

    let graph = Graph::open()?;

    match mode {
        "supports" => {
            let supports = graph.query_supports(belief_id)?;
            let json_results: Vec<serde_json::Value> = supports
                .iter()
                .map(|(from, source)| {
                    serde_json::json!({
                        "from_belief": from,
                        "to_belief": belief_id,
                        "source_project": source
                    })
                })
                .collect();
            Ok(serde_json::to_string_pretty(&json_results)?)
        }
        "attacks" => {
            let attacks = graph.query_attacks(belief_id)?;
            let json_results: Vec<serde_json::Value> = attacks
                .iter()
                .map(|(from, source, defeated)| {
                    serde_json::json!({
                        "from_belief": from,
                        "to_belief": belief_id,
                        "source_project": source,
                        "defeated": defeated
                    })
                })
                .collect();
            Ok(serde_json::to_string_pretty(&json_results)?)
        }
        "projects" => {
            let projects = graph.query_projects(belief_id)?;
            let json_results: Vec<serde_json::Value> = projects
                .iter()
                .map(|(source, entrenchment)| {
                    serde_json::json!({
                        "belief_id": belief_id,
                        "source": source,
                        "entrenchment": entrenchment
                    })
                })
                .collect();
            Ok(serde_json::to_string_pretty(&json_results)?)
        }
        _ => anyhow::bail!("unknown mode '{}'", mode),
    }
}

/// Handle mother search — cross-project belief FTS5 search
///
/// Thin wrapper over Graph::search_beliefs(). Returns JSON array per SPEC.
fn handle_mother_search(query: &str, limit: usize) -> Result<String> {
    use patina::mother::Graph;

    let graph = Graph::open()?;
    let results = graph.search_beliefs(query, limit)?;

    if results.is_empty() {
        return Ok("[]".to_string());
    }

    // Return JSON array with all fields including metrics
    let json_results: Vec<serde_json::Value> = results
        .iter()
        .map(|entry| {
            let projects = graph.query_belief_applied_in(&entry.id).unwrap_or_default();
            serde_json::json!({
                "id": entry.id,
                "source": entry.source,
                "kind": entry.kind,
                "statement": entry.statement,
                "entrenchment": entry.entrenchment,
                "status": entry.status,
                "facets": entry.facets,
                "cited_by_beliefs": entry.cited_by_beliefs,
                "cited_by_sessions": entry.cited_by_sessions,
                "applied_in": entry.applied_in,
                "evidence_count": entry.evidence_count,
                "evidence_verified": entry.evidence_verified,
                "health_score": entry.health_score,
                "contested_by": entry.contested_by,
                "grounding_score": entry.grounding_score,
                "grounding_code_count": entry.grounding_code_count,
                "grounding_commit_count": entry.grounding_commit_count,
                "grounding_session_count": entry.grounding_session_count,
                "grounding_forge_count": entry.grounding_forge_count,
                "verification_total": entry.verification_total,
                "verification_passed": entry.verification_passed,
                "verification_failed": entry.verification_failed,
                "verification_errored": entry.verification_errored,
                "last_activity": entry.last_activity,
                "projects": projects
            })
        })
        .collect();

    Ok(serde_json::to_string_pretty(&json_results)?)
}

/// Emit a usage event to events.db (best-effort, warns on failure).
///
/// Shared helper for MCP handlers that need to record context.query or assay.query events.
pub(super) fn emit_usage_event(event_type: &str, source_id: &str, data: &serde_json::Value) {
    if let Err(e) = (|| -> Result<()> {
        let conn = patina::eventlog::open_events_db()?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        patina::eventlog::insert_event(
            &conn,
            event_type,
            &timestamp,
            source_id,
            None,
            &data.to_string(),
        )?;
        Ok(())
    })() {
        tracing::warn!(event_type, error = %e, "failed to record usage event");
    }
}

/// Format full detail content based on event type
fn format_detail_content(event_type: &str, raw_json: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(raw_json) {
        Ok(v) => v,
        Err(_) => return raw_json.to_string(),
    };

    match event_type {
        "code.function" => {
            let name = parsed["name"].as_str().unwrap_or("unknown");
            let file = parsed["file"].as_str().unwrap_or("unknown");
            let is_pub = parsed["is_public"].as_bool().unwrap_or(false);
            let is_async = parsed["is_async"].as_bool().unwrap_or(false);
            let params: Vec<&str> = parsed["parameters"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let return_type = parsed["return_type"].as_str().unwrap_or("");

            let mut sig = String::new();
            if is_pub {
                sig.push_str("pub ");
            }
            if is_async {
                sig.push_str("async ");
            }
            sig.push_str(&format!("fn {}({})", name, params.join(", ")));
            if !return_type.is_empty() {
                sig.push_str(&format!(" -> {}", return_type));
            }

            format!("File: {}\n\n{}", file, sig)
        }
        "belief.surface" => {
            // Belief data has a "content" field with full markdown
            parsed["content"].as_str().unwrap_or(raw_json).to_string()
        }
        "git.commit" => {
            let message = parsed["message"].as_str().unwrap_or("");
            let author = parsed["author_name"].as_str().unwrap_or("");
            let files = parsed["files"].as_array();

            let mut out = format!("Author: {}\nMessage: {}\n", author, message);
            if let Some(files) = files {
                out.push_str(&format!("\nFiles changed ({}):\n", files.len()));
                for f in files.iter().take(20) {
                    let path = f["path"].as_str().unwrap_or("?");
                    let change = f["change_type"].as_str().unwrap_or("?");
                    let added = f["lines_added"].as_u64().unwrap_or(0);
                    let removed = f["lines_removed"].as_u64().unwrap_or(0);
                    out.push_str(&format!(
                        "  {} {} (+{} -{})\n",
                        change, path, added, removed
                    ));
                }
                if files.len() > 20 {
                    out.push_str(&format!("  ... and {} more\n", files.len() - 20));
                }
            }
            out
        }
        t if t.starts_with("pattern.") => {
            // Pattern data has a "content" field with full markdown
            parsed["content"].as_str().unwrap_or(raw_json).to_string()
        }
        _ => {
            // For other types, try "content" field first, then pretty-print JSON
            if let Some(content) = parsed["content"].as_str() {
                content.to_string()
            } else {
                serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| raw_json.to_string())
            }
        }
    }
}
