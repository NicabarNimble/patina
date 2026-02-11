---
type: belief
id: transparent-complexity
persona: architect
facets: [architecture, rust, agentic, design-philosophy]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-11
revised: 2026-02-11
---

# transparent-complexity

Complexity you can't see will kill you — every code path, ownership boundary, and control flow must be visible to the compiler, not hidden behind abstractions that only humans can reason about.

## Statement

Complexity you can't see will kill you — every code path, ownership boundary, and control flow must be visible to the compiler, not hidden behind abstractions that only humans can reason about.

## Reasoning

Two architectural philosophies share the same root insight — hidden complexity kills systems — but apply it through different mechanisms:

**Organizational transparency (Jerry Nixon):** "Simple is the best architecture. Your job is to say NO." Argue every box out of your system design. Make decisions visible through process, review, and team structure. The org chart is the enforcement layer.

**Compiler transparency (Andrew Kelley / Zig):** "No hidden control flow." Argue every feature out of your language. Make every code path, ownership transfer, and side effect visible to the compiler. The type system is the enforcement layer.

Both are correct — at their respective layers. But in agentic development, the organizational layer is absent. No PR review, no team lead, no QA gate. The compiler is the only reviewer that reliably fires. Therefore:

- **Enum over trait** — dispatch is visible, exhaustive match forces handling every case
- **Typestate over documentation** — protocol ordering is compiler-enforced, not human-remembered
- **Typed enums over strings** — domain values are closed sets, not open-ended text
- **Explicit ownership over smart defaults** — who owns what is visible in the type signature

This doesn't mean traits and abstractions are wrong. It means the *default* should be the transparent option. Extract to abstraction only when the concrete need arrives (a third variant, a plugin boundary, a WIT interface).

## Evidence

- [[session-20260211-100430]]: Traced three version bump paths — `bump_type: &str` allowed protocol violations invisible to the compiler. `BumpType` enum and `PreparedRelease` typestate make both the domain and the protocol compiler-visible. (weight: 0.9)
- [[session-20260211-100430]]: Historical analysis of 14 patch releases — all were spec-worthy work. The `version patch` escape hatch hid spec-bypass behind a routine-sounding command name. Renaming to `version hotfix` makes the escape visible and intentional. (weight: 0.7)
- [[session-20260211-100430]]: `spec status complete` had zero safeguards while `version milestone` had five — but nothing enforced calling the safeguarded path. The protocol was documented, not enforced. Typestate (`preflight()` → `PreparedRelease` → `execute()`) makes the compiler enforce it. (weight: 0.9)

## Supports

- [[compiler-enforced-safety]] — this belief provides the philosophical foundation; compiler-enforced-safety provides the specific mechanism
- [[dependable-rust]] — small stable interfaces are inherently transparent; internal complexity is hidden behind a visible contract
- [[spec-driven-design]] — specs make decisions visible at the process layer; this belief extends visibility to the code layer
- [[dead-code-requires-decision]] — dead code hides complexity from the compiler; removing it restores transparency

## Attacks

<!-- No beliefs defeated yet -->

## Attacked-By

- Ergonomics: transparent code can be verbose. Zig's explicit error handling is more visible but more lines than Rust's `?` operator. The trade-off is real — but in agentic contexts, verbosity is cheaper than invisibility.
  - Status: acknowledged — apply proportionally. Hot paths that are modified often benefit most from transparency. Stable utilities can afford abstraction.

## Applied-In

- [[version-consolidation]] spec: `ReleaseStrategy` enum (visible dispatch) over `ReleaseHook` trait (hidden dispatch). `BumpType` enum over `&str`. `PreparedRelease` typestate over "remember to call preflight first."
- Mother children: `MotherChild` trait is justified — third-party plugins need dynamic dispatch. The trait boundary is the plugin boundary, not hidden complexity.

## Revision Log

- 2026-02-11: Created from advisory comparison (Nixon vs Kelley) during version consolidation design
