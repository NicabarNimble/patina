---
type: belief
id: bearer-token-forces-plaintext-exposure
persona: architect
facets: [security, protocol, secrets]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-22
revised: 2026-02-22
---

# bearer-token-forces-plaintext-exposure

Bearer token auth forces plaintext credential exposure to the consumer — this is an industry-wide protocol constraint, not an implementation gap.

## Statement

Bearer token auth forces plaintext credential exposure to the consumer — this is an industry-wide protocol constraint, not an implementation gap.

## Evidence

- [[session-20260222-165738]]: IronClaw proxy code (`nearai/ironclaw` src/sandbox/proxy/http.rs:246-248) explicitly states HTTPS CONNECT tunnels cannot inject credentials without MITM. Falls back to env vars for authenticated HTTPS. (weight: 0.95)
- Industry survey: GitHub Actions (`${{ secrets.X }}` → env var), Docker (env/mounted files), Kubernetes (env from secrets), 1Password CLI (`op run --` → env vars), HashiCorp Vault (agent provides token to app) — all inject plaintext. (weight: 0.9)
- Protocol alternatives exist but require API provider support: AWS Sig V4 (signature travels, not key), mTLS (client cert auth), FIDO2/WebAuthn (hardware signs challenge). None are used by LLM API providers today. (weight: 0.85)
- User insight: "The API key unlocks the LLM, not LLM unlocks self" — auth is transport layer, not LLM layer. Separating these would require protocol change. (weight: 0.8)

## Supports

- [[storage-encryption-vs-runtime-isolation]]
- [[llm-threat-model-unique]]

## Attacks

- Implicitly attacks any design that assumes a proxy can hide credentials from HTTPS consumers without MITM

## Attacked-By

- "AWS Sig V4 proves credentials don't need to be plaintext" — true for AWS, but requires API provider adoption. Anthropic/OpenAI use Bearer tokens today.
- "Local MITM proxy with custom CA" — technically works but breaks TLS trust model, invasive to deploy

## Applied-In

- `src/secrets/mod.rs::run_with_secrets()`: Bearer token injected as `ANTHROPIC_API_KEY` env var — the only interface Claude Code supports
- `nearai/ironclaw` src/sandbox/proxy/http.rs: HTTP credential injection works, HTTPS falls back to env vars
- Future: If Anthropic API ever supports signed requests, patina's age identity could sign directly without exposing plaintext

## Revision Log

- 2026-02-22: Created — metrics computed by `patina scrape`
