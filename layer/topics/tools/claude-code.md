---
id: claude-code
version: 1
created_date: 2025-07-25
confidence: medium
oxidizer: nicabar
tags: []
---

# Claude Code Reference

**Current Version**: 1.0.60  
**Last Checked**: 2025-01-25  
**Source**: https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md

## What is Claude Code

Claude Code is Anthropic's terminal-based AI coding assistant that integrates directly into developer workflows. It operates as a command-line tool that can read files, execute commands, modify code, and interact with development tools while maintaining conversational context.

## Core Capabilities

### Direct Terminal Integration
- Runs directly in the terminal alongside existing tools
- Can execute shell commands via Bash tool
- Reads and writes files with user permission
- Maintains working directory context
- Supports piping and command chaining (e.g., `tail -f app.log | claude -p 'alert on errors'`)

### Development Operations
- **Code Generation**: Creates features from natural language descriptions
- **Code Modification**: Edits existing files with permission-based safeguards  
- **Debugging**: Analyzes errors, suggests fixes, implements solutions
- **Testing**: Writes and runs tests, analyzes test failures
- **Refactoring**: Improves code structure while maintaining functionality
- **Documentation**: Generates and updates documentation
- **Git Operations**: Creates commits, manages branches, reviews changes

## Command Structure

### Basic Commands
- `claude` - Start interactive REPL session
- `claude "task description"` - Start with initial task
- `claude -p "query"` - One-off query, exits after response
- `claude -c` - Continue most recent conversation
- `claude -r <session-id>` - Resume specific session
- `claude update` - Update to latest version
- `claude mcp` - Configure Model Context Protocol servers

### Key Flags
- `--add-dir <path>` - Add additional working directories
- `--allowedTools <tools>` - Pre-approve specific tool usage
- `--model <model>` - Select model (sonnet, opus, etc.)
- `--output-format <format>` - Set output format (text, json, stream-json)
- `--permission-mode <mode>` - Start in specific permission mode
- `--verbose` - Enable detailed logging

## Slash Commands

Built-in commands available during sessions:
- `/add-dir` - Add working directories
- `/agents` - Manage sub-agents
- `/clear` - Clear conversation history
- `/compact` - Compress conversation with optional focus
- `/config` - View/modify configuration
- `/cost` - Show token usage
- `/help` - Display available commands
- `/init` - Initialize project with CLAUDE.md
- `/memory` - Edit memory files
- `/model` - Change active model
- `/permissions` - Manage access permissions
- `/review` - Request code review

## Memory System

### CLAUDE.md Files
- **Project Memory** (`./CLAUDE.md`): Team-shared project context
- **User Memory** (`~/.claude/CLAUDE.md`): Personal preferences
- Automatically loaded on startup
- Supports imports via `@path/to/file` syntax
- Recursive discovery up directory tree
- Quick memory addition with `#` prefix

## Model Context Protocol (MCP)

Enables connection to external tools and data sources:
- **Server Types**: stdio, SSE, HTTP
- **Resource Access**: Use `@` mentions for external resources
- **Custom Commands**: `/mcp__servername__commandname` format
- **Authentication**: OAuth 2.0 support
- **Scopes**: Local, project, or user-level

Full MCP docs: https://docs.anthropic.com/en/docs/claude-code/mcp

## Sub-Agents System

Specialized AI assistants for focused tasks:
- **Creation**: `/agents` command or manual file creation
- **Storage**: `.claude/agents/` (project) or `~/.claude/agents/` (user)
- **Structure**: Markdown with YAML frontmatter
- **Invocation**: Automatic or explicit
- **Context**: Separate from main conversation
- **Tools**: Configurable per agent

Sub-agents docs: https://docs.anthropic.com/en/docs/claude-code/sub-agents

## Hooks System

User-defined commands that execute at specific lifecycle points:
- **PreToolUse**: Before tool execution (can block)
- **PostToolUse**: After tool completion
- **Notification**: On notifications
- **Stop**: When Claude Code finishes
- **SubAgentStop**: When sub-agent completes

Configured via `.claude/config.json` or user config.

Hooks guide: https://docs.anthropic.com/en/docs/claude-code/hooks-guide

## SDK Integration

Programmatic usage for building AI-powered tools:
- **TypeScript**: `@anthropic-ai/claude-code` npm package
- **Python**: `claude-code-sdk` PyPI package
- **Features**: Multi-turn conversations, custom prompts, MCP support
- **Authentication**: Anthropic API, Bedrock, Vertex AI

SDK docs: https://docs.anthropic.com/en/docs/claude-code/sdk

## Custom Commands

Project or user-defined slash commands:
- **Location**: `.claude/commands/` or `~/.claude/commands/`
- **Format**: Markdown files with optional YAML frontmatter
- **Arguments**: Support via `$ARGUMENTS` placeholder
- **Namespacing**: Organized in subdirectories

## Security and Permissions

- Always requests permission before file modifications
- Hooks run with current environment credentials
- Third-party MCP servers carry prompt injection risks
- Configurable permission modes and tool allowlists

## Installation and Requirements

```bash
npm install -g @anthropic-ai/claude-code
```

- Requires Node.js 18 or newer
- Available on macOS, Linux, Windows
- Supports major shells (bash, zsh, fish, PowerShell)

## Version Checking

To check for updates to this reference:
- Review: https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md
- Current tracked version: 1.0.60

---

*This reference provides comprehensive understanding of Claude Code's capabilities for AI assistants. For implementation details specific to Patina integration, see brain/projects/patina/claude-integration.md*