Manage spec lifecycle — query status, mutate state, guide workflow decisions.

This skill covers the full spec surface area. Use CLI (`patina spec <command>`)
via Bash as the primary interface. Fall back to MCP tools (spec.*) only if
CLI is unavailable.

## When to Use Each Operation

**QUERIES (read-only, safe to call anytime):**

- `spec.list` — Show all specs. Use when the user asks "what specs exist?"
  or you need to understand the current landscape.
  Optional filters: status, target.

- `spec.ready` — Show actionable specs. Use when the user asks "what can
  I work on?" Returns only ready/active specs with all blockers complete.

- `spec.blocked` — Show stuck specs. Use when the user asks "what's blocked?"
  Returns specs with incomplete dependencies and blocker details.

- `spec.next` — Recommend next spec. Use at session start, when the user
  finishes a task, or asks "what should I work on?" Returns ranked
  recommendations with reasoning.

**MUTATIONS (change state, confirm with user first):**

- `spec.create` — Create a new spec and fill it in. Triggers on any
  signal that work should be tracked: "this should be fixed", "we need
  to handle X", "let's clean this up", "I wonder why Y happens", "spec
  this", "that's a bug", "we should add", "let's plan out". You do NOT
  need the user to say "create a spec" — recognize the intent and offer.
  Infer type from context: bug/broken/wrong → fix, new capability/add →
  feat, cleanup/restructure → refactor, question/investigate → explore.
  Parameters: spec_type (required), id (required), title, description,
  blocked_by.

  **After scaffolding, always fill in the body:**
  1. Read the created SPEC.md (the CLI writes empty section headings)
  2. Write substantive content into every section from conversation
     context — Problem, Solution/Root Cause/Fix, Exit Criteria, etc.
  3. For exit criteria, write specific checkable items, not vague goals
  4. If you lack context for a section, write what you know and mark
     gaps with "TODO: clarify with user"
  5. If DESIGN.md exists (scaffolded for feat and refactor types),
     fill it in with: approach, planned commits, and key files before
     starting implementation. The design doc turns a spec from a
     contract into an execution plan — when filled in, commits land
     almost mechanically.
  6. Commit the populated spec
  The spec should be useful immediately — never leave a skeleton.

- `spec.promote` — Advance: draft -> ready -> active. Use when a spec
  is reviewed and ready to progress. Promoting to active creates a git tag.
  Parameters: id (required).

- `spec.pause` — Park active work. Use when the user says "let's stop
  this and work on something else" or discovers a blocking issue.
  Creates WIP commit if dirty, tags state for later resume.
  Enforces one-paused-spec rule — resolve existing pause first.
  Parameters: id (required), reason (required).

- `spec.resume` — Restore paused/blocked work. Use when returning to
  paused work or when a blocker completes. Shows context diffs.
  Parameters: id (required), force (optional, for blocked specs).

- `spec.block` — Mark dependency. Use when the user discovers "we need
  spec-X done before we can continue spec-Y."
  Parameters: id (required), by (required), reason (required).

- `spec.complete` — Ship it. Use when all exit criteria are met.
  Triggers version bump + archive + git tag.
  Parameters: id (required), major (optional, for 1.0.0 moments).

- `spec.abandon` — Kill it. Use when the user decides a spec is no
  longer worth pursuing. Archives without release.
  Parameters: id (required), reason (optional).

- `spec.split` — Ship done half, draft the rest. Use when some work
  is shippable but the spec isn't fully complete. Completes original,
  creates new draft with split_from provenance.
  Parameters: id (required), new_id (optional), description (optional).

## Judgment Guidance

**Proactive spec creation — don't wait for magic words:**
- When the user describes a problem, a feature need, a cleanup, or an
  investigation, offer to create a spec. Signals include:
  - Bug reports: "this is broken", "that's wrong", "it crashes when"
  - Feature ideas: "we should add", "it would be nice if", "we need"
  - Refactor needs: "this is messy", "let's clean up", "should restructure"
  - Investigations: "I wonder why", "we should look into", "not sure how"
  - Mid-work discoveries: "wait, this other thing is broken too"
- Suggest the type and a kebab-case id. Example: "That sounds like a
  fix spec — want me to create `fix/db-rollback-on-failure`?"
- When a bug is found during active spec work, offer to pause the
  current spec and create a fix spec for the discovered issue.

**Other proactive suggestions:**
- When a spec's exit criteria are all checked, suggest completing it.
- At session start, run spec.next to orient the conversation.
- When finishing work, check if any blocked specs are now unblocked.

**Constraint enforcement:**
- One paused spec at a time — if a pause exists, surface it before
  allowing another pause.
- Mutations require confirmation — always tell the user what will happen
  before calling a mutation tool.
- Reason is required for pause and block — ask the user if not provided.

## Parameter Filling from Conversation

- **id**: Infer from context. If discussing "spec-workflow-rigor", use that.
  If ambiguous, ask.
- **reason**: Quote the user's words when they explain why they're stopping
  or blocking. Don't fabricate reasons.
- **by** (for block): Infer the blocker spec from the conversation. If the
  user says "we need auth first", look for a spec matching that description.

## Presenting Results

- For `spec.list`: Summarize counts by status, highlight active and paused.
- For `spec.next`: Lead with the top recommendation and its reason.
  Mention alternatives briefly.
- For mutations: Confirm the state change, show the git tag created,
  and suggest the natural next action (e.g., after pause, suggest what
  to work on next).

## Example: Create Workflow

User says: "the complete command leaves the DB in a bad state when
git operations fail"

You recognize this as a bug → fix spec. Respond:

> That sounds like a fix spec. Want me to create
> `fix/spec-complete-atomicity`?

If user confirms:
1. Call `spec.create` with type=fix, id=spec-complete-atomicity,
   title="Spec complete atomicity gap",
   description="complete_spec_value updates DB before release, leaving
   inconsistent state on failure"
2. Read the created SPEC.md
3. Fill in Problem (what the user described + what you know from the
   codebase), Root Cause (trace it in code), Fix (proposed approach),
   Exit Criteria (specific testable items)
4. Commit: `git add ... && git commit -m "spec: flesh out spec-complete-atomicity"`
5. Tell the user what you wrote and ask if anything needs adjusting
