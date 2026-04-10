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
        json: bool,
    },

    /// Update the current AI session artifact
    Update {
        #[arg(long)]
        session: Option<String>,

        #[arg(long)]
        json: bool,
    },

    /// Add a note to the current AI session artifact
    Note {
        content: String,

        #[arg(long)]
        session: Option<String>,
    },

    /// End an AI session and archive its durable artifact
    End {
        #[arg(long)]
        session: Option<String>,

        #[arg(long)]
        note: Option<String>,

        #[arg(long)]
        commit: bool,

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
    voice: Option<String>,

    #[arg(long)]
    path: Option<String>,

    #[arg(long, default_value_t = false)]
    default: bool,

    #[arg(long, default_value_t = false, conflicts_with = "no_tmux")]
    tmux: bool,

    #[arg(long, default_value_t = false, conflicts_with = "tmux")]
    no_tmux: bool,
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

    /// Launch PI through the Patina AI interface
    Pi {
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
        commit: bool,

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
            voice: launch.voice,
            path: launch.path,
            set_default: launch.default,
            tmux: launch.tmux,
            no_tmux: launch.no_tmux,
        }),
        Some(AiCommands::OpenCode { launch }) => surface::launch(surface::AiLaunchRequest {
            interface_name: "opencode".to_string(),
            title: launch.title,
            requested_session: launch.session,
            voice: launch.voice,
            path: launch.path,
            set_default: launch.default,
            tmux: launch.tmux,
            no_tmux: launch.no_tmux,
        }),
        Some(AiCommands::Gemini { launch }) => surface::launch(surface::AiLaunchRequest {
            interface_name: "gemini".to_string(),
            title: launch.title,
            requested_session: launch.session,
            voice: launch.voice,
            path: launch.path,
            set_default: launch.default,
            tmux: launch.tmux,
            no_tmux: launch.no_tmux,
        }),
        Some(AiCommands::Pi { launch }) => surface::launch(surface::AiLaunchRequest {
            interface_name: "pi".to_string(),
            title: launch.title,
            requested_session: launch.session,
            voice: launch.voice,
            path: launch.path,
            set_default: launch.default,
            tmux: launch.tmux,
            no_tmux: launch.no_tmux,
        }),
        Some(AiCommands::List { json }) => internal::list(json),
        Some(AiCommands::End {
            session,
            note,
            commit,
            json,
        }) => internal::end(session, note, commit, json),
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
            ["patina", "pi", "--default"],
        ] {
            let parsed = AiCli::try_parse_from(args).unwrap();
            match parsed.command {
                AiCommands::Claude { launch }
                | AiCommands::OpenCode { launch }
                | AiCommands::Gemini { launch }
                | AiCommands::Pi { launch } => assert!(launch.default),
                other => panic!("unexpected command: {other:?}"),
            }
        }
    }

    #[test]
    fn launch_peer_commands_accept_tmux_flags() {
        for args in [
            ["patina", "claude", "--tmux"],
            ["patina", "opencode", "--no-tmux"],
            ["patina", "gemini", "--tmux"],
            ["patina", "pi", "--no-tmux"],
        ] {
            let parsed = AiCli::try_parse_from(args).unwrap();
            match parsed.command {
                AiCommands::Claude { launch }
                | AiCommands::OpenCode { launch }
                | AiCommands::Gemini { launch }
                | AiCommands::Pi { launch } => {
                    assert!(launch.tmux || launch.no_tmux)
                }
                other => panic!("unexpected command: {other:?}"),
            }
        }
    }

    #[test]
    fn launch_peer_commands_reject_conflicting_tmux_flags() {
        let parsed = AiCli::try_parse_from(["patina", "claude", "--tmux", "--no-tmux"]);
        assert!(parsed.is_err());
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

    #[test]
    fn ai_session_note_command_parses_content_and_selector() {
        let parsed = AiCli::try_parse_from([
            "patina",
            "session",
            "note",
            "capture this",
            "--session",
            "runtime-123",
        ])
        .unwrap();

        match parsed.command {
            AiCommands::Session {
                command:
                    AiSessionCommands::Note {
                        content, session, ..
                    },
            } => {
                assert_eq!(content, "capture this");
                assert_eq!(session.as_deref(), Some("runtime-123"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn end_commands_accept_commit_flag() {
        let parsed = AiCli::try_parse_from(["patina", "end", "--commit"]).unwrap();
        match parsed.command {
            AiCommands::End { commit, .. } => assert!(commit),
            other => panic!("unexpected command: {other:?}"),
        }

        let parsed = AiCli::try_parse_from(["patina", "session", "end", "--commit"]).unwrap();
        match parsed.command {
            AiCommands::Session {
                command: AiSessionCommands::End { commit, .. },
            } => assert!(commit),
            other => panic!("unexpected command: {other:?}"),
        }
    }
}
