---
type: belief
id: audit-before-refactor
status: active
confidence: high
entrenchment: medium
facets: [process, quality, code-review]
session_origin: session-20260331-224232-852361000
created: 2026-03-31
---
# Audit Before You Refactor

Read every module before making any judgment about it. Deep-dive file-level audits surface findings that structural scans miss.

## Evidence

In [[session-20260331-224232-852361000]], 8 parallel audit agents reading every file surfaced 6 high-severity findings (UTF-8 panic risk, thread-unsafe CWD mutation, divergent capability allowlists, stubbed data loss, dimension mismatch, schema drift) that the initial structural scan did not detect. An independent verification agent then confirmed all 26 findings and corrected a count error (17 → 18 unarchived specs). The cost of reading code is low; the cost of recommending changes to code you haven't read is high.

## Test

Before proposing a change to a module: have you read the module? If you're recommending removal: have you grepped for callers? If you're flagging dead code: have you checked git blame for why it exists?

## Connects

- [[patina-identity]] — the five verbs define what's core vs tooling vs infrastructure
- [[dependable-rust]] — private internals mean you must read the module to know what's inside
- [[read-code-before-write]] — the operational form of this belief
