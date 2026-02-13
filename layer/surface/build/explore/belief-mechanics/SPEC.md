---
type: explore
id: belief-mechanics
status: design
created: 2026-02-13
sessions:
  origin: 20260213-120746
related:
- layer/core/patina-identity.md
beliefs:
- patina-is-knowledge-protocol
- beliefs-are-entities-not-documents
- anti-tunneling-as-belief-challenge
---

# explore: Belief Mechanics — Challenge, Speculation, and Idea Injection

> Beliefs should be the one system for all project knowledge assertions.
> Right now they only capture validated decisions. We need them to also
> carry challenges to existing knowledge and speculative ideas that
> haven't been proven yet.

## Problem

107 beliefs exist. All are `status: active`. All are `entrenchment: medium`
or higher. There is no mechanism for:

1. **Challenge** — When new evidence contradicts an existing belief, there's
   no workflow to flag it, track the tension, or force resolution. Today we
   discovered [[patina-is-knowledge-protocol]] challenged the 6-pillar model
   in patina-identity. We resolved it by updating the identity doc directly.
   No belief was formally challenged. The `status: challenged` field exists
   in the schema but has never been used.

2. **Speculation** — When someone has a hunch ("the spec system will become
   a command-world plugin"), there's nowhere to put it. It's too ungrounded
   for a belief but too valuable to lose. Ideas that could shape architecture
   disappear into session logs.

3. **Idea injection** — External input (agent reviews, user insights,
   reading notes) generates assertions that should enter the belief system
   but aren't yet project-tested. They need a home that's findable via scry
   but clearly marked as unvalidated.

## Observation: One System, Multiple Confidence Levels

The user's insight: "they should all still be beliefs, just less validated
or anchored in reality." This means:

- Don't build a separate "ideas" system or "challenges" system
- Use the existing belief format with expanded entrenchment levels
- Let the same scry/assay/context tools find everything
- Confidence level determines how beliefs influence decisions

## Current Schema

```yaml
type: belief
id: some-belief-name
persona: architect
facets: [architecture, rust]
entrenchment: medium          # low | medium | high | very-high
status: active                # active | challenged | retired
endorsed: true
```

Current entrenchment levels in practice: `medium` (97%), `high` (9),
`very-high` (1). Nobody uses `low`. Nobody uses `status: challenged`.

## Proposed Extensions

### Entrenchment: Add `speculative`

```
speculative → low → medium → high → very-high
```

`speculative` beliefs have:
- No evidence yet (or evidence from outside the project)
- A statement (the assertion)
- Optional `origin` field (where the idea came from)
- No verification queries
- No applied-in references

They graduate to `low` when first evidence arrives. They graduate to
`medium` when evidence spans multiple sessions or commits.

### Status: Activate `challenged`

A belief becomes `challenged` when:
- New evidence directly contradicts its statement
- A newer belief's supports/attacks section names it
- An identity doc or core pattern changes in ways that conflict

Challenge workflow:
1. Create the challenging belief (or update the attacker's Attacks section)
2. Set the challenged belief's `status: challenged`
3. The `patina belief audit` command surfaces challenged beliefs prominently
4. Resolution: update the belief, retire it, or reject the challenge

### New field: `challenges` (optional)

```yaml
challenges:
- target: v1-three-pillars
  reason: Protocol framing reduces core from 6 pillars to 5 verbs
```

This is the inverse of `attacks`. Attacks say "this belief weakens that
one." Challenges say "this belief may invalidate that one — review needed."

### New field: `origin` (optional)

```yaml
origin: external-agent-review     # or: session, reading, hunch, user
```

Tracks where a speculative belief came from. Useful for audit — "how many
of our speculative beliefs from agent reviews actually graduated?"

## What Changes in Code

### `patina belief audit`

- Add a `CHALLENGED` section — show beliefs with `status: challenged`
- Add a `SPECULATIVE` section — show beliefs with `entrenchment: speculative`
- Existing sections (by entrenchment, by facet) unchanged

### Belief creation (skill + scrape)

- The `/epistemic-beliefs` skill should support `entrenchment: speculative`
- `patina scrape` belief indexer already handles all entrenchment values
- No schema migration needed — just new values in existing fields

### `patina belief challenge <id>` (new subcommand, optional)

```bash
patina belief challenge v1-three-pillars \
  --reason "Protocol framing reduces core to 5 verbs" \
  --challenger patina-is-knowledge-protocol
```

Sets `status: challenged` on the target, adds to revision log. This is
sugar — you can always edit the file directly.

## What This Enables

- **Ideas don't die in sessions** — speculative beliefs survive and are
  findable via scry. "What did we speculate about the spec system?" works.
- **Contradictions are visible** — `patina belief audit` shows tensions
  instead of hiding them behind 107 green "active" statuses.
- **Knowledge has a lifecycle** — speculative → low → medium → high mirrors
  how real understanding forms. Not everything starts validated.
- **External input has a landing zone** — agent reviews, user hunches, and
  reading notes enter the belief system with honest confidence markers.

## Exit Criteria

- [ ] Document the entrenchment graduation model
- [ ] Create at least one speculative belief to test the workflow
- [ ] Challenge at least one existing belief to test the workflow
- [ ] Determine if `patina belief challenge` subcommand is needed or if
      manual editing + audit visibility is sufficient
- [ ] Update the `/epistemic-beliefs` skill to support speculative beliefs

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | design | Emerged from identity doc reframe. Protocol framing challenged 6-pillar model but no belief mechanism existed to track the challenge. |
