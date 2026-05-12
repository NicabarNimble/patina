# Patina MCT Positioning Audit

Slate work: [[patina-mct-positioning-audit]]

Goal: align `README.md` and future marketing material around what Patina actually is today, with special focus on Mother / Child / Toy architecture. This is an evidence-first audit: every useful message should map back to repo truth, not hype.

## 1. Current README Message

`README.md` currently positions Patina as:

> Context management for AI-assisted development.

The strongest existing README claims are:

- Patina is a local-first Rust CLI that turns a repository into reusable knowledge for humans and AI tools.
- Patina scrapes code, git history, layer artifacts, and external sources into local stores and indices.
- `scry`, `assay`, and `context` provide semantic retrieval, structural/factual retrieval, and AI-facing guidance.
- Mother adds child orchestration, cross-project queries, secrets management, and session coordination.
- Children are WASM components built on the WASI component model with Patina toy interfaces for sandboxed capability access.
- `layer/` is git-tracked project memory and is explicitly called “THE PRODUCT”.

This is truthful, but it undersells the new product center: Patina is no longer only “context management”. The repo now tells a richer story: **Patina is a local-first knowledge protocol plus an MCT control plane for trustworthy AI development systems.**

## 2. Repo Truth / Evidence Matrix

| Claim | Evidence | Source |
|---|---|---|
| Patina is a knowledge protocol, not just a search CLI. | Core identity says Patina has five protocol verbs: capture, index, search, believe, evolve. | `layer/core/patina-identity.md` |
| The durable product is the project knowledge layer. | Core identity says “The binary is the pipeline. The layer is the product. The protocol is the contract.” README layout labels `layer/` as “THE PRODUCT”. | `layer/core/patina-identity.md`, `README.md` |
| Patina is local-first. | Core invariants say all knowledge lives on disk; README shows `~/.patina/mother/projects/{uid}/` databases and local `.patina/` caches. | `layer/core/patina-identity.md`, `README.md` |
| Patina has a real knowledge pipeline. | README documents `scrape`, `oxidize`, `scry`, `assay`, `context`, `belief`, `report`, and `measure`. | `README.md`, `src/main.rs` |
| Mother is the control plane / router. | Core identity says Mother is cross-project daemon, routing, plugin management, and “Mother IS how Patina scales.” | `layer/core/patina-identity.md` |
| Mother has explicit lifecycle behavior. | Allium lifecycle spec covers start/stop/restart/status/install/uninstall, supervisor behavior, readiness, PID state, and startup profiles. | `layer/allium/mother/mother-lifecycle.allium` |
| Mother routes source and graph knowledge. | Source graph Allium covers source definitions, runs, graph nodes/edges, belief graph, sync state, search/query behavior. | `layer/allium/mother/mother-source-graph-routing.allium` |
| Children are modular execution units. | `children/README.md` says each child is a standalone crate with `Cargo.toml` and `child.toml`; current repo has many first-party child manifests. | `children/README.md`, `children/*/child.toml` |
| Children use typed contracts and can fail closed. | `mother/src/runtime.rs` default typed `call` bails if not implemented. Slate manifest is `wit-only` and declares operation allowlist. | `mother/src/runtime.rs`, `children/slate-manager/child.toml` |
| Toys are explicit capabilities. | `child.toml` manifests declare `[needs].toys`; contract-first value says toys are explicit capabilities and one must point to toy scope for a flow. | `children/*/child.toml`, `layer/core/values/contract-first-execution.md` |
| MCT is a safety boundary, not branding only. | Belief [[control-plane-authority-distributed-execution]] separates control plane registration/routing/policy from child execution. [[explicit-fail-closed-over-hidden-fallbacks]] requires visible errors over hidden compatibility tunnels. | `layer/core/beliefs/control-plane-authority-distributed-execution.md`, `layer/core/beliefs/explicit-fail-closed-over-hidden-fallbacks.md` |
| Slate is emerging as project-living work/proof layer. | Slate artifacts live under `layer/slate/work/<id>/work.toml`; projection DB is rebuildable/derived. | `layer/slate/README.md`, `src/slate.rs` |
| `patina spec` is still compatibility workflow, but Slate/MCT are the direction. | Active specs include [[slate-pando-migration]] and [[sdk-vision-lock]], while `README.md` still emphasizes `patina spec` command workflow. | `layer/surface/build/feat/slate-pando-migration/SPEC.md`, `layer/surface/build/feat/sdk-vision-lock/SPEC.md`, `README.md` |

