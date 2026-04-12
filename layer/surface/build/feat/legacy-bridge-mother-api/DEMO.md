# Demo: Legacy Bridge Mother API

## 1) Start Mother

```bash
patina mother start
```

## 2) Allow verdict

```bash
curl -s --unix-socket ~/.patina/run/serve.sock \
  -H 'Content-Type: application/json' \
  -d '{"action":"dispatch","legacy_toys":["log","state"],"payload":null}' \
  http://localhost/api/bridge/translate | jq
```

Expected:
- `verdict: "allow"`
- `typed_toys` includes `logging`, `keyvalue`

## 3) Deny verdict (fail-closed)

```bash
curl -s --unix-socket ~/.patina/run/serve.sock \
  -H 'Content-Type: application/json' \
  -d '{"action":"dispatch","legacy_toys":["log","nope"],"payload":null}' \
  http://localhost/api/bridge/translate | jq
```

Expected:
- `verdict: "deny"`
- `denied_toys: ["nope"]`

## 4) Verification

```bash
cargo check -q
cargo test -q -p mother bridge_translate
```
