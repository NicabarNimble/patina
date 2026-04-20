---
type: refactor
id: interface-session-artifacts
status: active
created: 2026-04-16
sessions:
  origin: 20260416-080506-702140000
related:
- src/commands/ai/surface.rs
- src/commands/ai/internal.rs
- src/interface/internal/checkin.rs
- src/interface/internal/launcher.rs
- src/session/internal/live.rs
- src/session/internal/projection.rs
- src/session/internal/artifact.rs
- src/commands/session/internal.rs
- mother/src/state.rs
beliefs:
- '[[durability-lives-outside-interface-process]]'
- '[[session-system-needs-multi-interface-redesign]]'
exit_criteria:
- id: isa1-launch-join-contract
  text: '`patina ai <interface>` follows 0/1/many rule: none=start, one=attach, many=prompt in TTY and fail-closed in non-interactive mode.'
  checked: true
- id: isa2-no-new-flag-surface
  text: No new primary session-selection flags are introduced for this refactor; existing `--session` remains explicit override.
  checked: true
- id: isa3-session-contained-durable-object
  text: Session is defined as a contained durable object with stable identity, lifecycle, and artifact references.
  checked: true
- id: isa4-interface-owned-artifacts
  text: Each interface must produce and maintain its own durable session artifact representation.
  checked: true
- id: isa5-claude-flow-skills-intact
  text: Claude user flow and skill design remain intact; changes only ensure clean integration with session/tmux infrastructure.
  checked: true
- id: isa6-pi-artifact-path
  text: PI has first-class session artifact creation and lifecycle updates wired through the same session object contract.
  checked: true
- id: isa7-pointer-lane-determinism
  text: Pointer and transport lane resolution remain deterministic under multiple active sessions for one interface.
  checked: true
- id: isa8-mixed-seam-register
  text: Mixed-state seam register is maintained in this spec as migration checklist across sessions.
  checked: true
- id: isa9-claude-capture-profile
  text: Claude capture profile is explicitly defined as LLM-authored summarization (no canonical machine transcript), including start/update/end authoring requirements.
  checked: true
- id: isa10-pi-log-distill-profile
  text: PI capture profile is explicitly defined as machine-log distill + optional LLM enrichment, with deterministic mapping from PI JSONL logs to session artifact updates.
  checked: true
- id: isa11-claude-pi-artifact-parity
  text: Claude and PI produce near-identical session artifact structure and sections, differing only in evidence acquisition path.
  checked: true
- id: isa12-non-target-interface-boundary
  text: OpenCode and Gemini are explicitly out of active capture-migration scope for this slice.
  checked: true
- id: isa13-structured-yaml-frontmatter
  text: Claude and PI artifacts both use the canonical structured YAML frontmatter schema.
  checked: true
- id: isa14-tmux-session-state-parity
  text: Claude and PI both integrate with the same tmux session-state infrastructure (lane binding, lookup, teardown semantics).
  checked: true
- id: isa15-programmatic-ingest-frame-lock
  text: 'Session markdown frame is locked for programmatic ingestion: canonical YAML frontmatter keys and canonical section headings/order remain stable across Claude and PI.'
  checked: true
- id: isa16-one-shot-cutover
  text: Migration mechanics are short-lived for this implementation session only; no long-lived shadow/dual-write migration mode remains after cutover.
  checked: true
- id: isa17-spec-bound-session-contract
  text: Each session is explicitly bound to a work spec id and carries a stable continuity uid across restarts/takeovers.
  checked: true
- id: isa18-user-verified-successor-flow
  text: When a prior session dies, successor session creation requires explicit user verification and records lineage in structured metadata.
  checked: true
validated_against_commit: a4d55b81
last_freshness_check: 2026-04-16
freshness_scope:
- src/session/internal/artifact.rs
- src/session/internal/live.rs
- src/session/internal/projection.rs
- src/commands/session/internal.rs
- src/interface/internal/checkin.rs
- src/interface/internal/launcher.rs
- src/commands/ai/surface.rs
- src/commands/ai/internal.rs
---
# refactor: Interface Session Artifacts

> Sessions are durable contained objects. Interfaces own their session artifact capture. Keep Claude user flow and skill design intact, design PI capture ground-up for infrastructure alignment, and converge both to near-identical durable artifacts.

## Problem

Current behavior mixes three concerns:

