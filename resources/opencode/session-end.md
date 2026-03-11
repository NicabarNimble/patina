End the current Patina live session through MCP:

1. Run `/session-update` first if recent work has not been captured yet.

2. Call MCP tool `session.end`.
   - If multiple active sessions exist, call `session.list` first and retry with `session=<runtime_id|file_id>`.
   - If you need a final outcome sentence, pass it as `note`.

3. Use the returned fields to summarize the outcome:
   - `classification`
   - `files_changed`
   - `commits_made`
   - `start_tag` and `end_tag`
   - `artifact_path`

4. Remind the user that the durable session is archived and the `last_session_path` pointer was updated.
