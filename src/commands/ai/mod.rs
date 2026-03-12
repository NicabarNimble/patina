mod internal;
pub(crate) mod surface;

use anyhow::Result;
use clap::Args;

#[derive(Debug, Clone, clap::Subcommand)]
pub enum AiSessionCommands {
    /// Start an AI session without launching an interface
    Start {
        title: String,

        #[arg(long)]
        adapter: Option<String>,

        #[arg(long)]
        json: bool,
    },

    /// Update the current AI session artifact
    Update {
        #[arg(long)]
        session: Option<String>,

        #[arg(long)]
        json: bool,
    },

    /// End an AI session and archive its durable artifact
    End {
        #[arg(long)]
        session: Option<String>,

        #[arg(long)]
        note: Option<String>,

        #[arg(long)]
        json: bool,
    },

    /// List active AI sessions for this project
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Args)]
pub struct AiLaunchArgs {
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

    #[arg(long, default_value_t = false)]
    default: bool,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum AiCommands {
    /// Deploy the Patina AI bundles for this project
    Setup {
        #[arg(value_name = "interface", hide = true)]
        interface: Option<String>,

        #[arg(long)]
        path: Option<String>,

        #[arg(long)]
        force: bool,
    },

    /// Refresh one bundle or all Patina AI bundles for this project
    Refresh {
        #[arg(value_name = "interface")]
        interface: Option<String>,

        #[arg(long)]
        path: Option<String>,

        #[arg(long)]
        force: bool,
    },

    /// Launch Claude Code through the Patina AI interface
    Claude {
        #[command(flatten)]
        launch: AiLaunchArgs,
    },

    /// Launch OpenCode through the Patina AI interface
    #[command(name = "opencode")]
    OpenCode {
        #[command(flatten)]
        launch: AiLaunchArgs,
    },

    /// Launch Gemini CLI through the Patina AI interface
    Gemini {
        #[command(flatten)]
        launch: AiLaunchArgs,
    },

    /// List active Patina AI sessions for this project
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

    /// Machine-readable AI session lifecycle commands
    Session {
        #[command(subcommand)]
        command: AiSessionCommands,
    },
}

pub fn execute(command: Option<AiCommands>) -> Result<()> {
    match command {
        None => surface::launch_default(),
        Some(AiCommands::Setup {
            interface,
            path,
            force,
        }) => surface::setup(surface::AiSetupRequest {
            interface,
            path,
            force,
        }),
        Some(AiCommands::Refresh {
            interface,
            path,
            force,
        }) => surface::refresh(surface::AiRefreshRequest {
            interface,
            path,
            force,
        }),
        Some(AiCommands::Claude { launch }) => surface::launch(surface::AiLaunchRequest {
            interface_name: "claude".to_string(),
            title: launch.title,
            requested_session: launch.session,
            persona: launch.persona,
            path: launch.path,
            no_tmux: launch.no_tmux,
            set_default: launch.default,
        }),
        Some(AiCommands::OpenCode { launch }) => surface::launch(surface::AiLaunchRequest {
            interface_name: "opencode".to_string(),
            title: launch.title,
            requested_session: launch.session,
            persona: launch.persona,
            path: launch.path,
            no_tmux: launch.no_tmux,
            set_default: launch.default,
        }),
        Some(AiCommands::Gemini { launch }) => surface::launch(surface::AiLaunchRequest {
            interface_name: "gemini".to_string(),
            title: launch.title,
            requested_session: launch.session,
            persona: launch.persona,
            path: launch.path,
            no_tmux: launch.no_tmux,
            set_default: launch.default,
        }),
        Some(AiCommands::List { json }) => internal::list(json),
        Some(AiCommands::End {
            session,
            note,
            json,
        }) => internal::end(session, note, json),
        Some(AiCommands::Session { command }) => internal::session(command),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct AiCli {
        #[command(subcommand)]
        command: AiCommands,
    }

    #[test]
    fn launch_peer_commands_accept_default_flag() {
        for args in [
            ["patina", "claude", "--default"],
            ["patina", "opencode", "--default"],
            ["patina", "gemini", "--default"],
        ] {
            let parsed = AiCli::try_parse_from(args).unwrap();
            match parsed.command {
                AiCommands::Claude { launch }
                | AiCommands::OpenCode { launch }
                | AiCommands::Gemini { launch } => assert!(launch.default),
                other => panic!("unexpected command: {other:?}"),
            }
        }
    }

    #[test]
    fn setup_command_no_longer_requires_interface_argument() {
        let parsed = AiCli::try_parse_from(["patina", "setup"]).unwrap();
        match parsed.command {
            AiCommands::Setup { interface, .. } => assert!(interface.is_none()),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn refresh_command_accepts_optional_interface_argument() {
        let parsed = AiCli::try_parse_from(["patina", "refresh", "gemini"]).unwrap();
        match parsed.command {
            AiCommands::Refresh { interface, .. } => {
                assert_eq!(interface.as_deref(), Some("gemini"))
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }
}