1. lifecycle authority (Mother/session store),
2. durable project artifact (`layer/sessions/*.md`),
3. interface-specific runtime evidence/logs.

The result is operational ambiguity when multiple same-interface sessions exist and unclear ownership of interface-native artifacts.

## Goal

1. Define a session as a **contained durable object**.
2. Require **interface-owned durable artifacts** for each interface runtime.
3. Keep Claude user flow and skill design unchanged.
4. Design PI capture from the ground up to integrate with Patina session/tmux infrastructure.
5. Converge Claude and PI to near-identical session artifact structure.
6. Require canonical structured YAML frontmatter for both Claude and PI artifacts.
7. Bind each session to a work spec id and stable continuity uid.
8. Support user-verified successor takeover when prior session dies.
9. Preserve deterministic attach/start resolution without adding flag sprawl.
10. Keep OpenCode/Gemini outside active migration scope for this slice.

## Status

Draft complete and implementation-ready for same-session cutover work. Promotion is blocked only by structural readiness gates and not by unresolved direction.

## Solution

Implement a one-shot cutover that:

1. keeps Claude workflow/skills unchanged,
2. adds PI log-distill capture in core,
3. locks canonical machine-ingestable markdown frame,
4. binds sessions to `work_spec` + `continuity_uid`,
5. adds user-verified successor takeover when a prior session dies,
6. aligns Claude/PI tmux session-state behavior.

## Resolved Decisions

- Claude remains summary-first while working; no UX/skill redesign.
- PI is machine-distill-first from durable JSONL logs.
- Artifact frame is locked for programmatic ingestion; content may differ by interface.
- OpenCode is deferred and Gemini is paused for this slice.
- Migration mechanics are short-lived and removed after same-session cutover verification.
- Sessions are spec-bound (`work_spec`) with stable continuity (`continuity_uid`) and explicit successor verification.

## Non-Goals

- Introducing a new session-selector flag surface.
- Requiring jj/Rivet for baseline session operation.
- Redesigning Claude command UX or skill prompts.
- Implementing OpenCode/Gemini capture-pipeline migrations in this phase.

## Session Object Contract

A session object contains:

- stable runtime identity (`runtime_id`, `file_id`),
- stable continuity identity (`continuity_uid`) that survives takeover/restart,
- explicit work binding (`work_spec`) to the active spec id,
- lifecycle status (`active`, `archived`),
- deterministic durable project record (`layer/sessions/{file_id}.md`),
- interface ownership (`interface_name`, `interface_kind`),
- references to interface-native artifacts,
- git/session boundary metadata (start/end tags, branch, commits),
- optional successor/takeover lineage metadata.

This keeps the project-level session object durable and auditable while allowing interfaces to manage their own capture formats.

## Spec-Bound Continuity & Successor Contract

Rules:

1. Every session must declare `work_spec` (spec id) and `continuity_uid` in frontmatter.
2. `continuity_uid` remains stable across runtime restarts and successor sessions.
3. If a prior session is considered dead/lost, creating a successor requires explicit user verification in interactive mode.
4. Non-interactive mode fails closed for successor takeover unless an explicit prior runtime/session selector is provided.
5. Successor session must record lineage (`handoff_from` and/or explicit takeover metadata) and note that user verified takeover.

This provides continuity even when transport dies while preserving operator intent and auditability.

## Interface Artifact Contract

Each interface must maintain its own durable artifact per session runtime.

Examples of interface-owned artifact forms (format can differ):

- markdown,
- json/jsonl,
- interface-specific logs projected into a stable session artifact path.

Rules:

1. Interface artifact path must be deterministic from session identity.
2. Interface artifact writes must occur on start/update/end lifecycle transitions.
3. Session object must reference interface artifact location(s).
4. Interface artifact failure handling is explicit (fail-closed vs degraded mode policy documented per interface).

## Launch/Attach Behavior Contract (No New Flags)

For `patina ai <interface>`:

- **0** matching active sessions: start new session.
- **1** matching active session: attach to existing session.
- **2+** matching active sessions:
  - interactive TTY: prompt user to choose existing session or start new,
  - non-interactive: fail closed with actionable choices.

Dead/lost-session handoff:

- if selected/pointed active session is no longer live at transport/runtime boundary, prompt user to verify successor takeover,
- on confirmation, start successor session with same `work_spec` + `continuity_uid`, and record lineage,
- without confirmation (or in ambiguous non-interactive mode), fail closed.

