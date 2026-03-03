---
type: feat
id: drift-detection
status: draft
created: 2026-03-03
sessions:
  origin: 20260303-101839
related:
- knowledge-system-architecture
- scrape-diff-driven
- structural-entropy-measure
beliefs:
- steenberg-lens-immutable-core
- transparent-complexity
- fix-architecture-not-documentation
- argue-every-box
exit_criteria:
- id: boundary-graph-validation
  text: '`patina measure` checks module dependency direction — domain modules cannot import from infrastructure modules (e.g., src/forge/ cannot import src/commands/)'
  checked: false
- id: public-api-surface-tracking
  text: '`patina measure` tracks public interface count per module and warns when it grows beyond a threshold'
  checked: false
- id: dependency-diff-check
  text: 'pre-push checks warn when new dependencies are added to Cargo.toml without a recorded justification in the commit message or spec'
  checked: false
- id: re-anchor-diagnostic
  text: '`patina context` includes active spec invariants and allowed change surface in its response, enabling LLM re-anchoring before coding'
  checked: false
- id: stop-the-line-triggers
  text: '`patina measure` flags structural violations as errors (not warnings) for: new public API, new module in core layers, boundary violations'
  checked: false
- id: drift-response-playbook
  text: 'documentation exists for drift response: stop, identify first violation, revert or isolate, reapply invariants'
  checked: false
---
# feat: Architectural Drift Detection and Prevention

> LLMs infer architecture from repo reality. One deviation becomes evidence
> of intent. Further changes align to drift. The further the divergence,
> the harder recovery becomes. Make drift detectable, expensive, reversible.

## Problem

Agentic development has no PR review, no team lead, no QA gate. The
compiler catches type errors, but not architectural drift — a new module
that duplicates an existing one, a dependency added without justification,
a public API that expands beyond its intended surface.

Per [[steenberg-lens-immutable-core]]: architecture should not require
periodic redesign. But without detection, drift accumulates silently until
redesign is the only option.

Per [[transparent-complexity]]: complexity you can't see will kill you.
Architectural drift is invisible complexity — it looks intentional because
it's in the codebase, so LLMs propagate it.

The compounding problem: each drift event teaches the next LLM session
that the drift is correct. Correction becomes exponentially harder.

## Solution

Three mechanisms: detect, prevent, respond.

**Detect:** Extend `patina measure` with structural checks that run on
every scrape. Boundary graph validation (which modules import which),
public API surface trending, dependency lockfile diffs.

**Prevent:** Stop-the-line triggers for high-risk structural changes.
Re-anchor loop in `patina context` that restates active spec invariants
so LLMs start from correct architecture, not inferred architecture.

**Respond:** Playbook for when drift is detected. Never normalize
deviation — rollback or realignment, not patch forward.

## Exit Criteria

See frontmatter.

## Non-Goals

- **Enforcing code style.** Formatting and linting are separate concerns.
  This spec addresses structural architecture, not surface aesthetics.
- **Blocking all changes.** Stop-the-line is for high-risk structural
  changes, not routine modifications. Per [[yegge-lens-spec-code-proportionality]],
  process overhead must be proportional.
- **Runtime enforcement.** This is build-time and scrape-time detection,
  not runtime checks.
