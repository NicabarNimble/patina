---
type: belief
id: pandos-are-products-children-are-compute
persona: architect
facets: [architecture, children, pando]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-06
revised: 2026-04-06
---

# pandos-are-products-children-are-compute

Pandos are user-facing products, children are internal compute. Users install pandos, not children.

## Statement

Pandos are user-facing products, children are internal compute. Users install pandos, not children.

## Evidence

- [[session-20260405-133644]] - Architectural discussion: children are libraries, pandos are apps. A child like schema-enforcer has no business being a CLI command. The pando is the user-facing thing. (weight: 0.95)

## Supports

- [[pando-is-composed-children]] — pandos are the composition layer; this belief adds that pandos are specifically the *user-facing* composition layer
- [[children-have-agency-toys-are-capabilities]] — children have bounded agency within the sandbox; pandos are what gives that agency a user-facing surface

## Applied-In

- [[spec-child-construction-canon]] — pando added as fourth architectural concept (Mother, Children, Toys, Pandos)
- Slate pando (planned) — first interactive pando with CLI commands, replacing spec-manager builtin dispatch

## Revision Log

- 2026-04-06: Created — metrics computed by `patina scrape`
