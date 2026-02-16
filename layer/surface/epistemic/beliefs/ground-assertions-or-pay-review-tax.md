---
type: belief
id: ground-assertions-or-pay-review-tax
persona: architect
facets: [specs, governance, verification, process, review]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-15
revised: 2026-02-15
---

# ground-assertions-or-pay-review-tax

Every testable claim in a spec — verification commands, invariant assertions, execution prerequisites — must be grounded inline with evidence at the point of assertion. Ungrounded assertions are hypotheses masquerading as contracts; each one costs 2-3 review cycles to stabilize because reviewers must discover hidden assumptions one at a time.

## Statement

Every testable claim in a spec — verification commands, invariant assertions, execution prerequisites — must be grounded inline with evidence at the point of assertion. Ungrounded assertions are hypotheses masquerading as contracts; each one costs 2-3 review cycles to stabilize because reviewers must discover hidden assumptions one at a time.

## Evidence

- [[session-20260215-214046]]: workspace-cleanup spec went through 10+ review rounds across a single session; five categories of ungrounded assertions each created compounding review tax — untested commands (3 rewrites for cargo publish), unjustified invariants (build.rs questioned), undiscovered false positives (rg gate iterated 3 times), unscoped doc touchpoints (surfaced late per-phase), unstated execution context (pre/post-commit confusion on every gate) (weight: 0.95)
- [[session-20260215-214046]]: Cost is non-linear. One ungrounded command (`cargo publish --dry-run`) didn't cost 1 round — it cost 3, because each fix introduced a new unstated assumption (clean tree → credentials → timing). Five ungrounded categories × 2-3 rounds each = 10+ rounds for a spec whose content was mostly correct from round 1. (weight: 0.9)
- [[spec-needs-code-verification]]: Prior belief established that specs miss ~30% of affected paths without code reading. This belief extends the principle: even after reading code, verification commands and invariant claims must be TESTED, not just stated. Reading proves "what exists"; running proves "what the command does." (weight: 0.8)

## Supports

- [[spec-driven-design]] — specs are contracts; ungrounded assertions are broken contracts
- [[spec-needs-code-verification]] — extends code-reading to command-testing
- [[argue-every-box]] — grounding IS arguing both sides (what works AND what breaks)

## Attacks

- Spec velocity — "grounding slows down spec creation." Counter: one sentence of justification per assertion vs. 2-3 review cycles per ungrounded assertion. Net cost is negative.
- "The reviewer will catch it" — this belief says the reviewer catching it IS the problem. Each catch is a round-trip. The author should catch it first.

## Attacked-By

- "Specs are living documents — just fix them in review." Counter: review loops compound. 5 ungrounded assertions × 3 rounds each = 15 cycles. 5 grounded assertions × 0 rounds = 0 cycles + 5 sentences of upfront work.
- Exploration specs where the point IS to discover unknowns. Grounding applies to assertions, not questions. An explore spec that says "we don't know if X works" is grounded by admission.

## Applied-In

- [[workspace-cleanup]] spec: 10+ rounds of review churn traced to 5 categories of ungrounded assertions. After grounding (inline justifications, targeted verification, explicit timing), the spec stabilized.

## The Three Grounding Forms

**1. Verification commands** — run the command, document what it returns:
```
Bad:  "Run `rg 'patina-metal'` — should return zero"
Good: "Run `rg 'patina-metal' src/ grammars/ plugins/` post-commit.
       Targets only actionable locations. Should return zero."
```

**2. Invariants ("doesn't change")** — state WHY in a parenthetical:
```
Bad:  "Grammar build.rs files — do not change"
Good: "Grammar build.rs files — paths are crate-relative; cargo runs
       build.rs from crate root, inner grammars/ dir moves with crate"
```

**3. Prerequisites** — state context BEFORE the command:
```
Bad:  "cargo package -p patina-sdk"
Good: "cargo package -p patina-sdk (post-commit; validates manifest
       and include/exclude globs, no registry credentials needed)"
```

## Implementation Direction

This belief should inform three layers:

1. **Core pattern**: Add Rule 7 "Ground Every Assertion" to [[spec-driven-design]] — makes grounding a governance requirement, not just advice
2. **Spec creation**: AI agents should run verification commands during spec creation and document output inline — the spec equivalent of TDD
3. **Tooling (future)**: `patina spec ground` — parse a spec for verification code blocks, run them, report untested assertions

## Revision Log

- 2026-02-15: Created — metrics computed by `patina scrape`
