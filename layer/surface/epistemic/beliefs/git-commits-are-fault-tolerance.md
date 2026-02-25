---
type: belief
id: git-commits-are-fault-tolerance
persona: architect
facets: [workflow, resilience, git]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-25
revised: 2026-02-25
---

# git-commits-are-fault-tolerance

Frequent small git commits during AI sessions provide fault tolerance against API failures — each commit is a recovery checkpoint, so context loss (500 errors, disconnects) never loses work

## Statement

Frequent small git commits during AI sessions provide fault tolerance against API failures — each commit is a recovery checkpoint, so context loss (500 errors, disconnects) never loses work

## Evidence

- [[session-20260225-115133]]: 10+ API 500 errors across sessions 3-4 of spec audit batch; zero work lost because every logical change was committed immediately (weight: 0.9)
- [[session-20260225-113728]]: 5 API 500 errors during [[spec-query-filesystem-truth]] session; each recovery picked up at last git commit (weight: 0.8)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[spec-complete-atomicity]], [[spec-scan-efficiency]], [[spec-query-filesystem-truth]], [[spec-history]] — four specs completed across sessions with frequent 500 errors, zero rework needed
- Session-git integration: `patina session start/update/end` creates git tags at session boundaries, making every session recoverable

## Revision Log

- 2026-02-25: Created — metrics computed by `patina scrape`
