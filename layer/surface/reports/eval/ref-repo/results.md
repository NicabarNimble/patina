# Ref Repo Semantic Eval Results

Generated: 2026-01-07T17:38:12Z

## gemini-cli

| Query | Expected | Found in Top 5 | Rank | Hit? |
|-------|----------|----------------|------|------|
| how does telemetry work | packages/core/src/telemetry/telemetry-utils.ts | - | - | NO |
| configuration and settings management | packages/core/src/config/config.ts | - | - | NO |
| hook system and event handling | packages/core/src/hooks/hookSystem.ts | packages/core/src/hooks/hookEventHandler.ts | 1 | YES |
| MCP server client connection | packages/core/src/tools/mcp-client.ts | packages/core/src/tools/mcp-client.ts | 1 | YES |
| session management and cleanup | packages/core/src/utils/session.ts | packages/cli/src/utils/sessionCleanup.ts | 1 | YES |
| file reading and writing tools | packages/core/src/tools/read-file.ts | packages/core/src/tools/write-file.ts | 1 | YES |
| grep and search functionality | packages/core/src/tools/grep.ts | packages/core/src/tools/grep.ts | 1 | YES |
| IDE integration and detection | packages/core/src/ide/detect-ide.ts | packages/core/src/ide/detect-ide.ts | 1 | YES |

**Summary:** Hit Rate: 75.0% (6/8), MRR: .750

## opencode

| Query | Expected | Found in Top 5 | Rank | Hit? |
|-------|----------|----------------|------|------|
| authentication and OAuth flow | packages/opencode/src/mcp/auth.ts | packages/opencode/src/mcp/auth.ts | 1 | YES |
| terminal UI component rendering | packages/app/src/components/terminal.tsx | packages/app/src/components/terminal.tsx | 3 | YES |
| autocomplete and suggestions | packages/opencode/src/cli/cmd/tui/component/prompt/autocomplete.tsx | packages/opencode/src/cli/cmd/tui/component/prompt/autocomplete.tsx | 1 | YES |
| dialog and modal components | packages/ui/src/context/dialog.tsx | - | - | NO |
| diff viewer and code comparison | packages/ui/src/components/diff.tsx | - | - | NO |
| markdown rendering | packages/ui/src/components/markdown.tsx | packages/ui/src/components/markdown.tsx | 3 | YES |

**Summary:** Hit Rate: 66.6% (4/6), MRR: .444

## dojo

| Query | Expected | Found in Top 5 | Rank | Hit? |
|-------|----------|----------------|------|------|
| world storage and state management | crates/dojo/core/src/world/storage.cairo | crates/dojo/core/src/world/storage.cairo | 1 | YES |
| model definition and handling | crates/dojo/core/src/model/model.cairo | crates/dojo/core/src/model/model.cairo | 5 | YES |
| TypeScript code generation bindgen | crates/dojo/bindgen/src/plugins/typescript/generator/schema.rs | - | - | NO |
| Unity game engine bindings | crates/dojo/bindgen/src/plugins/unity/mod.rs | crates/dojo/bindgen/src/plugins/unity/mod.rs | 1 | YES |
| configuration and environment | crates/dojo/world/src/config/mod.rs | crates/dojo/world/src/config/environment.rs | 5 | YES |
| transaction error handling | crates/dojo/utils/src/tx/error.rs | crates/dojo/utils/src/tx/error.rs | 1 | YES |

**Summary:** Hit Rate: 83.3% (5/6), MRR: .566

## codex

| Query | Expected | Found in Top 5 | Rank | Hit? |
|-------|----------|----------------|------|------|
| tool execution and handlers | codex-rs/core/src/tools | codex-rs/core/src/tools | 1 | YES |
| sandbox security and isolation | codex-rs/core/src/tools/sandboxing.rs | codex-rs/core/src/tools/sandboxing.rs | 4 | YES |
| agent message parsing and events | codex-rs/core/src/event_mapping.rs | codex-rs/core/src/event_mapping.rs | 1 | YES |
| OAuth and authentication | codex-rs/rmcp-client/src/oauth.rs | codex-rs/rmcp-client/src/oauth.rs | 1 | YES |
| TUI chat widget display | codex-rs/tui/src/chatwidget | codex-rs/tui/src/chatwidget | 2 | YES |
| API client and SSE streaming | codex-rs/codex-api/src/sse/chat.rs | - | - | NO |

**Summary:** Hit Rate: 83.3% (5/6), MRR: .625


---
Hit Rate = % of queries where at least one expected file appears in top 5
MRR = Mean Reciprocal Rank (1/rank of first hit, averaged)
