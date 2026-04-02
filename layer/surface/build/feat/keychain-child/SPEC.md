---
type: feat
id: keychain-child
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-064905-376539000
parent: mother-vault-authority
blocked_by:
  - vault-mother-consolidation
exit_criteria:
  - security-framework and core-foundation removed from mother/Cargo.toml
  - cargo tree -p mother shows zero security-framework/core-foundation deps
  - keychain-macos child crate exists under children/
  - Mother can discover and use keychain child for identity storage
  - Without child installed, patina secrets works using encrypted file only
  - With child installed, patina secrets uses Keychain when available
  - Mother's identity resolution order is env var -> child backends -> encrypted file
  - cargo check and all secrets tests pass
---
# feat: Extract Keychain to opt-in native child

> Move macOS Keychain identity storage from core binary to opt-in child. Eliminates security-framework + core-foundation (9 crates) from core.

## Problem

The macOS Keychain integration (`security-framework` + `core-foundation` +
`security-framework-sys` + `core-foundation-sys` + transitive deps = 9 crates)
is compiled into every Patina binary, even on Linux where it's dead code.

Empirical testing (session 20260222-054702) proved Keychain **never works over
SSH** (-25308 policy error). The encrypted file (`identity.enc`) is the primary
and universal identity storage path. Keychain is a console-only optimization
that most users never benefit from.

Compiling platform-specific security framework bindings into the core binary
is a supply chain surface that can be eliminated by making Keychain an opt-in
child.

## Goal

Extract Keychain to a native child crate. Core binary has zero platform-specific
security deps. Users who want Keychain support opt in explicitly.

## Status

Draft. Blocked by `vault-mother-consolidation` (keychain.rs must live in one
place before extraction).

## Non-Goals

- Changing the encrypted file storage mechanism (stays in core)
- Supporting non-macOS Keychain equivalents (Linux secret-service, Windows
  Credential Manager — future children if needed)
- WASM child (Keychain requires FFI to macOS frameworks)

## Target Shape

```
Mother identity resolution order:
  1. PATINA_IDENTITY env var (CI/headless escape hatch)
  2. Registered credential backend children (e.g., keychain-macos)
  3. Encrypted file (~/.patina/identity.enc) — always available

User opt-in:
  patina child add keychain-macos

Without child:
  patina secrets works using encrypted file only
  No security-framework in binary
```

## Solution

### Child crate structure

```
children/keychain-macos/
  Cargo.toml          — depends on security-framework, core-foundation
  src/lib.rs          — implements IdentityBackend trait
```

### Identity backend trait

Mother needs a trait that children can implement:

```rust
pub trait IdentityBackend {
    fn name(&self) -> &str;
    fn has_identity(&self) -> bool;
    fn get_identity(&self) -> Result<String>;
    fn store_identity(&self, identity: &str) -> Result<()>;
    fn delete_identity(&self) -> Result<()>;
}
```

This trait lives in Mother (or patina-protocol). The keychain-macos child
implements it using `security-framework`.

### Child registration

The keychain-macos child registers with Mother as a credential backend.
Mother's `storage.rs` checks registered backends before falling back to
encrypted file:

```rust
pub fn get_identity() -> Result<String> {
    // 1. Check registered credential backends
    for backend in registered_backends() {
        if backend.has_identity() {
            return backend.get_identity();
        }
    }

    // 2. Encrypted file fallback (always available)
    encrypted_file::get_identity()
}
```

### Native child (not WASM)

The keychain-macos child must be a native child because:
- Keychain access requires FFI to macOS Security.framework
- WASM sandbox cannot make FFI calls to host OS security APIs
- The child needs direct access to the macOS Keychain service

This means it follows the native child pattern (if one exists) or defines it.

## Implementation Order

1. Define `IdentityBackend` trait in Mother or protocol crate
2. Create `children/keychain-macos/` crate implementing the trait
3. Add child discovery/registration in Mother's storage layer
4. Move keychain.rs code from Mother to the child crate
5. Remove `security-framework`, `core-foundation` from mother/Cargo.toml
6. Update storage.rs to use trait dispatch
7. Test with and without child installed

## Resolved Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Child type | Native (not WASM) | Keychain requires FFI |
| Trait location | Mother crate | Child depends on Mother's trait |
| Fallback | Encrypted file always available | Universal, no platform deps |
| Resolution order | env → children → encrypted file | Most specific wins |

### Session history context

- **20260222-054702**: "macOS Keychain NEVER worked over SSH" — three approaches
  tested, all failed with -25308. PATINA_IDENTITY env var was the actual path.
- **20260220-142407 through 20260220-155849**: Multiple sessions fighting
  Keychain SSH access. Tried AlwaysThisDeviceOnly, raw SecItemCopyMatching,
  fresh items. All failed.
- **Belief `[[keychain-never-worked-ssh]]`**: Empirically proven.
- **Belief `[[raw-keychain-over-access-control]]`**: Use raw SecItemAdd, not
  SecAccessControlCreateWithFlags (fails on macOS 26).

This history shows Keychain is a marginal feature that caused weeks of
debugging. Making it opt-in is the right call.

## Verification

- `cargo tree -p mother | grep security-framework` — nothing
- `cargo tree -p mother | grep core-foundation` — nothing
- Without child: `patina secrets` works (encrypted file identity)
- With child: `patina secrets` uses Keychain when on macOS console
- `patina secrets --export-key --stdout --confirm` works in both modes
- Linux: no change in behavior (Keychain child not installable)

## Exit Criteria

- [ ] security-framework, core-foundation removed from mother/Cargo.toml
- [ ] Zero platform security deps in `cargo tree -p mother`
- [ ] keychain-macos child crate exists
- [ ] Mother discovers and uses child for identity
- [ ] Works without child (encrypted file only)
- [ ] Works with child (Keychain available)
- [ ] Identity resolution: env → children → encrypted file
- [ ] Tests pass

## Build Readiness

- [x] Keychain limitations documented (6+ sessions of empirical testing)
- [x] Current keychain.rs code analyzed (167 lines in Mother)
- [x] Platform dep count known (9 crates)
- [ ] IdentityBackend trait designed (needs review)
- [ ] Native child runtime pattern defined — **BLOCKER**: if dylib/subprocess
  pattern doesn't exist yet, this spec needs a prerequisite spec for native
  child runtime, OR the decision to use a cargo feature flag instead
- [ ] Blocked by vault-mother-consolidation
