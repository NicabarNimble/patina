---
type: belief
id: restart-mother-after-binary-install
persona: patina
facets: [development, mother, installation, operations]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-05-08
revised: 2026-05-08
---

# restart-mother-after-binary-install

After replacing the installed Patina CLI binary, restart Mother so the daemon process and CLI behavior do not drift.

## Statement

After replacing the installed Patina CLI binary, restart Mother so the daemon process and CLI behavior do not drift.

## Evidence

- In [[session-20260508-112917-717692000]], cargo install replaced ~/.cargo/bin/patina and patina mother restart was required for launchd Mother to run the current dev binary; this became the local dev lane.

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