## 3. MCT Vocabulary Lock

Use this vocabulary consistently in README and marketing material.

### Patina

A local-first knowledge protocol and runtime for AI-assisted development. It captures repository activity, indexes it, retrieves it, grounds it in beliefs, and evolves project memory over time.

Short form:

> Patina turns a repo into a durable, searchable, agent-ready knowledge layer.

### Mother

The control plane. Mother owns orchestration concerns: project registry, routing, daemon lifecycle, child loading, grants, cross-project graph/search, secrets/session coordination, and runtime views.

Marketing version:

> Mother is the local control plane that coordinates knowledge, policy, and tools across projects.

### Child

A sandboxed capability unit, usually a WASI component with a `child.toml` manifest and typed WIT operations. Children do specific work: verify beliefs, manage Slate work, write parquet, monitor folders, run doctor checks, enforce schema, etc.

Marketing version:

> Children are small, typed tools Mother can load, grant, route to, observe, and replace.

### Toy

An explicit capability granted to a child: filesystem, git, logging, keyvalue, measure, messaging, etc. Toys are how Patina prevents ambient authority.

Marketing version:

> Toys are capability handles: a child only gets the powers it declares and Mother grants.

### Pando

A composed product made from children. README currently says pandos are composed groups of children that appear as user-facing products. Keep this, but do not lead with it until MCT is explained.

### Slate

Project-living work/proof transactions. Slate should be described as the new workbench layer for build/refactor/fix work, grounded by Allium intent and beliefs, with Mother projections as derived state.

### Allium

Behavioral/product intent. Allium files capture what should be true about Mother and other product surfaces. Do not describe Allium as test-only or implementation-only.

### Beliefs

Evidence-backed project doctrine: principles, decisions, and reusable lessons. Beliefs explain why an architectural choice is trusted and what evidence could change it.

## 4. Audience

Primary audiences for this positioning:

1. **AI-heavy developers** who repeatedly re-explain a codebase to assistants and want memory that compounds.
2. **Agent/tool builders** who need safe local orchestration, typed tools, and explicit capabilities.
3. **Rust/WASI/component-model developers** who understand why sandboxed components and WIT contracts matter.
4. **Solo builders and small teams** who want durable project knowledge without adopting a cloud platform.
5. **AI workflow skeptics** who need proof: local files, git history, compiler checks, explicit grants, and fail-closed boundaries.

Secondary audiences:

- Documentation-heavy open source projects.
- Teams with multiple repos and recurring architectural decisions.
- Developers building private/local agent control planes.

## 5. Positioning Pillars

### Pillar 1 — Local-first project memory

Patina makes project knowledge durable by storing it in local files and rebuildable local databases. `layer/` is git-tracked; `.patina/` and Mother databases are local projections/caches/runtime state.

Message:

> Your project should remember what happened, why it happened, and where the proof lives.

### Pillar 2 — Retrieval that agents can actually use

Patina provides `scry` for semantic retrieval, `assay` for factual/structural retrieval, and `context` for project guidance. This turns knowledge into command surfaces that humans and agents can call.

Message:

> Patina is not another chat UI. It is the memory and retrieval substrate behind one.

### Pillar 3 — Mother as control plane

Mother coordinates projects, children, grants, sessions, secrets, graph routing, and runtime views. It is the local control plane, not the LLM.

Message:

> Mother decides what is available, what is allowed, and where work should go.

### Pillar 4 — Children as typed, replaceable capabilities

Children are standalone components with manifests and contracts. They are the extraction path for functionality that should not live forever in the core binary.

Message:

> Instead of one growing binary, Patina grows by loading small, typed children.

### Pillar 5 — Toys as explicit power

Toys make capability boundaries visible. A child asks for specific powers; Mother grants and observes them. This is the concrete safety story behind MCT.

Message:

> Agent tools should not get ambient authority by accident.

### Pillar 6 — Intent and doctrine alongside code

