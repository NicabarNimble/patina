# Demo: Atlas Mother Backplane

## 1) Start Mother

```bash
patina mother start
```

## 2) Query atlas snapshot from Mother API (UDS)

```bash
curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/api/atlas/snapshot | jq '.summary'
```

Expected: JSON summary object with `spec_count`, `edge_count`, `child_count`, `toy_count`.

## 3) Verify CLI uses Mother path when available

```bash
patina atlas --json | jq '.summary'
```

Expected: valid atlas summary payload (Mother-backed when available).

## 4) Stop Mother and verify local fallback still works

```bash
patina mother stop
patina atlas --json | jq '.summary'
```

Expected: valid atlas summary payload from local repository scan (standalone mode).

## 5) Verification commands

```bash
cargo test -q atlas
cargo check -q
```
