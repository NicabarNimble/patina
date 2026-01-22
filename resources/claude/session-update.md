Update the current Patina session with Git-aware progress tracking:

1. Execute the session update script:
   `.claude/bin/session-update.sh`

2. The script will show what time period to document (e.g., "14:15 → 14:45")

3. Note the Git status shown (uncommitted changes, last commit time)

4. Read `.claude/context/active-session.md` and find the new update section

5. Fill in the update section with what happened during that time period:
   - **Work completed**: Code written, files modified, problems solved
   - **Discussion context**: Key questions asked, reasoning frameworks used, why we chose this approach
   - **Key decisions**: Design choices, trade-offs, reasoning behind changes
   - **Challenges faced**: Errors encountered, debugging steps, solutions found
   - **Patterns observed**: Reusable insights, things that worked well

6. **Check for beliefs to capture**: Review the update and ask yourself:
   - Any design decisions made? ("We chose X because Y")
   - Any repeated patterns? (Said 3+ times)
   - Any strong principles? ("Never do X", "Always Y")
   - Any lessons learned? ("That was wrong because...")

   If yes, suggest to user: "This sounds like a belief worth capturing: '{statement}'. Should I create it?"

7. If the script suggests a commit (30+ minutes or 100+ lines changed), consider:
   - Creating a checkpoint: `git commit -am "checkpoint: [description]"`
   - Breaking large changes into smaller logical commits

Note: Each update creates a time-stamped checkpoint with Git context for future reference.