Allium captures intended behavior; beliefs capture reusable doctrine and evidence; Slate captures work/proof transactions. Together they make AI work auditable rather than ephemeral.

Message:

> Patina does not only remember code. It remembers intent, decisions, evidence, and unfinished work.

## 6. Current README Gaps

### Gap: tagline is too small

Current:

> Context management for AI-assisted development

Better direction:

> Local-first project memory and tool orchestration for AI-assisted development.

Or sharper:

> A local knowledge protocol and control plane for AI-assisted software work.

### Gap: MCT appears late

README introduces Mother and children, but only after install and command lists. If MCT is the product center, the README needs a short early explanation.

Suggested early placement: after “What Patina Does”, add “The Mother / Child / Toy model”.

### Gap: “toy” is not explained

README mentions Patina toy interfaces but does not define toy in user/product terms. This loses the safety/capability story.

### Gap: proof points are scattered

The repo has strong proof: Allium specs, child manifests, WIT operations, local DB layout, fail-closed runtime behavior, release/session/spec workflow. README lists features but does not connect them into a trust story.

### Gap: too much command inventory before product shape

The current README is useful as a command reference, but marketing needs a clearer first-screen arc:

1. What problem Patina solves.
2. What makes it different.
3. How MCT works.
4. What is real today.
5. How to try it.

### Gap: experimental status is truthful but under-framed

The experimental warning is right. It should remain, but the README can pair it with a stronger “why this exists” story so experimental does not read as directionless.

## 7. Marketing Copy Blocks

### One-liner

Patina is a local-first knowledge protocol and Mother/Child/Toy control plane for AI-assisted software development.

### Short pitch

Patina turns a repository into durable memory for humans and AI agents. It captures code, git history, specs, sessions, beliefs, and external sources; indexes them locally; and exposes retrieval and workflow commands that make project context reusable. Mother extends that memory into a local control plane: it routes work to sandboxed children, grants explicit toys/capabilities, and keeps AI tooling observable instead of ambient.

### Long pitch

Modern AI coding workflows lose context constantly. Every new session asks the same questions: what matters in this repo, what did we decide, what is safe to change, and what work is still unfinished?

Patina makes that knowledge durable. It scrapes your project, builds local semantic and structural indices, records sessions/specs/beliefs, and keeps the durable layer in git. Agents can retrieve project truth with `scry`, inspect structure with `assay`, and load project rules with `context`.

Mother takes the next step: a local control plane for AI development systems. Instead of giving one tool ambient access to everything, Mother loads sandboxed children with typed WIT contracts. Each child declares the toys/capabilities it needs — filesystem, git, logging, measurement, messaging — and Mother controls routing, grants, readiness, and observability.

The result is a repo that can remember, explain, and safely coordinate work across humans, agents, and tools.

### MCT explainer

Mother / Child / Toy is Patina’s architecture for safe local agent tooling:

- **Mother** is the control plane. It knows the project graph, available children, policies, grants, and routes.
- **Children** are small WASI components that perform specific work through typed contracts.
- **Toys** are explicit capabilities granted to children, such as filesystem, git, key-value state, logging, or measurement.

This turns “let the agent run tools” into a contract: who is allowed to do what, through which typed operation, with which capability.

### Feature bullets

- Local-first project memory: code, git, specs, sessions, beliefs, and external sources.
- Semantic search with `patina scry`; structural/factual search with `patina assay`.
- Git-tracked knowledge layer under `layer/`; rebuildable local projections under `.patina/` and `~/.patina/`.
- Mother daemon for project registry, cross-project graph/search, child orchestration, secrets/session coordination, and runtime views.
- WASI/WIT child architecture with explicit `child.toml` manifests and toy grants.
- Beliefs for evidence-backed doctrine and Allium for behavioral intent.
- Slate work items for project-living build/refactor/fix transactions and proof trails.

### Proof points to cite

- `layer/core/patina-identity.md`: protocol verbs and layer-as-product framing.
- `README.md`: local DB layout, command surfaces, repository layout.
- `layer/allium/mother/*.allium`: behavioral specs for Mother lifecycle, orchestration, routing, and secrets/session behavior.
- `children/*/child.toml`: real first-party child manifests with declared toys.
- `children/slate-manager/wit-contract/slate.wit`: typed Slate work/spec operations.
- `mother/src/runtime.rs`: typed call default fail-closed behavior.
- `src/slate.rs`: project-living Slate artifacts projected into Mother `slate.db`.

