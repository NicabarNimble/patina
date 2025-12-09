# Available Tools

Commands and MCP tools for this project.

## CLI Commands

### Launcher
```bash
patina launch              # Open in default frontend
patina launch claude       # Open in Claude Code
patina launch -f gemini    # Open in Gemini CLI
patina adapter list        # Show available frontends
patina adapter default X   # Set default frontend
```

### Knowledge
```bash
patina scry "query"        # Search project knowledge
patina scry --file src/x.rs  # Find related files
patina scry --dimension temporal  # Co-change patterns
patina scry --all-repos    # Search all registered repos
```

### Project
```bash
patina init . --llm=claude  # Initialize project
patina rebuild             # Rebuild indices from layer/
patina scrape              # Build knowledge database
patina oxidize             # Train embeddings
```

### Daemon
```bash
patina serve               # Start mothership daemon
patina serve --host 0.0.0.0  # Allow container access
```

### External Repos
```bash
patina repo add owner/repo  # Add reference repo
patina repo list           # Show registered repos
patina repo update --all   # Update all repos
```

## MCP Tools (Future)

When connected via MCP, these tools become available:

| Tool | Purpose |
|------|---------|
| `patina_context` | Get project context and rules |
| `patina_scry` | Search codebase knowledge |
| `patina_session_start` | Begin tracked session |
| `patina_session_end` | End session, capture learnings |
| `patina_session_note` | Capture insight during session |

## Session Commands (Claude)

```bash
/session-start [name]      # Begin session with Git tracking
/session-update            # Update progress
/session-note [insight]    # Capture insight
/session-end               # End session, distill learnings
```

## Development

```bash
# Always run before push
./resources/git/pre-push-checks.sh

# Or individually
cargo fmt --all
cargo clippy --workspace
cargo test --workspace
```
