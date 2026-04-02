# Design: Build a Patina child that runs as a Cloudflare Worker

## Why This Design

The toy collapse spec and session discoveries claim Patina's architecture aligns with Cloudflare Workers. Claims need proof. The strongest proof is: compile one child, run it on two runtimes, get the same behavior. If that works, the toybox model is genuinely portable. If it doesn't, we learn exactly where the alignment breaks.

This is also the most compelling demo for the project: "this WASM child runs on our platform AND on Cloudflare's edge network, unchanged."

## Build Target

One child (`portable-fetcher`), one WASM artifact, two runtimes, one test fixture to verify identical behavior.

## Resolved Decisions

- Use only the portable toy subset: http, state, log. These have clear CF equivalents.
- The child code is pure — zero runtime detection. Adaptation is infrastructure, not application.
- JS Worker wrapper (Option B) is likely the most pragmatic adapter for CF, unless their native component model support has matured.

## The Adapter Problem

Patina and Cloudflare both implement capability-based security through opaque handles. But the handle shapes differ:

| Toy | Patina WIT call | Cloudflare JS call |
|-----|----------------|-------------------|
| http | `http::request("api", req)` | `await fetch(url, init)` |
| state | `state::get("key")` | `await env.KV.get("key")` |
| log | `log::log(level, msg)` | `console.log(msg)` |

The adapter must bridge: Patina WIT imports → CF JS bindings. Two approaches:

**JS wrapper (most likely):**
```js
// worker.js — thin bridge
import { instantiate } from './portable_fetcher.wasm';

export default {
  async fetch(request, env) {
    const child = await instantiate({
      'patina:http/http': {
        request: async (connection, req) => {
          // Map connection name to real URL from env vars
          const baseUrl = env[`${connection.toUpperCase()}_URL`];
          const response = await fetch(`${baseUrl}${req.url}`, {
            method: req.method,
            headers: req.headers,
            body: req.body,
          });
          return { status: response.status, headers: [...], body: await response.text() };
        }
      },
      'patina:state/state': {
        get: (key) => env.STATE.get(key),
        set: (key, value) => env.STATE.put(key, value),
        delete: (key) => env.STATE.delete(key),
        list: (prefix) => env.STATE.list({ prefix }),
      },
      'patina:log/log': {
        log: (level, message) => console.log(`[${level}] ${message}`),
      },
    });

    // Route HTTP request to child's handle() method
    const url = new URL(request.url);
    const action = url.pathname.slice(1); // "/fetch" → "fetch"
    const result = child.handle(action, await request.text());
    return new Response(result);
  }
}
```

**Native component model (if CF supports it):**
No JS wrapper needed. CF runtime directly satisfies WIT imports via binding configuration. This is the ideal end state.

## Commits

1. `feat(children): scaffold portable-fetcher child` — Create `children/portable-fetcher/` with Cargo.toml, child.toml, src/lib.rs. Uses only http, state, log toys. Build for wasm32-wasip2.

2. `test(portable-fetcher): verify on Patina` — Load under Mother, run fetch/cached actions, verify toy flow.

3. `feat(portable-fetcher): add Cloudflare adapter` — Create `deploy/cloudflare/` with wrangler.toml and JS wrapper (or native component config). Wire CF bindings to WIT imports.

4. `test(portable-fetcher): verify on Cloudflare` — Deploy via wrangler, run same actions, verify same outputs.

5. `docs: portable children guide` — How to build children that run on Patina and CF. Toy→binding map. Limitations. Adapter patterns.

## Direct Code Targets

- `children/portable-fetcher/` — new child crate
- `children/portable-fetcher/child.toml` — Patina manifest
- `deploy/cloudflare/wrangler.toml` — CF manifest
- `deploy/cloudflare/worker.js` — adapter (if JS wrapper approach)
- `docs/` or `sdk/` — portability guide

## Verification Plan

```bash
# Build once
cargo build --target wasm32-wasip2 -p portable-fetcher

# Test on Patina
patina mother start
patina child run portable-fetcher fetch '{"url": "/test"}'
patina child run portable-fetcher cached
# → capture output A

# Test on Cloudflare (local dev)
cd deploy/cloudflare && wrangler dev
curl http://localhost:8787/fetch
curl http://localhost:8787/cached
# → capture output B

# Compare: output A == output B (same data, same structure)
```

## Build Readiness

Blocked on `toy-collapse-wasi-alignment`. Also needs research into Cloudflare's current WASM component model support at execution time.

## Open Questions

- Does Cloudflare's `workerd` runtime support WIT imports natively yet? If yes, the JS wrapper is unnecessary. If no, the wrapper is the bridge.
- Should the adapter live in this repo or in a separate `patina-cf-adapter` repo?
- Can we use `wasi-virt` to create a virtualizing component instead of a JS wrapper?
- What's the right test fixture? A mock HTTP API that both runtimes can hit, returning deterministic data.
