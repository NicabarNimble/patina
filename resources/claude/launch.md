# /launch - Create implementation branch from session

Launch implementation branch from current session conversation.

## What it does

1. **Extracts implementation plan** from current session discussion
2. **Creates appropriate branch** based on current location:
   - If on `work` → creates `experiment/name` or `feature/name`
   - If on `work/something` → creates sub-branch `work/something/name`
   - If on other branch → suggests switching to work first
3. **Generates TODO** from conversation markers
4. **Creates Draft PR** with implementation plan
5. **Continues session** on new branch (doesn't end it)

## Usage

```
/launch [type/]name
```

## Examples

```bash
# Auto-detect type from session content
/launch semantic-scraping

# Explicit experiment branch
/launch experiment/semantic-scraping

# Explicit feature branch
/launch feature/semantic-scraping
```

## Branch Creation Logic

- `work` branch → `experiment/semantic-scraping` or `feature/semantic-scraping`
- `work/algo` branch → `work/algo/semantic-scraping`
- `main` branch → prompts to switch to `work` first
- Other branches → warns and asks for confirmation

## What Gets Created

1. **New Git branch** following Patina conventions
2. **IMPLEMENTATION_PLAN.md** with:
   - Extracted design from session
   - Key decisions made
   - TODO items found
   - Links to parent session
3. **Draft GitHub PR** with:
   - Session context
   - Implementation plan
   - TODO checklist
   - Test plan template

## Session Integration

The command:
- Reads from `.claude/context/active-session.md`
- Adds launch note to current session
- Maintains session continuity (no restart needed)
- Tags the branch point for tracking

## Extraction Markers

During conversation, the AI should use these markers for better extraction:

```markdown
## Implementation Tasks
- [ ] Add tree-sitter dependency
- [ ] Create scrape command

## Key Decisions
- Use DuckDB for storage
- Tree-sitter for AST parsing

## Design
[High-level design description]

## Success Criteria
- Reduces tokens by 10x
- Query time < 100ms
```

## After Launch

1. Review `IMPLEMENTATION_PLAN.md`
2. Edit/refine the extracted TODOs
3. Start implementing
4. Update PR description as you progress
5. Check off tasks in the PR

## Related Commands

- `/session-git-start` - Begin a design/exploration session
- `/session-git-update` - Track progress
- `/session-git-end` - Complete session
- `/launch` - Bridge from design to implementation