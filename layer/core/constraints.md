# Core Constraints

These constraints are always enforced across all projects and contexts.

## Language & Tools
- **Primary language**: Rust (always)
- **Async runtime**: Tokio (when async is needed)
- **Error handling**: anyhow for applications, thiserror for libraries
- **Serialization**: Serde (JSON/TOML as needed)
- **CLI framework**: clap for argument parsing
- **TUI framework**: ratatui when needed

## Code Style
- **Format**: rustfmt with default settings
- **Linting**: clippy with pedantic for libraries
- **Documentation**: Doc comments on all public APIs
- **Examples**: Working examples for all major features

## Architecture Constraints
- **No tight coupling**: Components communicate through traits
- **Plugin architecture**: Core functionality with pluggable adapters
- **Environment agnostic**: Must work on Mac, Linux, container environments
- **Context first**: Every decision optimizes for context clarity

## Safety & Security
- **No unsafe code** without explicit justification
- **No network calls** without user consent
- **No file writes** outside designated directories
- **Respect system boundaries**: Never modify system files

## Development Workflow
- **Test before commit**: All tests must pass
- **Document decisions**: Major choices recorded in sessions
- **Incremental progress**: Small, focused changes
- **User consent**: Always ask before major operations