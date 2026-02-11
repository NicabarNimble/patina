---
type: belief
id: take-plus-one-antipattern-on-streams
persona: architect
facets: [rust, sockets, antipattern]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-10
revised: 2026-02-10
---

# take-plus-one-antipattern-on-streams

Using Read::take(n+1) to detect oversized bodies causes deadlocks on non-EOF streams — size enforcement should use declared Content-Length, not read-ahead

## Statement

Using Read::take(n+1) to detect oversized bodies causes deadlocks on non-EOF streams — size enforcement should use declared Content-Length, not read-ahead

## Evidence

- [[session-20260210-222841]]: [[commit-aecaf18c]] - take(content_length+1) in microserver.rs blocked read_to_end waiting for 1 byte that never arrives on Unix sockets, causing silent failures in session.rs::uds_post() since inception (weight: 0.95)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-02-10: Created — metrics computed by `patina scrape`
