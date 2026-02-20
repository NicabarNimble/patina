---
type: belief
id: context-before-code
persona: architect
facets: [workflow, methodology, debugging]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-20
revised: 2026-02-20
---

# context-before-code

When blocked or confused, gather context from layer/sessions, specs, and git history before exploring code — linear understanding prevents loops and wasted effort

## Statement

When blocked or confused, gather context from layer/sessions, specs, and git history before exploring code — linear understanding prevents loops and wasted effort

## Evidence

- [[session-20260220-120045]]: User stopped exploration mid-task, insisted on reading layer/sessions and specs first — revealed full Feb 18→19→20 regression timeline and led directly to correct fix (weight: 0.95)
- [[spec-keychain-macos26-regression]]: Reading [[spec-secrets-keychain-ssh]], [[spec-launcher-auth]], and session history exposed that [[commit-1cca67ed]] had the right API but wrong policy — wouldn't have found this by exploring code alone (weight: 0.9)

## Supports

- [[read-code-before-write]]: Context gathering includes reading existing code, but expands to sessions/specs/history for blocked cases
- [[spec-first]]: Specs are context — reading them before coding prevents re-inventing solutions

## Attacks

- "Just explore the code, it has the truth": **Defeated** — code shows current state but not the reasoning behind it. [[session-20260219-083531]] and [[session-20260218-225007]] captured the "why" behind the decisions, which code alone doesn't reveal.
- "Git history is enough": **Partially defeated** — git shows what changed but not the full decision context. Session notes link commits to problems, reasoning, and trade-offs.

## Attacked-By

- "Context gathering takes too long, just start coding": Initial exploration feels faster but creates loops when you miss key constraints. The Feb 19 fix looked correct in isolation but broke SSH — reading [[spec-secrets-keychain-ssh]] would have prevented the regression. (status: defeated)
- "Sessions/specs might be out of date": Valid concern — but even stale context reveals the original intent, which informs how to evolve it correctly. (status: acknowledged)

## Applied-In

- [[session-20260220-120045]]: Used `patina scry` and manual file reads to trace keychain regression timeline before fixing code
- This session workflow: Read last-session.md → Read full session file → Read relevant specs → Read beliefs → THEN read/modify code
- [[spec-keychain-macos26-regression]]: Documents the value of preserving context (new spec vs overwriting old one)

## Revision Log

- 2026-02-20: Created — metrics computed by `patina scrape`