### Audience-specific angles

#### For AI-heavy developers

Stop re-explaining your repo. Patina gives agents a local, queryable memory of code, decisions, sessions, specs, and beliefs.

#### For tool builders

Build tools as children: typed components with explicit capabilities, loaded and routed by Mother.

#### For Rust/WASI developers

Patina applies the component model to AI development workflows: WIT contracts, `wasm32-wasip2` children, explicit capability grants, and local-first host orchestration.

#### For skeptics

Patina keeps the trust boundary visible. Durable knowledge lives in git-tracked files. Runtime state is local. Unsupported typed paths fail closed. Capability grants are declared in manifests.

## 8. Proposed README Rewrite Plan

### Phase 1 — Sharpen above-the-fold message

Replace or extend the current tagline:

Current:

> Context management for AI-assisted development

Candidate:

> Local-first project memory and tool orchestration for AI-assisted development.

Add one paragraph:

> Patina turns a repository into a durable knowledge layer and gives AI tools a local control plane. It captures code, git history, specs, sessions, beliefs, and external sources; indexes them locally; and lets Mother route typed work to sandboxed children with explicit toy/capability grants.

### Phase 2 — Add “Mother / Child / Toy” section near top

Insert after “What Patina Does”:

```md
## Mother / Child / Toy

Mother is Patina’s local control plane. Children are sandboxed WASI components with typed WIT contracts. Toys are explicit capabilities — filesystem, git, logging, measurement, messaging — granted to children by Mother.

This lets Patina grow by adding small, typed capabilities instead of giving one agent or monolithic binary ambient authority over the whole project.
```

### Phase 3 — Reframe “What Patina Does” around pillars

Change bullets from feature inventory to product value:

- Captures durable project memory from code, git, specs, sessions, beliefs, and sources.
- Retrieves that memory through semantic (`scry`), structural (`assay`), and guidance (`context`) surfaces.
- Grounds AI work in intent and doctrine using Allium, beliefs, specs, sessions, and Slate.
- Coordinates tools through Mother, children, typed WIT operations, and explicit toy grants.
- Stays local-first: git-tracked knowledge, rebuildable projections, machine-local runtime state.

### Phase 4 — Move command guide lower or compress early version

Keep the command guide, but make the early README read like product positioning before it becomes a reference manual.

### Phase 5 — Add architecture diagram focused on MCT

Keep the existing storage/query architecture diagram, and add a smaller MCT diagram:

```text
Human / AI interface
        |
      Mother  ---- project graph / policy / grants / routing
        |
   typed WIT calls
        |
     Children ---- Toys: filesystem, git, logging, measure, state
```

### Phase 6 — Preserve experimental status

Keep the warning, but improve context:

> Patina is experimental because it is building a local knowledge protocol and component control plane for AI development. Core retrieval and project-memory workflows are usable today; MCT/Slate surfaces are evolving quickly.

## 9. Open Questions Before README Editing

1. Should the public README lead with “knowledge protocol” or “project memory”?
   - “Project memory” is easier for users.
   - “Knowledge protocol” is more precise but abstract.
   - Recommendation: lead with “project memory”, explain protocol later.

2. How prominent should Slate be?
   - Slate is real and project-living now, but top-level CLI is not ready yet.
   - Recommendation: mention Slate as emerging work/proof layer, not first-run workflow.

3. Should “Pando” stay in top-level marketing?
   - It is less foundational than MCT for current positioning.
   - Recommendation: keep in architecture/reference sections, not headline.

4. Should README mention Allium?
   - Yes, but after beliefs/specs/context. Explain it as behavioral intent, not a prerequisite for first run.

## 10. First Recommended README Diff

Smallest useful diff:

1. Change tagline.
2. Rewrite first product paragraph.
3. Add a short MCT section near top.
4. Add toy definition.
5. Add a “Why it matters” paragraph tying memory + control plane together.

Do not rewrite the full README in one pass until the positioning direction is approved.
