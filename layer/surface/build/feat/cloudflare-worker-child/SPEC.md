---
type: feat
id: cloudflare-worker-child
status: draft
created: 2026-03-26
blocked_by:
  - toy-collapse-wasi-alignment
sessions:
  origin: 20260325-150227-161735000
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[children-are-wasm]]"
related:
  - sdk/patina-sdk/
  - wit/toys/
  - children/
exit_criteria:
  - id: cwc1-single-component
    text: "A single WASM component exists that compiles once and runs on both Patina (under Mother) and Cloudflare Workers (via wrangler). Same .wasm artifact, two runtimes."
    checked: false
  - id: cwc2-patina-runtime
    text: "The child runs under Mother using Patina toys (http, store, events, log). Toybox grants are mediated. child.toml manifest works."
    checked: false
  - id: cwc3-cloudflare-runtime
    text: "The same component runs on Cloudflare Workers with CF bindings satisfying the toy imports (fetch→http, KV→state, D1→store, Queues→events). wrangler.toml manifest works."
    checked: false
  - id: cwc4-same-behavior
    text: "Given the same inputs, the child produces the same outputs on both runtimes. Verified by test fixture."
    checked: false
  - id: cwc5-no-runtime-detection
    text: "The child code contains zero runtime detection logic. It does not know or care whether it's running under Mother or Cloudflare. The toy/binding abstraction is complete."
    checked: false
  - id: cwc6-documented
    text: "A guide exists for building portable children: how to write a child that runs on both Patina and Cloudflare, what toys map to what bindings, what the limitations are."
    checked: false
---
# feat: Build a Patina child that runs as a Cloudflare Worker

> Prove the toy/binding portability promise by building one WASM component that runs under Mother with Patina toys AND on Cloudflare Workers with CF bindings. Same child code, two runtimes.

## Problem

The toy collapse spec claims Patina's architecture aligns with Cloudflare Workers' binding model and the WASM component model's portability promise. But the claim is unproven. No Patina child has ever run on a second runtime. Without a concrete proof, the alignment is a diagram, not a property.

## Goal

Build one child. Compile it once. Run it on Patina and Cloudflare. Prove the portability is real, discover what breaks, and document the gaps.

This is the highest-value proof the architecture can produce: if a single WASM component can be hosted by two completely different runtimes through the same WIT interfaces, the toybox model is genuinely portable — not just Patina-locked.

## Non-Goals

- Do NOT build a general Patina→Cloudflare deployment pipeline. This is one proof-of-concept child.
- Do NOT make ALL children portable. Most Patina children use Patina-specific toys (store, events, peer) that don't have CF equivalents yet. This child uses the WASI-aligned subset.
- Do NOT modify Cloudflare's runtime. We use their platform as-is.
- Do NOT build a CF-to-Patina adapter. The portability comes from the WIT interface alignment, not from glue code.

## Session Discovery Context

This spec emerged from session `20260325-150227-161735000`, where we:
1. Discovered Patina's toybox model maps 1:1 to Cloudflare's binding model
2. Identified that 4 of our 8 collapsed toys align with WASI interfaces that CF already supports
3. Realized the component model's portability promise is testable: same WASM, different hosts
4. Asked "how could we connect children to Workers?" and the answer was: a child IS a Worker if the interfaces align

## Cloudflare Architecture Comparison

Cloudflare Workers has ~8 binding types (KV, R2, D1, Queues, Service Bindings, Durable Objects, fetch, Secrets). Millions of Workers across every domain use combinations of these 8 primitives. The design is capability-based security through opaque handles:

| Cloudflare Pattern | Patina Equivalent |
|---|---|
| `wrangler.toml` declares bindings | `child.toml` declares toys |
| `env.MY_KV` — opaque handle, Worker never sees namespace ID | `http::request("github", ...)` — opaque handle, child never sees endpoint URL |
| `env.GITHUB_TOKEN` — secret injected by runtime | Mother injects auth headers host-side |
| Worker starts with zero access | Child starts with zero capabilities |
| Read `wrangler.toml` = complete security audit | Read `child.toml` = complete security audit |

### Where Patina's Security Model Is Stronger

In Cloudflare's model, a Worker **sees the secret value**. `env.GITHUB_TOKEN` is a string the Worker reads and uses to construct auth headers. A malicious or buggy Worker can log, leak, or exfiltrate that token.

In Patina's model, the child **never sees the credential**:

```
Child calls:    http::request("github", { method: "GET", url: "/repos", headers: [], body: None })
                     ↓
Mother resolves: "github" → https://api.github.com
Mother injects:  Authorization: Bearer <pat>  (host-side, outside WASM)
Mother executes: makes the HTTP call
Mother returns:  response body to child (without auth headers)
```

The credential never enters WASM memory. The child can USE the connection (within its granted scope) but cannot STEAL the credential. Even a compromised child binary can only make authorized requests to granted endpoints — it has no token to exfiltrate.

This is the strongest property of the toybox model: **connection-name handles, not credential strings.**

### What This Means for the Portable Child

The portable-fetcher child proves this works across runtimes:
- On Patina: `http::request("api", ...)` → Mother injects credentials host-side
- On Cloudflare: same WIT import → CF adapter reads `env.API_TOKEN` and injects it

