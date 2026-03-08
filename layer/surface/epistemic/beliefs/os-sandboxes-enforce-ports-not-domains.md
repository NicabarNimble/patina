---
type: belief
id: os-sandboxes-enforce-ports-not-domains
persona: architect
facets: [security, architecture, sandbox]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-07
revised: 2026-03-07
---

# os-sandboxes-enforce-ports-not-domains

OS sandboxes (macOS SBPL, Linux Landlock) operate on IP:port pairs, not hostnames — domain-level enforcement requires application-layer proxying through Mother

## Statement

OS sandboxes (macOS SBPL, Linux Landlock) operate on IP:port pairs, not hostnames — domain-level enforcement requires application-layer proxying through Mother

## Evidence

- [[session-20260307-092539]]: Fork-based sandbox tests revealed sandbox_init() SBPL and Landlock cannot filter by hostname. Port 443 allows any HTTPS host. Discovered by testing, not by reading docs. (weight: 1.0)
- macOS SBPL `remote ip` filter matches `"host:port"` strings at the socket level — DNS resolution happens before `connect()`, so the sandbox only sees IP addresses (weight: 0.9)
- Linux Landlock `AccessNet::ConnectTcp` restricts by port number via `NetPort` — no hostname concept in the Landlock API (weight: 0.9)

## Supports

- [[host-proxied-io-is-the-security-model]] — this constraint is WHY I/O must be proxied through Mother. The OS sandbox alone is insufficient for domain enforcement.

## Attacks

- [[pipes-are-processes-not-wasm]] — partially challenges the claim that "OS-level sandboxing gives you the same security guarantees." It gives filesystem and port-level guarantees, but NOT domain-level. WASM children get domain enforcement via host functions; native children need pipe/http to match.

## Attacked-By

- "DNS-level enforcement could work" — a filtering DNS resolver that only resolves declared domains. Counter: a malicious child could hardcode IPs, bypassing DNS entirely. Application-layer proxying is more robust.

## Applied-In

- [[spec-pipe-mother-io]] — created specifically to close this gap. Mother proxies all HTTP, checks domains against manifest.
- `crates/patina-pipe/src/sandbox.rs` — current code allows port 443 (temporary), will be tightened to deny-all when pipe-mother-io lands.
- [[spec-pipe-native-transport]] "Known Gap: Domain Enforcement" section — documents this constraint and references the fix.

## Revision Log

- 2026-03-07: Created — metrics computed by `patina scrape`