`--session <id>` remains the explicit override path.

## Compatibility Guardrail

Claude flow must remain fully intact:

- same command shape,
- same `/session-*` skill workflow,
- same expected start/attach semantics,
- no required new flags,
- no surprise transport regressions.

Any changes in this spec for Claude are infrastructure integration only (session object + tmux state alignment), not UX/prompt redesign.

## PI Artifact Requirement

PI capture is designed ground-up for this infrastructure contract:

- session start initializes PI artifact representation and binds log correlation metadata,
- update/note/end persist PI artifact state from log-distilled facts,
- session object references PI artifact path and source-log span identifiers,
- archived PI session remains discoverable via session object metadata,
- PI follows the same tmux session-state lifecycle semantics as Claude.

## Interface Capture Profiles (v1)

This section defines how each interface captures and writes session artifacts.

| Interface | Primary Evidence Source | Capture Mode | Status in this spec |
|---|---|---|---|
| Claude | live conversation context + tool outputs in current turn | **LLM-authored summary-first (while working)** | active + locked UX/skills |
| PI | persisted JSONL conversation/event logs (`~/.pi/agent/sessions/...jsonl`) + bridge entries | **machine-distill-first** + optional LLM enrichment | active + new design |
| OpenCode | existing wrapper-driven summary flow | unchanged | deferred |
| Gemini | existing wrapper-driven summary flow | unchanged | paused |

### Claude profile (locked)

Claude has no canonical machine transcript artifact that Patina can safely treat as deterministic session truth. Therefore Claude requires active agent summarization **while working**.

Required behavior:

1. **Start**: create/load session object, read previous session artifact, write substantive `Previous Session Context` summary.
2. **Update**: append structured summary of completed work, decisions, challenges, and evidence links.
3. **Note**: append user/operator note with git context when available.
4. **End**: perform final update before archive; ensure outcome reflects actual work completed.

Lock rule: keep current Claude command UX and skill prompt design intact.

### PI profile (ground-up log-distill path)

PI can summarize from durable logs directly, so PI follows machine-first distillation.

Required behavior:

1. **Start**: bind session object to PI runtime/session log identity.
2. **Update**: distill new JSONL span into structured summary (messages, tool calls, failures, decisions), then optionally enrich with LLM narrative.
3. **Note**: append operator note and correlate with nearest log timestamp/span.
4. **End**: distill final span, persist archive summary, and keep source-log reference in session metadata.

Guardrail: machine-distilled facts are source of truth; enrichment cannot contradict extracted log facts.

### Claude ↔ PI artifact parity contract

Claude and PI must converge on near-identical durable artifact output.

Important distinction:

- **content/context may differ** (Claude summary-first vs PI log-distill),
- **frame must be locked** (same machine-parseable markdown structure).

Required canonical YAML frontmatter fields for both:

- `type`, `id`, `runtime_id`, `continuity_uid`, `work_spec`, `title`, `status`,
- `llm`, `interface`, `created`, `updated`, `start_timestamp`,
- optional `voice`, `participants`, `interfaces`,
- optional handoff lineage (`parent_session`, `handoff_from`, `handoff_to`),
- optional takeover metadata (`takeover_from_runtime`, `takeover_user_verified`),
- nested `git` object (`project_uid`, `branch`, `starting_commit`, `start_tag`, optional `end_tag`).

Frontmatter lock rules:

1. YAML block must use standard `---` delimiters.
2. Key names are stable (no interface-specific rename drift).
3. Optional fields may be omitted, but required keys must always be present.

Body-section parity target for both (exact headings, same order):

1. `## Previous Session Context`
2. `## Goals`
3. `## Activity Log`
4. `## Decisions`
5. `## Evidence`
6. `## Handoff`
7. `## Outcome`

Body lock rules:

- headings above are canonical ingestion anchors,
- interfaces may differ in prose/detail under each section,
- additional subsections are allowed only under canonical parent sections.

### OpenCode/Gemini boundary

OpenCode and Gemini are not active migration targets in this slice.

## tmux Session State Infrastructure Contract

Claude and PI both use the same tmux/session-state infrastructure contract:

1. deterministic lane identity derivation from project + interface + resolved session target,
2. shared attach/reattach semantics via check-in resolution,
3. shared teardown behavior on session archive/end,
4. shared stale-pointer/lane reconciliation policy.

This keeps transport behavior consistent even when evidence capture differs (summary-first vs log-distill).

