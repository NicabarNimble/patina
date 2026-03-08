//! GitHub connector — first native child on pipe architecture.
//!
//! Fetches GitHub issues and pull requests via REST API, emitting
//! github.issue and github.pr facts over the pipe protocol.
//! All HTTP goes through Mother via pipe/http — this binary never
//! opens sockets directly.

use std::io::{BufRead, Write};

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
        _params: &FetchParams,
        _io: &mut PipeIo<impl Write, impl BufRead>,
    ) -> Result<FetchResult, PipeError> {
        // Skeleton — GitHub REST API client lands in commit 2
        Ok(FetchResult {
            emitted: 0,
            cursor: None,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(GitHubConnector)
}
