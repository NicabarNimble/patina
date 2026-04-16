use anyhow::Result;

pub(crate) fn dispatch_scrape(command: Option<crate::ScrapeCommands>, rebuild: bool) -> Result<()> {
    if rebuild {
        crate::commands::scrape::execute_rebuild()?;
    } else {
        match command {
            None => crate::commands::scrape::execute_all()?,
            Some(crate::ScrapeCommands::Code { args }) => {
                crate::commands::scrape::execute_code(args.init, args.force)?
            }
            Some(crate::ScrapeCommands::Git { full }) => {
                crate::commands::scrape::execute_git(full)?
            }
            Some(crate::ScrapeCommands::Sessions { full }) => {
                crate::commands::scrape::execute_sessions(full)?
            }
            Some(crate::ScrapeCommands::Layer { full }) => {
                crate::commands::scrape::execute_layer(full)?
            }
        }
    }

    Ok(())
}

pub(crate) fn dispatch_oxidize(repo: Option<String>) -> Result<()> {
    if let Some(repo_name) = repo {
        crate::commands::oxidize::oxidize_for_repo(&repo_name)?;
    } else {
        crate::commands::oxidize::oxidize()?;
    }
    Ok(())
}

pub(crate) fn dispatch_rebuild(
    scrape: bool,
    oxidize: bool,
    force: bool,
    dry_run: bool,
) -> Result<()> {
    let options = crate::commands::rebuild::RebuildOptions {
        scrape_only: scrape,
        oxidize_only: oxidize,
        force,
        dry_run,
    };
    crate::commands::rebuild::execute(options)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_scry(
    command: Option<crate::ScryCommands>,
    query: Option<String>,
    file: Option<String>,
    belief: Option<String>,
    content_type: Option<String>,
    limit: usize,
    min_score: f32,
    repo: Option<String>,
    all_repos: bool,
    include_issues: bool,
    no_persona: bool,
    explain: bool,
    impact: bool,
    detail: Option<String>,
    rank: usize,
) -> Result<()> {
    // Handle subcommands first
    if let Some(subcmd) = command {
        match subcmd {
            crate::ScryCommands::Orient { path, limit } => {
                crate::commands::scry::execute_orient(&path, limit)?;
            }
            crate::ScryCommands::Recent { query, days, limit } => {
                crate::commands::scry::execute_recent(query.as_deref(), days, limit)?;
            }
            crate::ScryCommands::Why { doc_id, query } => {
                crate::commands::scry::execute_why(&doc_id, &query)?;
            }
            crate::ScryCommands::Open { query_id, rank } => {
                crate::commands::scry::execute_open(&query_id, rank)?;
            }
            crate::ScryCommands::Copy { query_id, rank } => {
                crate::commands::scry::execute_copy(&query_id, rank)?;
            }
            crate::ScryCommands::Feedback {
                query_id,
                signal,
                comment,
            } => {
                crate::commands::scry::execute_feedback(&query_id, &signal, comment.as_deref())?;
            }
        }
    } else if let Some(ref query_id) = detail {
        // D3: --detail mode — fetch full content for one result
        crate::commands::scry::execute_detail(query_id, rank)?;
    } else {
        let options = crate::commands::scry::ScryOptions {
            limit,
            min_score,
            dimension: None,
            file,
            repo,
            all_repos,
            include_issues,
            include_persona: !no_persona,
            explain,
            belief,
            content_type,
            impact,
        };
        crate::commands::scry::execute(query.as_deref(), options)?;
    }

    Ok(())
}

pub(crate) fn dispatch_context(topic: Option<String>) -> Result<()> {
    crate::commands::context::execute(topic.as_deref())?;
    Ok(())
}

pub(crate) fn dispatch_eval(
    dimension: Option<crate::Dimension>,
    feedback: bool,
    nl: bool,
    assay: bool,
    scry: bool,
    scry_raw: bool,
    combined: bool,
) -> Result<()> {
    if feedback {
        crate::commands::eval::execute_feedback()?;
    } else if nl {
        crate::commands::eval::execute_nl()?;
    } else if assay {
        crate::commands::eval::execute_assay()?;
    } else if scry_raw {
        crate::commands::eval::execute_scry_raw()?;
    } else if scry {
        crate::commands::eval::execute_scry()?;
    } else if combined {
        crate::commands::eval::execute_combined()?;
    } else {
        crate::commands::eval::execute(dimension.map(|d| d.as_str().to_string()))?;
    }

    Ok(())
}

pub(crate) fn dispatch_bench(command: crate::BenchCommands) -> Result<()> {
    match command {
        crate::BenchCommands::Grammar { files } => {
            crate::commands::bench::execute_grammar(files)?;
        }
        crate::BenchCommands::Retrieval {
            query_set,
            limit,
            json,
            verbose,
            rrf_k,
            fetch_multiplier,
            oracle,
            repo,
        } => {
            let options = crate::commands::bench::BenchOptions {
                query_set,
                limit,
                json,
                verbose,
                rrf_k,
                fetch_multiplier,
                oracle,
                repo,
            };
            crate::commands::bench::execute(options)?;
        }
        crate::BenchCommands::Generate {
            from_commits: _,
            repo,
            limit,
            output,
        } => {
            let options = crate::commands::bench::GenerateOptions {
                repo,
                limit,
                output,
            };
            crate::commands::bench::generate(options)?;
        }
    }

    Ok(())
}

pub(crate) fn dispatch_persona(command: crate::PersonaCommands) -> Result<()> {
    match command {
        crate::PersonaCommands::Note {
            content,
            domains,
            supersedes,
        } => {
            crate::commands::persona::execute_note(&content, domains, supersedes)?;
        }
        crate::PersonaCommands::Query {
            query,
            limit,
            min_score,
            domains,
        } => {
            crate::commands::persona::execute_query(&query, limit, min_score, domains)?;
        }
        crate::PersonaCommands::List { limit, domains } => {
            crate::commands::persona::execute_list(limit, domains)?;
        }
        crate::PersonaCommands::Materialize => {
            crate::commands::persona::execute_materialize()?;
        }
        crate::PersonaCommands::Status => {
            crate::commands::persona::execute_status()?;
        }
    }

    Ok(())
}

pub(crate) fn dispatch_assay(
    command: Option<crate::AssayCommands>,
    pattern: Option<String>,
    limit: usize,
    json: bool,
    repo: Option<String>,
    all_repos: bool,
) -> Result<()> {
    let options = match command {
        None => crate::commands::assay::AssayOptions {
            query_type: crate::commands::assay::QueryType::Inventory,
            pattern,
            limit,
            json,
            repo,
            all_repos,
            ..Default::default()
        },
        Some(crate::AssayCommands::Inventory {
            pattern,
            limit,
            json,
        }) => crate::commands::assay::AssayOptions {
            query_type: crate::commands::assay::QueryType::Inventory,
            pattern,
            limit,
            json,
            repo,
            all_repos,
            ..Default::default()
        },
        Some(crate::AssayCommands::Imports {
            module,
            limit,
            json,
        }) => crate::commands::assay::AssayOptions {
            query_type: crate::commands::assay::QueryType::Imports,
            pattern: Some(module),
            limit,
            json,
            repo,
            all_repos,
            ..Default::default()
        },
        Some(crate::AssayCommands::Importers {
            module,
            limit,
            json,
        }) => crate::commands::assay::AssayOptions {
            query_type: crate::commands::assay::QueryType::Importers,
            pattern: Some(module),
            limit,
            json,
            repo,
            all_repos,
            ..Default::default()
        },
        Some(crate::AssayCommands::Functions {
            pattern,
            limit,
            json,
        }) => crate::commands::assay::AssayOptions {
            query_type: crate::commands::assay::QueryType::Functions,
            pattern,
            limit,
            json,
            repo,
            all_repos,
            ..Default::default()
        },
        Some(crate::AssayCommands::Callers {
            function,
            limit,
            json,
        }) => crate::commands::assay::AssayOptions {
            query_type: crate::commands::assay::QueryType::Callers,
            pattern: Some(function),
            limit,
            json,
            repo,
            all_repos,
            ..Default::default()
        },
        Some(crate::AssayCommands::Callees {
            function,
            limit,
            json,
        }) => crate::commands::assay::AssayOptions {
            query_type: crate::commands::assay::QueryType::Callees,
            pattern: Some(function),
            limit,
            json,
            repo,
            all_repos,
            ..Default::default()
        },
        Some(crate::AssayCommands::Derive { json }) => crate::commands::assay::AssayOptions {
            query_type: crate::commands::assay::QueryType::Derive,
            pattern: None,
            limit: 0,
            json,
            repo,
            all_repos,
            ..Default::default()
        },
        Some(crate::AssayCommands::DeriveMoments { json }) => {
            crate::commands::assay::AssayOptions {
                query_type: crate::commands::assay::QueryType::DeriveMoments,
                pattern: None,
                limit: 0,
                json,
                repo,
                all_repos,
                ..Default::default()
            }
        }
        Some(crate::AssayCommands::Search {
            query: search_query,
            limit: search_limit,
            json: search_json,
            include_issues,
        }) => crate::commands::assay::AssayOptions {
            query_type: crate::commands::assay::QueryType::Search {
                query: search_query,
            },
            pattern: None,
            limit: search_limit,
            json: search_json,
            repo,
            all_repos,
            include_issues,
        },
        Some(crate::AssayCommands::Cochange {
            file,
            limit: cochange_limit,
            json: cochange_json,
        }) => crate::commands::assay::AssayOptions {
            query_type: crate::commands::assay::QueryType::Cochange { file },
            pattern: None,
            limit: cochange_limit,
            json: cochange_json,
            repo,
            all_repos,
            ..Default::default()
        },
        Some(crate::AssayCommands::Belief {
            id,
            limit: belief_limit,
            json: belief_json,
        }) => crate::commands::assay::AssayOptions {
            query_type: crate::commands::assay::QueryType::Belief { id },
            pattern: None,
            limit: belief_limit,
            json: belief_json,
            repo,
            all_repos,
            ..Default::default()
        },
    };
    crate::commands::assay::execute(options)?;

    Ok(())
}
