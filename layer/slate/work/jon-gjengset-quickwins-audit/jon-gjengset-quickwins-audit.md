# Jon Gjengset-Lens Quickwins Audit

Slate: `jon-gjengset-quickwins-audit`

This is a reasoning artifact, not a code-change request. The goal is to look at the merged watcher/Mother PR through a pragmatic systems-review lens and identify small cleanup moves before the next deeper child-storage work.

## Reviewer Lens

Assume the reviewer values:

- small, reviewable PRs;
- explicit type/API boundaries over stringly configuration;
- hermetic tests over local operational proof;
- boring failure modes and fail-closed install behavior;
- narrow core responsibilities;
- process artifacts that clarify decisions without replacing code clarity.

## What Worked

### Externalization sequence was disciplined

The watcher move followed a strong proof order:

1. define contract;
2. build externally;
3. publish GitHub release assets;
4. install through Mother registry;
5. smoke-test installed artifacts;
6. remove monorepo source.

This is the strongest part of the work. It avoided deleting in-tree code before a replacement existed.

Classification: accepted / preserve pattern.

### Bundle vs child separation stayed clean

The work avoided creating a runtime “bundled child.” Instead:

- children remain independently installable units;
- bundle is a future install/package UX concept;
- richer bundle installer work is parked in `mother-child-bundle-install-model`.

Classification: accepted / preserve pattern.

### Hash-verified release install stayed fail-closed

The release proof kept Slate-style assets:

- `.wasm`;
- `.wasm.sha256`;
- `child.toml`;
- `child.toml.sha256`;
- `checksums.txt`.

Mother registry install remained per-child and hash-verified.

Classification: accepted / preserve pattern.

## Concerns and Quick Wins

### 1. PR size was too large

The merged PR combined several separate themes:

- HITL skill lifecycle;
- handshake v2;
- repo-list discoverability;
- watcher extraction;
- GitHub release selectors;
- Slate archival cleanup;
- pre-push clippy cleanup.

A reviewer can approve the direction but still struggle to review the whole shape.

Classification: immediate process quick win.

Recommended next action:

- Keep the next child cleanup PR intentionally narrow.
- Target one visible concern only, such as child install-root inventory/reporting or config-contract design.
- Avoid mixing future bundle-installer design into storage cleanup.

### 2. GitHub release selectors are useful but stringly

Current selector surface:

```text
--tag-prefix
--asset-name-wasm
--asset-name-manifest
--asset-name-checksums
--patina-min
```

This is acceptable as a bridge, but it is still mostly strings. Longer-term, Mother probably wants a typed release selector model or bundle/source manifest.

Classification: next cleanup/design Slate, not immediate blocker.

Recommended follow-up:

- Keep current selectors as the minimal v1.
- In a future Slate, define a typed `GitHubReleaseSelector` / source manifest shape with validation rules and better error messages.
- Do not expand this inside `children-storage-cleanup` unless needed for storage migration.

Likely Slate:

- `mother-child-bundle-install-model` for bundle-level manifests;
- or a smaller `mother-github-release-selector-contract` if selector hardening is needed first.

### 3. External release/install proof is operational, not hermetic enough

The watcher proof used real GitHub releases and local Mother state. That is valuable evidence, but a reviewer may ask for a hermetic test that does not depend on this machine or live GitHub.

Classification: next cleanup quick win.

Recommended next action:

- Add a fixture/mocked-provider test for two releases from one source repo:
  - tag-prefix filtering;
  - exact asset-name selection;
  - checksum extraction;
  - two independent child entries.
- This could be a focused test-only PR.

Likely Slate:

- `mother-github-selector-fixture-tests` or fold into `child-sdk-conformance-suite` only if the scope is registry conformance.

### 4. Dual install roots remain a design smell

Watcher install proof had two roots:

```text
~/.patina/children
~/.patina/plugins
```

Mother registry install writes to `~/.patina/children`; `patina child call` still uses command-child/plugin compat root under `~/.patina/plugins`. The smoke proof had to bridge them manually.

Classification: core quick win for `children-storage-cleanup`.

Recommended next action:

- Make this the next concrete cleanup target.
- Start with read-only inventory/reporting before migration:
  - which child names exist in each root;
  - whether wasm/manifest hashes match;
  - which root is authoritative for each command surface;
  - which installs are duplicates, stale, or compat-only.

Likely Slate:

- continue `children-storage-cleanup` with a durable inventory artifact and/or a doctor-style read-only command.

### 5. Mother is absorbing more orchestration responsibility

Mother now owns or is growing toward:

- child registry;
- child install/assignment;
- HITL skill projection;
- interface handshake;
- source graph/routing;
- future bundle installer;
- future session artifact normalization.

This may be the right direction, but Mother can become the next monolith unless internal seams stay typed and narrow.

Classification: deeper design concern.

Recommended next action:

- Keep the three-system model explicit:
  - `patina-ai` = belief/context/local knowledge;
  - `patina-mother` = MCT runtime/control plane;
  - `patina-sdk` = child development system.
- For each new Mother capability, ask whether it is:
  - host-safety infrastructure;
  - child/package lifecycle infrastructure;
  - or domain behavior that should live in a child.

Likely Slate:

- `mother-control-plane-surface-boundaries` if the seam becomes unclear.

### 6. Allium/Slate helped but did not fully constrain scope

Slate/Allium forced several useful decisions:

- watcher as external child-bundle repo;
- per-child release tags;
- future bundle installer deferred;
- no monorepo removal until external proof.

But the merged PR still became very large. The process helped with correctness, less with PR slicing.

Classification: process quick win.

Recommended next action:

- Before starting a new Slate, define whether it is:
  - design-only;
  - test-only;
  - implementation-only;
  - or removal-only.
- Prefer one of those per PR.

## Suggested Next Child Cleanup Slice

Best next step for `children-storage-cleanup`:

> Build a read-only child install inventory/report that reconciles `~/.patina/children`, `~/.patina/plugins`, repo children, and external child sources.

Why this first:

- It addresses the largest remaining smell from watcher extraction.
- It is safer than migration.
- It gives humans and agents shared evidence before changing install paths.
- It can expose duplicate/stale/compat-only children without moving files.

Possible proof criteria:

- report lists Mother runtime installs under `~/.patina/children`;
- report lists command-child compat installs under `~/.patina/plugins`;
- report identifies matching child names across roots;
- report compares manifest and wasm SHA-256 when both roots have the same child;
- report marks source-of-truth hints: registry-installed, command-installed, repo-local, external-known;
- report is read-only and does not mutate local child stores.

## Proposed Classification Table

| Concern | Classification | Suggested Destination |
| --- | --- | --- |
| PR too large | immediate process quick win | next PR discipline |
| Stringly GitHub selectors | follow-up design/test | selector contract or bundle model Slate |
| Live GitHub/local proof | quick win test gap | mocked provider/fixture tests |
| Dual install roots | next concrete cleanup | `children-storage-cleanup` |
| Mother monolith risk | deeper architecture | future boundary Slate |
| Allium/Slate scope control | process quick win | use narrower Slate/PR slices |

## Recommendation

Do not start with migration. Start with inventory.

Next proposed work inside `children-storage-cleanup`:

```text
child-install-inventory-report
```

Either as a sub-Slate or as the first implementation slice of `children-storage-cleanup`, depending on how much code is required.
