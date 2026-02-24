Manage spec lifecycle — query status, mutate state, guide workflow decisions.

This skill covers the full spec surface area. Use MCP tools (spec.*) for
structured operations. Fall back to CLI (`patina spec <command>`) when MCP
is unavailable or when the user requests human-readable output.

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

**Proactive suggestions:**
- When the user identifies a bug during spec work, offer to pause the
  current spec and work on a fix.
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
