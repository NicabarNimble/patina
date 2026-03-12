End the current Patina live session using the truthful Patina surface for this runtime:

1. Run `/session-update` first if recent work has not been captured yet.

2. Read root `AGENTS.md` first. Even if the `OpenCode` runtime section says Patina MCP is available, session lifecycle uses the native machine-readable CLI path for this phase.

3. End the session with:
   - `patina ai session end --json`
   - If multiple active sessions exist, use `patina ai session list --json`, then retry with `--session <runtime_id|file_id>`.
   - If you need a final outcome sentence, pass `--note "<text>"`.

4. Use the returned fields to summarize the outcome:
   - `classification`
   - `files_changed`
   - `commits_made`
   - `start_tag` and `end_tag`
   - `artifact_path`

5. Remind the user that the durable session is archived and the `last_session_path` pointer was updated.
