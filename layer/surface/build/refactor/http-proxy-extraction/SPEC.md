---
type: refactor
id: http-proxy-extraction
status: draft
created: 2026-03-10
sessions:
  origin: 20260310-074810
related:
- patina-connect
- ducklake
- pipe-architecture
exit_criteria: []
---
# refactor: Extract HTTP proxy to shared crate for child reuse

> Move HTTP proxy logic (domain validation, credential injection, leak detection) from broker/http.rs into patina-pipe so any child can proxy connector HTTP

## Current State

## Target State

## Steps

## Exit Criteria
