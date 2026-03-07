---
type: belief
id: specs-describe-current-code-not-aspirations
persona: architect
facets: [governance, specs, process]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-07
revised: 2026-03-07
---

# specs-describe-current-code-not-aspirations

Spec EC text and verify fields must describe what the code does today, not what it will do after future work — forward references are fine, aspirational ECs are lies

## Statement

Spec EC text and verify fields must describe what the code does today, not what it will do after future work — forward references are fine, aspirational ECs are lies

## Evidence

- [[session-20260307-092539]]: Another agent updated pipe-native-transport specs to say "deny all outbound sockets" when the code still allowed port 443/53. Caught and corrected: specs were describing pipe-mother-io's future state as current reality. (weight: 1.0)
- Same session: original pipe-native-transport spec claimed "domain-level filtering" via SBPL regex. The SBPL syntax was invalid and had never been tested. The spec described intent, not reality. (weight: 0.9)

## Supports

- [[spec-driven-design]] — "SPECs decide. Code executes." A spec that describes code that doesn't exist has decided nothing — it's aspirational prose, not a contract.

## Attacks

- "Specs should describe target state" — Counter: target state belongs in the Steps or Target State section. EC verify fields are contracts — they must be verifiable against the current codebase. A verify field that can't be run is not a verify field.

## Attacked-By

- "Forward references blur the line" — Valid tension. Saying "current sandbox is port-level; pipe-mother-io tightens to deny-all" is a forward reference, not an aspirational EC. The distinction: the EC verify field tests current code, the prose explains what comes next.

## Applied-In

- [[spec-pipe-native-transport]] EC3 — rewritten from "deny all outbound sockets" to "port 443 + DNS, tested via fork-based tests." Forward reference to pipe-mother-io for tightening.
- [[spec-pipe-native-transport]] "Known Gap: Domain Enforcement" section — explicitly states the gap and names the fix, rather than pretending the gap doesn't exist.
- [[commit-b5c527c5]] — corrected 3 specs that described future state as current.

## Revision Log

- 2026-03-07: Created — metrics computed by `patina scrape`
