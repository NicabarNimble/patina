use anyhow::Result;
use clap::{Args, Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(author, version = env!("CARGO_PKG_VERSION"), about = "Context management for AI-assisted development", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new project
    Init {
        /// Project name
        name: String,

        /// LLM to use (claude, gemini, local)
        #[arg(long)]
        llm: String,

        /// Development environment (docker, dagger, native)
        #[arg(long)]
        dev: Option<String>,

        /// Force initialization, backup and replace existing patina branch
        #[arg(long)]
        force: bool,

        /// Local-only mode (skip GitHub integration)
        #[arg(long)]
        local: bool,
    },

    /// Check for new Patina CLI versions
    Upgrade {
        /// Only check for updates, don't show instructions
        #[arg(short, long)]
        check: bool,

        /// Output results as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Developer commands (only available with --features dev)
    #[cfg(feature = "dev")]
    Dev {
        #[command(subcommand)]
        command: DevCommands,
    },

    /// Build project with Docker
    Build,

    /// Run tests in configured environment
    Test,

    /// Check project health and environment
    Doctor {
        /// Output results as JSON
        #[arg(short, long)]
        json: bool,

        /// Check reference repositories in layer/dust/repos
        #[arg(long)]
        repos: bool,

        /// Update stale repositories (requires --repos)
        #[arg(long, requires = "repos")]
        update: bool,

        /// Audit project files and directories for cleanup
        #[arg(long)]
        audit: bool,
    },

    /// Show version information
    Version {
        /// Output as JSON
        #[arg(short, long)]
        json: bool,

        /// Show component versions
        #[arg(short, long)]
        components: bool,
    },

    /// Build semantic knowledge database
    Scrape {
        #[command(subcommand)]
        command: Option<ScrapeCommands>,
    },

    /// Build embeddings and projections from recipe
    Oxidize,

    /// Rebuild .patina/ from layer/ and local sources (portability)
    Rebuild {
        /// Only run scrape step (skip oxidize)
        #[arg(long)]
        scrape: bool,

        /// Only run oxidize step (assume db exists)
        #[arg(long)]
        oxidize: bool,

        /// Delete existing data before rebuild
        #[arg(long)]
        force: bool,

        /// Show what would be rebuilt without doing it
        #[arg(long)]
        dry_run: bool,
    },

    /// Search knowledge base using vector similarity
    Scry {
        /// Query text to search for (optional if --file is provided)
        query: Option<String>,

        /// File path for temporal/dependency queries (e.g., src/auth.rs)
        #[arg(long)]
        file: Option<String>,

        /// Maximum number of results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Minimum similarity score (0.0-1.0, default: 0.0)
        #[arg(long, default_value = "0.0")]
        min_score: f32,

        /// Dimension to search (semantic, temporal, dependency)
        #[arg(long)]
        dimension: Option<String>,

        /// Query a specific external repo (registered via 'patina repo')
        #[arg(long)]
        repo: Option<String>,

        /// Query all registered repos (current project + reference repos)
        #[arg(long)]
        all_repos: bool,

        /// Include GitHub issues in search results
        #[arg(long)]
        include_issues: bool,

        /// Exclude persona knowledge from results
        #[arg(long)]
        no_persona: bool,
    },

    /// Evaluate retrieval quality across dimensions
    Eval {
        /// Specific dimension to evaluate (semantic, temporal)
        #[arg(long)]
        dimension: Option<String>,
    },

    /// Generate and manage semantic embeddings
    Embeddings {
        #[command(subcommand)]
        command: EmbeddingsCommands,
    },

    /// Query observations and beliefs using semantic search
    Query {
        #[command(subcommand)]
        command: QueryCommands,
    },

    /// Validate beliefs using neuro-symbolic reasoning
    Belief {
        #[command(subcommand)]
        command: BeliefCommands,
    },

    /// Cross-project user knowledge (preferences, style, history)
    Persona {
        #[command(subcommand)]
        command: PersonaCommands,
    },

    /// Ask questions about the codebase
    Ask {
        #[command(flatten)]
        args: commands::ask::AskCommand,
    },

    /// Manage external repositories for cross-project knowledge
    Repo {
        #[command(subcommand)]
        command: Option<RepoCommands>,

        /// Repository URL (shorthand for 'patina repo add <url>')
        #[arg(conflicts_with = "command")]
        url: Option<String>,

        /// Enable contribution mode (create fork for PRs)
        #[arg(long, requires = "url")]
        contrib: bool,

        /// Also fetch and index GitHub issues
        #[arg(long, requires = "url")]
        with_issues: bool,
    },

    /// Generate YOLO devcontainer for autonomous AI development
    Yolo {
        /// Use interactive mode to choose options
        #[arg(short, long)]
        interactive: bool,

        /// Use all defaults without prompting
        #[arg(short, long, conflicts_with = "interactive")]
        defaults: bool,

        /// Additional tools to include (e.g., --with cairo,solidity)
        #[arg(long, value_delimiter = ',')]
        with: Option<Vec<String>>,

        /// Tools to exclude from detection (e.g., --without python)
        #[arg(long, value_delimiter = ',')]
        without: Option<Vec<String>>,

        /// Output results as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Start the Mothership daemon (Ollama-style HTTP server)
    Serve {
        /// Host to bind to (default: 127.0.0.1, use 0.0.0.0 for container access)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to bind to
        #[arg(long, default_value = "50051")]
        port: u16,
    },

    /// Open project in AI frontend (like 'code .' for VS Code)
    Launch {
        /// Project path (default: current directory)
        path: Option<String>,

        /// Frontend to use (claude, gemini, codex)
        #[arg(short, long)]
        frontend: Option<String>,

        /// Don't auto-start mothership
        #[arg(long)]
        no_serve: bool,
    },

    /// List available AI frontends
    Adapter {
        #[command(subcommand)]
        command: Option<AdapterCommands>,
    },
}

/// Common arguments for all scrape subcommands
#[derive(Args)]
struct ScrapeArgs {
    /// Initialize the knowledge database
    #[arg(long)]
    init: bool,

    /// Force full re-index (ignore incremental updates)
    #[arg(long)]
    force: bool,
}

#[derive(Subcommand)]
enum ScrapeCommands {
    /// Extract semantic information using modular architecture
    Code {
        #[command(flatten)]
        args: ScrapeArgs,
    },
    /// Extract git commit history and co-change relationships
    Git {
        /// Full rebuild (ignore incremental)
        #[arg(long)]
        full: bool,
    },
    /// Extract sessions, goals, and observations from session files
    Sessions {
        /// Full rebuild (ignore incremental)
        #[arg(long)]
        full: bool,
    },
}

#[derive(Subcommand)]
enum EmbeddingsCommands {
    /// Generate embeddings for all beliefs and observations
    Generate {
        /// Force regeneration of all embeddings
        #[arg(long)]
        force: bool,
    },

    /// Show embedding coverage status
    Status,
}

#[derive(Subcommand)]
enum QueryCommands {
    /// Search observations using semantic similarity
    Semantic {
        /// Query text to search for
        query: String,

        /// Filter by observation types (comma-separated: pattern,technology,decision,challenge)
        #[arg(long, value_delimiter = ',')]
        r#type: Option<Vec<String>>,

        /// Minimum similarity score (0.0-1.0, default: 0.35)
        #[arg(long, default_value = "0.35")]
        min_score: f32,

        /// Maximum number of results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum BeliefCommands {
    /// Validate a belief using semantic evidence and symbolic reasoning
    Validate {
        /// Belief statement to validate
        query: String,

        /// Minimum similarity score for evidence (0.0-1.0, default: 0.50)
        #[arg(long, default_value = "0.50")]
        min_score: f32,

        /// Maximum number of observations to consider (default: 20)
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum PersonaCommands {
    /// Capture knowledge directly
    Note {
        /// Content to capture
        content: String,

        /// Domains this applies to (comma-separated, e.g., rust,error-handling)
        #[arg(long, value_delimiter = ',')]
        domains: Option<Vec<String>>,

        /// Event ID this supersedes (replaces old knowledge)
        #[arg(long)]
        supersedes: Option<String>,
    },

    /// Search persona knowledge
    Query {
        /// Search query
        query: String,

        /// Maximum results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Minimum similarity score (0.0-1.0, default: 0.0)
        #[arg(long, default_value = "0.0")]
        min_score: f32,

        /// Filter by domains (comma-separated)
        #[arg(long, value_delimiter = ',')]
        domains: Option<Vec<String>>,
    },

    /// List captured knowledge
    List {
        /// Maximum entries to show (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Filter by domains (comma-separated)
        #[arg(long, value_delimiter = ',')]
        domains: Option<Vec<String>>,
    },

    /// Process events into searchable index
    Materialize,
}

#[derive(Subcommand)]
enum RepoCommands {
    /// Add an external repository
    Add {
        /// GitHub URL (e.g., https://github.com/owner/repo or owner/repo)
        url: String,

        /// Enable contribution mode (create fork for PRs)
        #[arg(long)]
        contrib: bool,

        /// Also fetch and index GitHub issues
        #[arg(long)]
        with_issues: bool,
    },

    /// List registered repositories
    List,

    /// Update a repository (git pull + rescrape)
    Update {
        /// Repository name (or --all for all repos)
        name: Option<String>,

        /// Update all repositories
        #[arg(long)]
        all: bool,

        /// Also run oxidize to build semantic indices
        #[arg(long)]
        oxidize: bool,
    },

    /// Remove a repository
    #[command(alias = "rm")]
    Remove {
        /// Repository name
        name: String,
    },

    /// Show details about a repository
    Show {
        /// Repository name
        name: String,
    },
}

#[derive(Subcommand)]
enum AdapterCommands {
    /// List available frontends (global) and allowed frontends (project)
    List,

    /// Set default frontend (global or project with --project)
    Default {
        /// Frontend name (claude, gemini, codex)
        name: String,

        /// Set default for current project (not global)
        #[arg(short, long)]
        project: bool,
    },

    /// Check frontend installation status
    Check {
        /// Frontend name (optional, checks all if not specified)
        name: Option<String>,
    },

    /// Add a frontend to project's allowed list
    Add {
        /// Frontend name (claude, gemini, codex)
        name: String,
    },

    /// Remove a frontend from project's allowed list
    Remove {
        /// Frontend name (claude, gemini, codex)
        name: String,

        /// Don't backup files before removing
        #[arg(long)]
        no_backup: bool,
    },
}

#[cfg(feature = "dev")]
#[derive(Subcommand)]
enum DevCommands {
    /// Validate resources and patterns
    Validate {
        /// Output results as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Prepare for a new release
    Release {
        /// Version bump type
        #[arg(value_enum)]
        bump: Option<BumpType>,

        /// Dry run - don't make changes
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Sync adapter templates from resources
    SyncAdapters {
        /// Specific adapter to sync (claude, gemini, etc)
        adapter: Option<String>,

        /// Dry run - show what would change
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Bump component versions
    BumpVersion {
        /// Component to bump (patina, claude-adapter, etc)
        component: String,

        /// Version bump type
        #[arg(value_enum)]
        bump_type: BumpType,

        /// Dry run - don't make changes
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Update test fixtures
    UpdateFixtures {
        /// Specific fixture to update
        fixture: Option<String>,
    },
}

#[cfg(feature = "dev")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
enum BumpType {
    Major,
    Minor,
    Patch,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            name,
            llm,
            dev,
            force,
            local,
        } => {
            commands::init::execute(name, llm, dev, force, local)?;
        }
        Commands::Upgrade { check, json } => {
            commands::upgrade::execute(check, json)?;
        }
        #[cfg(feature = "dev")]
        Commands::Dev { command } => match command {
            DevCommands::Validate { json } => {
                commands::dev::validate::execute(json)?;
            }
            DevCommands::Release { bump, dry_run } => {
                commands::dev::release::execute(
                    bump.map(|b| match b {
                        BumpType::Major => "major",
                        BumpType::Minor => "minor",
                        BumpType::Patch => "patch",
                    }),
                    dry_run,
                )?;
            }
            DevCommands::SyncAdapters { adapter, dry_run } => {
                commands::dev::sync_adapters::execute(adapter.as_deref(), dry_run)?;
            }
            DevCommands::BumpVersion {
                component,
                bump_type,
                dry_run,
            } => {
                let bump_str = match bump_type {
                    BumpType::Major => "major",
                    BumpType::Minor => "minor",
                    BumpType::Patch => "patch",
                };
                commands::dev::bump_version::execute(&component, bump_str, dry_run)?;
            }
            DevCommands::UpdateFixtures { fixture } => {
                commands::dev::update_fixtures::execute(fixture.as_deref())?;
            }
        },
        Commands::Build => {
            commands::build::execute()?;
        }
        Commands::Test => {
            commands::test::execute()?;
        }
        Commands::Scrape { command } => {
            match command {
                None => {
                    // Run all scrapers
                    println!("🔄 Running all scrapers...\n");

                    println!("📊 [1/3] Scraping code...");
                    commands::scrape::execute_code(false, false)?;

                    println!("\n📊 [2/3] Scraping git...");
                    let git_stats = commands::scrape::git::run(false)?;
                    println!("  • {} commits", git_stats.items_processed);

                    println!("\n📚 [3/3] Scraping sessions...");
                    let session_stats = commands::scrape::sessions::run(false)?;
                    println!("  • {} sessions", session_stats.items_processed);

                    println!("\n✅ All scrapers complete!");
                }
                Some(ScrapeCommands::Code { args }) => {
                    commands::scrape::execute_code(args.init, args.force)?;
                }
                Some(ScrapeCommands::Git { full }) => {
                    let stats = commands::scrape::git::run(full)?;
                    println!("\n📊 Git Scrape Summary:");
                    println!("  • Commits processed: {}", stats.items_processed);
                    println!("  • Time elapsed: {:?}", stats.time_elapsed);
                    println!("  • Database size: {} KB", stats.database_size_kb);
                }
                Some(ScrapeCommands::Sessions { full }) => {
                    let stats = commands::scrape::sessions::run(full)?;
                    println!("\n📊 Sessions Scrape Summary:");
                    println!("  • Sessions processed: {}", stats.items_processed);
                    println!("  • Time elapsed: {:?}", stats.time_elapsed);
                    println!("  • Database size: {} KB", stats.database_size_kb);
                }
            }
        }
        Commands::Oxidize => {
            commands::oxidize::oxidize()?;
        }
        Commands::Rebuild {
            scrape,
            oxidize,
            force,
            dry_run,
        } => {
            let options = commands::rebuild::RebuildOptions {
                scrape_only: scrape,
                oxidize_only: oxidize,
                force,
                dry_run,
            };
            commands::rebuild::execute(options)?;
        }
        Commands::Scry {
            query,
            file,
            limit,
            min_score,
            dimension,
            repo,
            all_repos,
            include_issues,
            no_persona,
        } => {
            let options = commands::scry::ScryOptions {
                limit,
                min_score,
                dimension,
                file,
                repo,
                all_repos,
                include_issues,
                include_persona: !no_persona,
            };
            commands::scry::execute(query.as_deref(), options)?;
        }
        Commands::Eval { dimension } => {
            commands::eval::execute(dimension)?;
        }
        Commands::Embeddings { command } => match command {
            EmbeddingsCommands::Generate { force } => {
                commands::embeddings::generate(force)?;
            }
            EmbeddingsCommands::Status => {
                commands::embeddings::status()?;
            }
        },
        Commands::Query { command } => match command {
            QueryCommands::Semantic {
                query,
                r#type,
                min_score,
                limit,
            } => {
                commands::query::semantic::execute(&query, r#type.clone(), min_score, limit)?;
            }
        },
        Commands::Belief { command } => match command {
            BeliefCommands::Validate {
                query,
                min_score,
                limit,
            } => {
                commands::belief::validate::execute(&query, min_score, limit)?;
            }
        },
        Commands::Persona { command } => match command {
            PersonaCommands::Note {
                content,
                domains,
                supersedes,
            } => {
                commands::persona::execute_note(&content, domains, supersedes)?;
            }
            PersonaCommands::Query {
                query,
                limit,
                min_score,
                domains,
            } => {
                commands::persona::execute_query(&query, limit, min_score, domains)?;
            }
            PersonaCommands::List { limit, domains } => {
                commands::persona::execute_list(limit, domains)?;
            }
            PersonaCommands::Materialize => {
                commands::persona::execute_materialize()?;
            }
        },
        Commands::Doctor {
            json,
            repos,
            update,
            audit,
        } => {
            let exit_code = commands::doctor::execute(json, repos, update, audit)?;
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Commands::Ask { args } => {
            commands::ask::run(args)?;
        }
        Commands::Repo {
            command,
            url,
            contrib,
            with_issues,
        } => {
            use commands::repo::RepoCommand;

            let cmd = match (command, url) {
                // Subcommand form: patina repo add/list/update/etc
                (
                    Some(RepoCommands::Add {
                        url,
                        contrib,
                        with_issues,
                    }),
                    _,
                ) => RepoCommand::Add {
                    url,
                    contrib,
                    with_issues,
                },
                (Some(RepoCommands::List), _) => RepoCommand::List,
                (Some(RepoCommands::Update { name, all, oxidize }), _) => {
                    if all {
                        RepoCommand::Update {
                            name: None,
                            oxidize,
                        }
                    } else {
                        RepoCommand::Update { name, oxidize }
                    }
                }
                (Some(RepoCommands::Remove { name }), _) => RepoCommand::Remove { name },
                (Some(RepoCommands::Show { name }), _) => RepoCommand::Show { name },

                // Shorthand form: patina repo <url> [--contrib] [--with-issues]
                (None, Some(url)) => RepoCommand::Add {
                    url,
                    contrib,
                    with_issues,
                },

                // No args: show list
                (None, None) => RepoCommand::List,
            };

            commands::repo::execute(cmd)?;
        }
        Commands::Yolo {
            interactive,
            defaults,
            with,
            without,
            json,
        } => {
            commands::yolo::execute(interactive, defaults, with, without, json)?;
        }
        Commands::Version { json, components } => {
            commands::version::execute(json, components)?;
        }
        Commands::Serve { host, port } => {
            let options = commands::serve::ServeOptions { host, port };
            commands::serve::execute(options)?;
        }
        Commands::Launch {
            path,
            frontend,
            no_serve,
        } => {
            let options = commands::launch::LaunchOptions {
                path,
                frontend,
                auto_start_mothership: !no_serve,
                auto_init: true,
            };
            commands::launch::execute(options)?;
        }
        Commands::Adapter { command } => {
            use patina::adapters::launch as frontend;
            use patina::project;

            match command {
                None | Some(AdapterCommands::List) => {
                    // Show global frontends
                    let frontends = frontend::list()?;
                    println!("📱 Available AI Frontends (Global)\n");
                    println!("{:<12} {:<15} {:<10} VERSION", "NAME", "DISPLAY", "STATUS");
                    println!("{}", "─".repeat(50));
                    for f in frontends {
                        let status = if f.detected {
                            "✓ found"
                        } else {
                            "✗ missing"
                        };
                        let version = f.version.unwrap_or_else(|| "-".to_string());
                        println!(
                            "{:<12} {:<15} {:<10} {}",
                            f.name, f.display, status, version
                        );
                    }

                    let default = frontend::default_name()?;
                    println!("\nGlobal default: {}", default);

                    // Show project frontends if in a patina project
                    let cwd = std::env::current_dir()?;
                    if project::is_patina_project(&cwd) {
                        let config = project::load_with_migration(&cwd)?;
                        println!("\n📁 Project Allowed Frontends\n");
                        println!("Allowed: {:?}", config.frontends.allowed);
                        println!("Project default: {}", config.frontends.default);
                    }
                }
                Some(AdapterCommands::Default { name, project: is_project }) => {
                    if is_project {
                        // Set project default
                        let cwd = std::env::current_dir()?;
                        if !project::is_patina_project(&cwd) {
                            anyhow::bail!("Not a patina project. Run `patina init .` first.");
                        }
                        let mut config = project::load_with_migration(&cwd)?;
                        if !config.frontends.allowed.contains(&name) {
                            anyhow::bail!(
                                "Frontend '{}' is not in allowed list. Add it first: patina adapter add {}",
                                name, name
                            );
                        }
                        config.frontends.default = name.clone();
                        project::save(&cwd, &config)?;
                        println!("✓ Project default frontend set to: {}", name);
                    } else {
                        // Set global default
                        frontend::set_default(&name)?;
                        println!("✓ Global default frontend set to: {}", name);
                    }
                }
                Some(AdapterCommands::Check { name }) => {
                    if let Some(n) = name {
                        let f = frontend::get(&n)?;
                        if f.detected {
                            println!("✓ {} is installed", f.display);
                            if let Some(v) = f.version {
                                println!("  Version: {}", v);
                            }
                        } else {
                            println!("✗ {} is not installed", f.display);
                        }
                    } else {
                        // Check all
                        for f in frontend::list()? {
                            let status = if f.detected { "✓" } else { "✗" };
                            println!("{} {}", status, f.display);
                        }
                    }
                }
                Some(AdapterCommands::Add { name }) => {
                    // Verify frontend exists
                    let _ = frontend::get(&name)?;

                    let cwd = std::env::current_dir()?;
                    if !project::is_patina_project(&cwd) {
                        anyhow::bail!("Not a patina project. Run `patina init .` first.");
                    }

                    let mut config = project::load_with_migration(&cwd)?;
                    if config.frontends.allowed.contains(&name) {
                        println!("Frontend '{}' is already in allowed list.", name);
                        return Ok(());
                    }

                    config.frontends.allowed.push(name.clone());
                    project::save(&cwd, &config)?;

                    println!("✓ Added '{}' to allowed frontends", name);
                    println!("  Allowed: {:?}", config.frontends.allowed);

                    // TODO: Copy adapter templates if needed
                }
                Some(AdapterCommands::Remove { name, no_backup }) => {
                    let cwd = std::env::current_dir()?;
                    if !project::is_patina_project(&cwd) {
                        anyhow::bail!("Not a patina project. Run `patina init .` first.");
                    }

                    let mut config = project::load_with_migration(&cwd)?;
                    if !config.frontends.allowed.contains(&name) {
                        println!("Frontend '{}' is not in allowed list.", name);
                        return Ok(());
                    }

                    // Backup files if requested
                    if !no_backup {
                        // Backup bootstrap file (CLAUDE.md, GEMINI.md, etc.)
                        let bootstrap_file = match name.as_str() {
                            "claude" => "CLAUDE.md",
                            "gemini" => "GEMINI.md",
                            "codex" => "CODEX.md",
                            _ => "",
                        };
                        if !bootstrap_file.is_empty() {
                            let file_path = cwd.join(bootstrap_file);
                            if let Some(backup_path) = project::backup_file(&cwd, &file_path)? {
                                println!("  ✓ Backed up {} to {}", bootstrap_file, backup_path.display());
                            }
                        }

                        // Backup adapter directory (.claude/, .gemini/, etc.)
                        let adapter_dir = cwd.join(format!(".{}", name));
                        if adapter_dir.exists() {
                            // For directories, we just note they exist - full backup would be complex
                            println!("  ⚠ Adapter directory .{}/ exists (not backed up)", name);
                        }
                    }

                    // Remove from allowed list
                    config.frontends.allowed.retain(|f| f != &name);

                    // Update default if we removed it
                    if config.frontends.default == name {
                        config.frontends.default = config
                            .frontends
                            .allowed
                            .first()
                            .cloned()
                            .unwrap_or_default();
                        if !config.frontends.default.is_empty() {
                            println!("  ✓ Default changed to: {}", config.frontends.default);
                        }
                    }

                    project::save(&cwd, &config)?;

                    println!("✓ Removed '{}' from allowed frontends", name);
                    println!("  Allowed: {:?}", config.frontends.allowed);
                    println!("\n💡 To also remove files: rm -rf .{}/ {}.md", name, name.to_uppercase());
                }
            }
        }
    }

    Ok(())
}
