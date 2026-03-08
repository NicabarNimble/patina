//! GitHub connector — first native child on pipe architecture.
//!
//! Fetches GitHub issues and pull requests via REST API, emitting
//! github.issue and github.pr facts over the pipe protocol.
//! All HTTP goes through Mother via pipe/http — this binary never
//! opens sockets directly.

mod github;

use std::io::{BufRead, Write};

use github::GitHubClient;
use patina_pipe::{
    run, Capabilities, Child, FetchParams, FetchResult, InitializeParams, PipeError, PipeIo,
};

struct GitHubConnector;

impl Child for GitHubConnector {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            provider: "github".to_string(),
            data_types: vec!["issues".to_string(), "prs".to_string()],
            supports_incremental: true,
        }
    }

    fn initialize(&mut self, _params: &InitializeParams) -> Result<(), PipeError> {
        eprintln!("[github] connector initialized");
        Ok(())
    }

    fn fetch(
        &mut self,
        params: &FetchParams,
        io: &mut PipeIo<impl Write, impl BufRead>,
    ) -> Result<FetchResult, PipeError> {
        let owner = params.params["owner"].as_str().ok_or(PipeError::Fatal {
            message: "missing 'owner' in params".to_string(),
        })?;
        let repo = params.params["repo"].as_str().ok_or(PipeError::Fatal {
            message: "missing 'repo' in params".to_string(),
        })?;
        let limit = params.limit as usize;
        let since = params.since.as_deref();

        let client = GitHubClient::new(owner, repo);
        let mut total_emitted: u64 = 0;
        let mut latest_cursor: Option<String> = None;

        // Fetch issues if requested
        if params.types.iter().any(|t| t == "issues") {
            eprintln!("[github] syncing issues for {}/{} (limit: {})", owner, repo, limit);
            let result = client.fetch_and_emit_issues(limit, since, io)?;
            total_emitted += result.emitted as u64;
            merge_cursor(&mut latest_cursor, result.latest_updated_at);
            eprintln!(
                "[github] issues: {} fetched, {} emitted",
                result.fetched, result.emitted
            );
        }

        // Fetch PRs if requested
        if params.types.iter().any(|t| t == "prs") {
            eprintln!("[github] syncing PRs for {}/{} (limit: {})", owner, repo, limit);
            let result = client.fetch_and_emit_prs(limit, since, io)?;
            total_emitted += result.emitted as u64;
            merge_cursor(&mut latest_cursor, result.latest_updated_at);
            eprintln!(
                "[github] PRs: {} fetched, {} emitted",
                result.fetched, result.emitted
            );
        }

        Ok(FetchResult {
            emitted: total_emitted,
            cursor: latest_cursor,
        })
    }
}

/// Merge a candidate cursor, keeping the latest timestamp.
fn merge_cursor(current: &mut Option<String>, candidate: Option<String>) {
    if let Some(c) = candidate {
        match current {
            Some(existing) if c > *existing => *current = Some(c),
            None => *current = Some(c),
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(GitHubConnector)
}
