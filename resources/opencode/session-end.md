End the current Patina live session:

1. Run `/session-update` first if recent work has not been captured yet.

2. End the session with the bundled wrapper:
   - `.opencode/bin/session-end.sh`

3. Use the returned fields to summarize the outcome:
   - `classification`
   - `files_changed`
   - `commits_made`
   - `start_tag` and `end_tag`
   - `artifact_path`

4. Remind the user that the durable session is archived and the `last_session_path` pointer was updated.
