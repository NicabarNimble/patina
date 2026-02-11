---
type: belief
id: live-testing-reveals-socket-bugs
persona: architect
facets: [testing, rust, sockets]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-10
revised: 2026-02-10
---

# live-testing-reveals-socket-bugs

Unit tests with mock I/O (Cursor, Vec) pass but mask stream semantics bugs — always test stream-based code with real sockets, not just in-memory readers

## Statement

Unit tests with mock I/O (Cursor, Vec) pass but mask stream semantics bugs — always test stream-based code with real sockets, not just in-memory readers

## Evidence

- [[session-20260210-222841]]: [[commit-aecaf18c]] - microserver take(len+1) deadlock passed all Cursor-based unit tests, only live UnixStream testing exposed the read_to_end blocking (weight: 0.95)

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