## Mixed-State Edge Register (Canonical)

| Edge | Current State | Risk | Target State |
|---|---|---|---|
| E1: launch ambiguity | multiple same-interface active sessions currently error without interactive choice | friction + operator confusion | interactive picker (TTY) + fail-closed non-TTY |
| E2: artifact ownership blur | project artifact exists, interface-native artifact ownership not explicit | inconsistent capture expectations | interface-owned artifact contract per runtime |
| E3: pointer cardinality | interface pointer model is singleton-biased | attach ambiguity under parallel sessions | deterministic pointer + chooser flow |
| E4: transport naming | tmux lane naming is interface/project scoped | session lane collision potential | lane resolution keyed deterministically by session identity |
| E5: surface split | `ai launch` and `ai session start` semantics have drift risk | operator surprise | shared attach/start policy contract |
| E6: PI gap | PI lacks explicit artifact-first contract parity | uneven interface durability | PI artifact lifecycle implemented |
| E7: local WIP drift (2026-04-16) | uncommitted behavior changes in `src/commands/ai/internal.rs` and `src/main.rs` | accidental merge of unreviewed behavior | reconcile explicitly against this spec before shipping |
| E8: Claude transcript gap | no canonical machine transcript artifact is available for deterministic distillation | low-signal or generic updates if agent does not actively summarize | enforce Claude summary-first authoring contract |
| E9: PI log correlation seam | PI logs exist but mapping to Patina session runtime/file ids is not yet standardized | summaries could drift from true runtime span | define deterministic PI log↔session correlation keys and span windows |
| E10: Claude↔PI artifact parity gap | Claude and PI currently differ in how evidence is captured and reflected in artifact content | drift in durable artifact shape and fields | enforce canonical frontmatter + section parity contract |
| E11: tmux infra divergence risk | capture-profile work could drift transport/session-state behavior per interface | inconsistent attach/teardown semantics | shared tmux state contract across Claude and PI |
| E12: non-target interface scope creep | OpenCode/Gemini may get pulled into active migration accidentally | delivery risk and churn | keep OpenCode deferred and Gemini paused for this slice |
| E13: ingest frame drift | interfaces may diverge on heading names/order or frontmatter keys over time | DB/index parsers become brittle | lock canonical frame + add parity tests |
| E14: end-append section drift | end flow currently appends extra top-level sections (`Beliefs Captured`, `Session Classification`, `User Prompts`) | canonical section-order parser instability | move computed summaries under canonical section anchors or structured metadata |
| E15: PI distill missing in core | PI wrappers/prompts currently mirror summary-first shape | PI cannot yet produce machine-distilled authoritative updates | implement PI JSONL span distill pipeline in core session update/end path |
| E16: spec binding absent | sessions are not yet hard-bound to a spec id in frontmatter | weak traceability from work log to spec intent | require `work_spec` on session creation/continuity |
| E17: successor verification gap | dead-session replacement can be implicit/manual without explicit verification metadata | accidental takeover or ambiguous continuity | require user-verified successor protocol + metadata |

## Technical Reality Check (2026-04-16)

Current implementation status against this spec:

- **In place**
  - canonical durable artifact path (`layer/sessions/{id}.md`),
  - typed YAML frontmatter model in core session artifact code,
  - Mother-backed session lifecycle and active-session listing,
  - Claude wrapper/skill flow already operational.
- **Missing/partial**
  - PI log-distill pipeline is not implemented in core,
  - canonical frame lock (strict heading/order + required-key validator) is not yet enforced,
  - many-session interactive chooser is not yet implemented (current behavior is fail-closed error),
  - tmux lane naming is interface-scoped, not session-target scoped,
  - session metadata does not yet carry standardized source-log span references for PI,
  - session frontmatter does not yet require `work_spec` + `continuity_uid`,
  - no explicit user-verified successor metadata path for dead-session takeover.

This reality check is normative for planning: implementation must close these gaps in the order below.

## Migration Safety Protocol (short-lived, current session only)

This protocol is temporary and applies only while wiring this refactor today. It is **not** a long-lived migration track.

1. **Frame freeze**
   - keep canonical frontmatter keys and canonical heading order unchanged.
2. **Read-compat before writer changes**
   - update parsers/validators first; change artifact writer only after compatibility tests pass.
3. **Temporary PI shadow verification (optional but preferred)**
   - validate PI distill output against canonical frame before canonical artifact writes.
