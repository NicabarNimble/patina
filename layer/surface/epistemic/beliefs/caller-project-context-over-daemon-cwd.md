---
type: belief
id: caller-project-context-over-daemon-cwd
persona: patina
facets: [architecture, mother, cli, context]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-05-08
revised: 2026-05-08
---

# caller-project-context-over-daemon-cwd

Global daemons should not infer project intent from daemon cwd; caller-facing commands should derive or pass caller project context explicitly.

## Statement

Global daemons should not infer project intent from daemon cwd; caller-facing commands should derive or pass caller project context explicitly.

## Evidence

- In [[session-20260508-112917-717692000]], launchd-started Mother had no repo cwd, so patina mother status was fixed in [[commit-b3555b29]] to display Project context from the CLI caller's current repo instead of daemon startup cwd.

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-05-08: Created — metrics computed by `patina scrape`
