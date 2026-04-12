# Design: Legacy Bridge Mother API

## Endpoint

- `POST /api/bridge/translate`
- auth-gated under existing `/api/*` route policy
- request body: `mother::bridge::BridgeRequest`
- response body: `mother::bridge::BridgeResponse`

## Runtime

Extend `ApiRuntime` with:

- `bridge_translate(request: BridgeRequest) -> Result<BridgeResponse>`

Daemon implementation delegates to `mother::bridge::evaluate_bridge_request`.

## Client

Extend `src/mother/internal.rs` client with:

- `bridge_translate(&BridgeRequest) -> Result<BridgeResponse>`

Transport behavior mirrors existing control-plane methods:

- UDS first when local
- TCP+token fallback

## Fail-closed behavior

- Invalid JSON => `400`
- Unknown aliases => response verdict `deny` with denied aliases included
- No side effects (policy-only read/compute)

## Code targets

- `mother/src/http_routes.rs`
- `mother/src/http_api.rs`
- `mother/src/daemon_bootstrap_config.rs`
- `src/commands/mother/daemon.rs`
- `src/mother/internal.rs`
