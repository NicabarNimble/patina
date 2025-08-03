# Git Hooks Setup for Patina

This guide explains how to enable git-aware hooks in Claude Code for Patina development.

## Quick Setup

1. Ensure you have the required dependencies:
   ```bash
   which jq || echo "Please install jq for JSON parsing"
   ```

2. Create `.claude/hooks.json` in your Patina directory:
   ```json
   {
     "hooks": {
       "PostToolUse": [
         {
           "matcher": "Bash",
           "hooks": [
             {
               "type": "command",
               "command": "echo \"$CLAUDE_TOOL_INPUT\" | jq -r '.tool_input.command' | grep -q 'session-start\\.sh' && .claude/hooks/post-session-start.sh"
             }
           ]
         },
         {
           "matcher": "Bash",
           "hooks": [
             {
               "type": "command",
               "command": "echo \"$CLAUDE_TOOL_INPUT\" | jq -r '.tool_input.command' | grep -q 'session-update\\.sh' && .claude/hooks/post-session-update.sh"
             }
           ]
         }
       ],
       "PreToolUse": [
         {
           "matcher": "Bash",
           "hooks": [
             {
               "type": "command",
               "command": "echo \"$CLAUDE_TOOL_INPUT\" | jq -r '.tool_input.command' | grep -q 'session-end\\.sh' && .claude/hooks/pre-session-end.sh"
             }
           ]
         }
       ]
     }
   }
   ```

3. Restart Claude Code for hooks to take effect

## What the Hooks Do

- **post-session-start**: Checks git status and suggests workflows
- **post-session-update**: Reminds about commits if many changes
- **pre-session-end**: Warns about uncommitted changes

## Hook Scripts Location

The hook scripts are already included in `.claude/hooks/` when you run `patina init`.

## Troubleshooting

- Hooks require `jq` for JSON parsing
- Check `.claude/hooks/` has executable scripts
- Hooks configuration is local (not committed to git)