---
type: belief
id: five-boundaries-no-overlap
persona: architect
facets: [architecture, identity, sdk, children]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-24
revised: 2026-04-01
---

# five-boundaries-no-overlap

Five responsibility boundaries, no overlap: Patina (belief core), Mother (infrastructure daemon), child + toy (knowledge worker), interface + agent (AI guest), projects (user workspace).

## Statement

Five responsibility boundaries, no overlap: Patina (belief core), Mother (infrastructure daemon), child + toy (knowledge worker), interface + agent (AI guest), projects (user workspace).

## Evidence

- [[session-20260324-101606-299953000]] - Original four-role distillation: Patina owns the knowledge protocol, Mother owns the runtime, children do the work, projects are where you build. (weight: 0.95)
- [[session-20260331-224232-852361000]] - Full code audit revealed interfaces and agents as a distinct responsibility boundary with their own code surface (`src/interface/`, `AiInterface` trait), lifecycle (bootstrap, check-in, projection), and architectural role (AI guests that consume Patina through Mother). Expanded from four to five roles. (weight: 0.90)

## The Five Roles

1. **Patina** — belief core. Five protocol verbs (capture, index, search, believe, evolve). Native CLI. Eventlog is truth, layer is the product.
2. **Mother** — infrastructure daemon. Hosts children, manages state, secrets, graph, broker. Serves scry. The authority boundary.
3. **Child + Toy** — knowledge worker. Children are WASM compute legos, toys are their granted capabilities. Inseparable pair: children declare needs, Mother grants toys.
4. **Interface + Agent** — AI guest. Interfaces adapt (Claude, Gemini, OpenCode), agents consume. Guests never own infrastructure.
5. **Projects** — user workspace. Where `patina init` runs, where `layer/` accumulates, where children are developed. Git-tracked, portable.

## Supports

- [[patina-identity]] — refines the protocol identity into distinct architectural roles
- [[core-primitives-are-not-children]] — children are workers, not the protocol itself
- [[children-have-agency-toys-are-capabilities]] — children use toys granted by Mother
- [[core-baseline-child-strategy-extensions]] — children extend core, never replace it
- [[agents-are-guests-mother-is-infrastructure]] — agents are guests, Mother is infrastructure

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[child-construction-canon]] — "Only Three Things" section covers the Mother/child/toy runtime boundary. The full five-role framing includes Patina core and interface/agent layers.
- [[sdk-toybox-definition]] — toybox defined as Mother's controlled surface, children consume via SDK
- [[fix-grammar-pipeline]] — grammar children use pipeline toys, embedded fallback uses same WASM path
- [[greenfield-mother-patina-rebuild]] — M6 crate architecture separates core (patina) from runtime (mother)

## Revision Log

- 2026-03-24: Created — metrics computed by `patina scrape`
- 2026-04-01: Revised — expanded from four roles to five. Added interface + agent as distinct responsibility boundary (AI guest). Toys paired with children as inseparable knowledge worker unit. Projects reframed from "dev zone" to "user workspace."
