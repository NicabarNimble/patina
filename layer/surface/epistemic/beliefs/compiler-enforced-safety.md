---
type: belief
id: compiler-enforced-safety
persona: architect
facets: [architecture, rust, agentic, safety]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-11
revised: 2026-02-11
---

# compiler-enforced-safety

In agentic execution contexts, the compiler is the only reliable review gate — type-level enforcement replaces organizational policy because the execution context lacks traditional human safeguards.

## Statement

In agentic execution contexts, the compiler is the only reliable review gate — type-level enforcement replaces organizational policy because the execution context lacks traditional human safeguards.

## Evidence

- [[session-20260211-100430]]: Traced three version bump paths — stringly-typed `bump_type: &str` allowed protocol violations no review caught. Typestate pattern prevents calling `release()` without `preflight()` at compile time. (weight: 0.9)
- [[session-20260211-100430]]: Compared two architectural advisory perspectives — organizational policy (process checklists, human review gates) vs type-level enforcement (enums, typestate, exhaustive match). In agentic context, organizational gates don't exist. The compiler is the only gate that reliably fires. (weight: 0.9)
- [[session-20260211-100430]]: `spec status complete` had zero safeguards — no clean tree check, no tag-exists check, no behind-remote check. Five safeguards existed in `version milestone` but nothing enforced calling them. A human might remember; an agent won't. (weight: 0.8)

## Reasoning

This belief is not a judgment on humans or AI agents. It's a property of the execution context:

1. **Traditional software teams** have layered review: PR review, team lead approval, CI gates, QA. Any of these can catch a protocol violation (e.g., releasing without safeguard checks).

2. **Agentic execution** removes most of these layers. An AI agent modifying code has one reliable gate: compilation. If bad code compiles, it ships.

3. **The implication**: safety mechanisms that rely on "the developer will remember to..." must be replaced with mechanisms the compiler enforces. Enums over strings (can't typo a variant). Typestate over documentation (can't skip a step). Exhaustive match over convention (can't forget a case).

This doesn't mean organizational policy is wrong — it means it operates at a different layer. When that layer is absent, the type system must compensate.

## Supports

- [[dependable-rust]] — small stable interfaces prevent misuse; this belief extends that to protocol enforcement
- [[spec-driven-design]] — specs as authority need deterministic enforcement, not just documentation
- [[dead-code-requires-decision]] — compiler warnings as decision forcing functions, same principle

## Attacks

<!-- No beliefs defeated yet -->

## Attacked-By

- Pragmatism: over-engineering types for simple cases adds complexity without proportional safety. A two-variant enum with a no-op arm is arguably an if-statement with extra steps.
  - Status: acknowledged — apply proportionally. The belief is strongest where protocol ordering matters (preflight→release), weakest where branching is trivial.

## Applied-In

- [[version-consolidation]] spec: `BumpType` enum replacing `&str`, typestate for preflight→release ordering, `ReleaseStrategy` enum for Cargo/NoOp dispatch

## Revision Log

- 2026-02-11: Created from version consolidation walkthrough — metrics computed by `patina scrape`
