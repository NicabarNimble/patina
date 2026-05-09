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

## 3) Bridge policy observability

The archived Atlas prototype no longer reports lane visibility. Future Mother view-composer data catalog collectors should expose these same deterministic bridge policy signals:

- children with legacy aliases show `lane_hint: "legacy-bridge-lane"`
- typed children show `lane_hint: "typed-manifest-lane"`

## 4) Full build guard

```bash
cargo check -q
```
