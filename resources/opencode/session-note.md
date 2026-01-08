Add a human note to the current Patina session with Git context:

1. Run the session note script with your insight:
   - Execute: `.opencode/bin/session-note.sh "$ARGUMENTS"`
   - Example: `.opencode/bin/session-note.sh "discovered dual session architecture is key"`

2. The script will add Git context [branch@sha] to the note

3. Confirm the note was added:
   - Say: "Note added [branch@sha]: [what user said]"

4. If the note contains keywords (breakthrough, discovered, solved, fixed):
   - The script will suggest a checkpoint commit
   - Consider: `git commit -am "checkpoint: [discovery]"`

5. Purpose of Git-linked notes:
   - Create searchable memory tied to specific code states
   - Enable future queries like "when did we solve X?"
   - Build knowledge graph through Git history

Note: These notes are prioritized during session-end distillation and become searchable through Git.