4. **Crash-safe artifact writes**
   - use temp-write + atomic replace semantics for session markdown updates.
5. **Frequent checkpointing**
   - require periodic `session update` and small commits when touching parser/writer paths.
6. **Fail-closed mutation policy**
   - if canonical frame validation fails, abort mutation and preserve last-known-good artifact.
7. **Recovery path clarity**
   - PI recovery can replay from JSONL span; Claude recovery uses durable artifact + update/note history.
8. **Same-session cleanup requirement**
   - remove/disable temporary migration scaffolding (shadow outputs, temporary toggles) once cutover is verified.

## Implementation Order (locked)

1. **Frame lock first**
   - enforce canonical frontmatter/heading contract + parser tests.
2. **Spec/continuity identity second**
   - require `work_spec` + `continuity_uid` and successor-lineage metadata in session object.
3. **PI distill third**
   - implement JSONL span correlation and distill writer path for update/end.
4. **Canonical-section discipline fourth**
   - relocate appended computed summaries under canonical anchors (no top-level drift).
5. **Selection + successor UX fifth**
   - add interactive chooser for many-session case in TTY,
   - add user-verified successor takeover flow when prior session died,
   - keep non-TTY fail-closed.
6. **tmux/session-target hardening sixth**
   - align lane identity and teardown semantics with session-target resolution.
7. **same-session cleanup seventh**
   - remove temporary migration scaffolding so no lasting migration mode remains.

## Implementation Phases

1. **Phase A — Contract lock**
   - finalize session object + interface artifact contract text
   - lock 0/1/many launch behavior
   - lock required `work_spec` + `continuity_uid`
2. **Phase B — Resolver/transport hardening**
   - deterministic attach resolution, interactive choice path, lane/pointer consistency
   - define dead-session detection boundary for successor takeover
3. **Phase C — Interface artifact wiring**
   - lock Claude profile integration (summary-first while working) with no UX/skill changes
   - implement PI machine-distill artifact lifecycle from the ground up
   - enforce Claude↔PI artifact frontmatter/body parity
   - keep OpenCode deferred and Gemini paused
4. **Phase D — Canonical body discipline + chooser UX**
   - remove top-level section drift,
   - implement TTY chooser for many-session ambiguity,
   - implement user-verified successor takeover flow
5. **Phase E — tmux/session-target hardening + cleanup**
   - lane identity/teardown alignment,
   - remove temporary migration scaffolding,
   - tests, docs, and local WIP reconciliation

### Phase Progress (2026-04-16)

- **Completed:** Frame lock validators + canonical heading discipline in session writes.
- **Completed:** `work_spec`/`continuity_uid` frontmatter wiring and default hinting from active spec.
- **Completed:** interactive many-session chooser + user-verified successor takeover path.
- **Completed:** session-target tmux lane naming with legacy-lane teardown fallback.
- **In progress:** PI distill lifecycle parity (start/end metadata correlation still to harden).

## Build Readiness

High for immediate implementation in this session:

- direction is resolved for Claude/PI,
- core seams are identified with concrete file targets,
- acceptance criteria and verification checks are explicit,
- rollout is one-shot with temporary safeguards removed after cutover.

## Verification (planned)

```bash
cargo check --workspace -q
cargo test --workspace -q session
cargo test --workspace -q ai
cargo test --workspace -q checkin
```

Scenarios:

- `patina ai <interface>` none/one/many matching sessions
- non-interactive ambiguity fails closed
- Claude summary-first quality path (start/update/end produce substantive sections while working)
- PI log-distill path (JSONL span -> artifact update) on start/update/end
- Claude↔PI frontmatter parity check (canonical YAML fields present)
- Claude↔PI body-section parity check (same core section layout)
- shared tmux session-state behavior check for Claude and PI
- programmatic-ingest checks: frontmatter keys + canonical heading order parse cleanly
- required frontmatter identity checks: `work_spec` + `continuity_uid`
- user-verified successor takeover flow check when prior session is dead
- no non-canonical top-level section headings added during end/update flows
- no lingering shadow/temporary migration mode enabled after cutover
- OpenCode/Gemini unchanged boundary check
- deterministic resolution with explicit `--session`

## Notes for Future Sessions

Use this spec as migration checklist source of truth. Update by:

1. edge register row progress,
2. phase notes,
3. exit criteria checks.
