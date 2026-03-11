End the current Patina live session using the truthful Patina surface for this runtime:

1. Run `/session-update` first if recent work has not been captured yet.

2. Read `.opencode/PATINA.md` first and determine whether it says Patina MCP is available in this OpenCode runtime.

3. End the session using exactly one truthful path:
   - If `.opencode/PATINA.md` says MCP is available, call MCP tool `session.end`.
   - Otherwise, execute `patina ai session end --json`.
   - If multiple active sessions exist, use `session.list` on the MCP path or `patina ai session list --json` on the native fallback path, then retry with `session=<runtime_id|file_id>`.
   - If you need a final outcome sentence, pass it as `note` on the MCP path or `--note` on the native fallback path.

4. Use the returned fields to summarize the outcome:
   - `classification`
   - `files_changed`
   - `commits_made`
   - `start_tag` and `end_tag`
   - `artifact_path`

5. Remind the user that the durable session is archived and the `last_session_path` pointer was updated.
