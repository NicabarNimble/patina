# Demo: Spec Atlas + MCT Visibility

This demo uses only local repository truth (no Mother daemon required).

## 1) Generate JSON snapshot

```bash
cargo run -q -- atlas --json > .tmp/atlas/spec-atlas.json
```

Expected shape:
- `summary` (spec/edge/child/toy counts + lane/status counts)
- `specs` with criteria progress (`checked/total`)
- `spec_edges` from `blocked_by` + resolvable `related`
- `children` inventory from `children/*/child.toml`
- `toys` inventory from `wit/toys/deps/toys-registry.toml`

Quick check:

```bash
python3 - <<'PY'
import json
with open('.tmp/atlas/spec-atlas.json') as f:
    d=json.load(f)
print('specs', len(d['specs']))
print('children', len(d['children']))
print('toys', len(d['toys']))
print('edges', len(d['spec_edges']))
print('statuses', d['summary']['status_counts'])
print('lanes', d['summary']['lane_counts'])
PY
```

## 2) Generate standalone HTML dashboard

```bash
cargo run -q -- atlas --html --output .tmp/atlas/spec-atlas.html
```

Open in browser:

```bash
open .tmp/atlas/spec-atlas.html
```

## 3) Walkthrough checklist

1. **Summary cards**
   - spec count / edge count / child count / toy count
2. **Specs table**
   - filter by status and lane
   - inspect criteria progress (green/amber/red)
3. **Spec edges**
   - verify blockers and related links are visible
4. **Children table**
   - see kind/role, toy grants, lane hints, typed-vs-handle indicators
5. **Toys table**
   - inspect registry IDs, source, version, file

## 4) Deterministic tests for this slice

```bash
cargo test -q atlas
cargo check -q
```