The child code is identical. The credential handling differs per runtime. The child never knows.

## The Portability Map

The child can only use toys that have equivalents on both runtimes:

| Patina Toy | Cloudflare Binding | WASI Interface | Portable? |
|---|---|---|---|
| http | `fetch()` | `wasi:http` | Yes |
| state | KV | `wasi:keyvalue` | Yes |
| log | `console.log` / Workers Analytics | `wasi:logging` | Yes |
| fs | R2 (object storage) | `wasi:filesystem` | Partial — different semantics |
| store | D1 (SQLite) | — | Maybe — D1 is SQL, our store is generic |
| events | Queues | — | Maybe — different ack/offset model |
| task | — | — | No CF equivalent |
| peer | Service Bindings | — | Different model |
| git | — | — | No CF equivalent |

**The portable subset: http, state, log.** These are the safest starting point. The proof child should use only these three toys.

## Target Shape

### The child: a simple data fetcher

A child that:
1. Fetches data from an external API via `http` toy
2. Caches results in `state` toy
3. Logs what it did via `log` toy

Simple enough to build quickly. Complex enough to prove the point.

### On Patina

```toml
# child.toml
name = "portable-fetcher"
kind = "knowledge-child"

[needs]
toys = ["http", "log", "state"]

[needs.toys.api]
type = "http"

[provides]
child = "portable-fetcher"
```

Runs under Mother. Mother grants http/state/log toys, resolves the "api" connection.

### On Cloudflare

```toml
# wrangler.toml
name = "portable-fetcher"
main = "target/wasm32-wasip2/release/portable_fetcher.wasm"
compatibility_date = "2026-03-26"

[[kv_namespaces]]
binding = "STATE"
id = "..."

[vars]
API_URL = "https://api.example.com"
```

Runs on Cloudflare. CF runtime provides `fetch()` for http, KV for state, console for log.

### The child code (same for both)

```rust
use patina_sdk::prelude::*;

struct PortableFetcher;

impl KnowledgeChild for PortableFetcher {
    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "fetch" => {
                log::info("starting fetch");
                let response = http::request("api", &HttpRequest {
                    method: "GET".into(),
                    url: "/data".into(),
                    headers: vec![],
                    body: None,
                })?;
                state::set("last-fetch", &response.body)?;
                log::info("fetch complete, cached in state");
                Ok(response.body)
            }
            "cached" => {
                state::get("last-fetch").ok_or("no cached data".into())
            }
            _ => Err(format!("unknown action: {}", action)),
        }
    }
}
```

Zero runtime detection. The child doesn't know if "api" is resolved by Mother or by Cloudflare. It just calls toys.

## Solution

### Step 1: Build the child using patina-sdk with collapsed toys

After the toy collapse lands, build `children/portable-fetcher/` using http, state, log toys only.

### Step 2: Verify it runs under Mother

Load via `child.toml`, grant toys, call `handle("fetch", ...)` and `handle("cached", ...)`. Verify http→state→log flow works.

### Step 3: Build a Cloudflare adapter layer

The gap: Cloudflare's runtime provides `fetch()`, `KV`, and `console` — not Patina WIT interfaces. We need a thin adapter that maps CF bindings to our WIT imports.

Options:
- **Option A**: Use `cargo-component` + Cloudflare's WASM component support (if CF supports WIT imports natively by then)
- **Option B**: Write a JS Worker wrapper that imports the WASM component and bridges CF bindings to the WIT interface calls
- **Option C**: Use `wasi-virt` to create a virtualizing component that maps WASI interfaces to CF bindings

The right option depends on CF's component model support state at execution time.

### Step 4: Deploy to Cloudflare and verify same behavior

Deploy via `wrangler`. Call the same actions. Verify same outputs for same inputs.

### Step 5: Document the portability guide

What worked, what didn't, what the adapter layer looks like, what toys are portable, what the limitations are.

## Implementation Order

Blocked on `toy-collapse-wasi-alignment` — the child must be built against collapsed toy interfaces.

1. Build the child (after toy collapse)
2. Verify on Patina
3. Research CF component model support at that time
4. Build adapter layer
5. Deploy and verify on CF
6. Document

## Resolved Decisions

- Start with the narrowest portable toy subset: http, state, log. Don't try to map store or events until the basic proof works.
- The child MUST NOT detect its runtime. If it needs `#[cfg(...)]` for Patina vs CF, the portability claim fails.
- The adapter layer (if needed) lives outside the child. The child is pure. The adaptation is infrastructure.

## Verification

```bash
# Patina side:
cargo build --target wasm32-wasip2 -p portable-fetcher
patina child load portable-fetcher
patina child run portable-fetcher fetch
patina child run portable-fetcher cached
# Outputs should match

# Cloudflare side:
wrangler deploy
curl https://portable-fetcher.workers.dev/fetch
curl https://portable-fetcher.workers.dev/cached
# Outputs should match Patina side
```

## Build Readiness

Blocked on `toy-collapse-wasi-alignment`. The child must be written against the collapsed 8-toy interface set. Also depends on Cloudflare's WASM component model support at execution time — research needed during Step 3.
