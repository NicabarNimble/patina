# Demo: Legacy Typed Bridge Seam

## 1) Bridge policy module tests

```bash
cargo test -q -p mother bridge::tests
```

Expected: pass (mapping success + fail-closed unknown alias).

## 2) Typed bridge child compile + tests

```bash
cargo check -q -p patina-ai-child-legacy-typed-bridge
```

Expected: pass.

## 3) Atlas lane visibility via bridge policy

```bash
patina atlas --json | jq '.children[] | {folder, lane_hint, toys}'
```

Expected:
- children with legacy aliases show `lane_hint: "legacy-bridge-lane"`
- typed children show `lane_hint: "typed-manifest-lane"`

## 4) Full build guard

```bash
cargo check -q
```
