mod internal;

use anyhow::Result;

#[derive(Debug, Clone, clap::Subcommand)]
pub enum AiSessionCommands {
    /// Start a native AI session without launching an interface
    Start {
        title: String,

        #[arg(long)]
        adapter: Option<String>,

        #[arg(long)]
        json: bool,
    },

    /// Update the current native AI session artifact
    Update {
        #[arg(long)]
        session: Option<String>,

        #[arg(long)]
        json: bool,
    },

    /// End a native AI session and archive its durable artifact
    End {
        #[arg(long)]
        session: Option<String>,

        #[arg(long)]
        note: Option<String>,

        #[arg(long)]
        json: bool,
    },

    /// List active native AI sessions for this project
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum AiCommands {
    /// Create or refresh Patina-managed projection for a native AI interface
    Setup {
        adapter: String,

        #[arg(long)]
        path: Option<String>,
    },

    /// Launch OpenCode through the native Patina AI interface
    #[command(name = "opencode")]
    OpenCode {
        #[arg(long)]
        title: Option<String>,

        #[arg(long)]
        session: Option<String>,

        #[arg(long)]
        persona: Option<String>,

        #[arg(long)]
        path: Option<String>,

        #[arg(long)]
        no_tmux: bool,
    },

    /// Launch Gemini CLI through the native Patina AI interface
    Gemini {
        #[arg(long)]
        title: Option<String>,

        #[arg(long)]
        session: Option<String>,

        #[arg(long)]
        persona: Option<String>,

        #[arg(long)]
        path: Option<String>,

        #[arg(long)]
        no_tmux: bool,
    },

    /// List active Mother-backed AI sessions for this project
    List {
        #[arg(long)]
        json: bool,
    },

    /// End an active AI session and archive its durable artifact
    End {
        #[arg(long)]
        session: Option<String>,

        #[arg(long)]
        note: Option<String>,

        #[arg(long)]
        json: bool,
    },

    /// Machine-readable native session lifecycle commands
    Session {
        #[command(subcommand)]
        command: AiSessionCommands,
    },
}

pub fn execute(command: Option<AiCommands>) -> Result<()> {
    match command {
        None => internal::launch_default(),
        Some(AiCommands::Setup { adapter, path }) => internal::setup(&adapter, path),
        Some(AiCommands::OpenCode {
            title,
            session,
            persona,
            path,
            no_tmux,
        }) => internal::launch("opencode", title, session, persona, path, no_tmux),
        Some(AiCommands::Gemini {
            title,
            session,
            persona,
            path,
            no_tmux,
        }) => internal::launch("gemini", title, session, persona, path, no_tmux),
        Some(AiCommands::List { json }) => internal::list(json),
        Some(AiCommands::End {
            session,
            note,
            json,
        }) => internal::end(session, note, json),
        Some(AiCommands::Session { command }) => internal::session(command),
    }
}
