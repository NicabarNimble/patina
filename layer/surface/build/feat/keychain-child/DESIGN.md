# Design: Extract Keychain to opt-in native child

## Why This Design

Keychain integration pulls 9 platform-specific crates into every binary,
including Linux where it's dead code. Empirical testing proved it never works
over SSH. Making it opt-in via the child architecture eliminates the supply
chain surface while preserving the feature for console users who want it.

The child architecture already exists for WASM children. This spec may be the
first native child, which means the native child runtime pattern needs to be
defined (possibly in a separate spec if the interface design is complex).

## Build Target

New `children/keychain-macos/` crate. `IdentityBackend` trait in Mother.
Storage layer uses trait dispatch. ~200 lines of code movement (not new logic).

## Resolved Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Trait location | `mother/src/secrets_authority_backend/` | Colocated with storage |
| Child binary | Shared library (.dylib) or subprocess | TBD — depends on native child runtime |
| Discovery | File-based (children/ directory scan) | Matches WASM child pattern |

### Keychain code to extract

Current location: `mother/src/secrets_authority_backend/keychain.rs` (167 lines)

```rust
// Platform-specific code that moves to child:
mod platform {
    pub fn store_identity(identity: &str) -> Result<()>  // SecItemAdd
    pub fn get_identity() -> Result<String>               // SecItemCopyMatching
    pub fn delete_identity() -> Result<()>                // SecItemDelete
    pub fn has_identity() -> bool                         // SecItemCopyMatching check
}
```

Constants: `KEYCHAIN_SERVICE = "patina"`, `KEYCHAIN_ACCOUNT = "Patina Secrets"`

Dependencies to extract:
- `security-framework` (3.6.0) — high-level Keychain API
- `security-framework-sys` (2.16.0) — raw FFI
- `core-foundation` (0.10.1) — CFString, CFData, CFDictionary
- `core-foundation-sys` (0.8.7) — raw FFI
- `bitflags` (2.11.0) — used by security-framework

### Beliefs informing design

- `[[keychain-never-worked-ssh]]` — -25308 over SSH, all approaches failed
- `[[keychain-always-this-device-only]]` — correct policy when used
- `[[raw-keychain-over-access-control]]` — use raw SecItemAdd, not
  SecAccessControlCreateWithFlags (fails on macOS 26 Tahoe)

## Commits

1. `feat(mother): define IdentityBackend trait` —
   Trait definition in storage module. Initially no children registered.
   Storage.rs updated to check backends before encrypted file.

2. `feat(children): create keychain-macos child crate` —
   Move keychain.rs code to `children/keychain-macos/src/lib.rs`.
   Implement `IdentityBackend` trait. Add to workspace members.

3. `feat(mother): native child discovery for identity backends` —
   Mother scans for registered native children at startup. keychain-macos
   registers itself as an identity backend.

4. `refactor(mother): remove keychain from core` —
   Delete `mother/src/secrets_authority_backend/keychain.rs`.
   Remove security-framework, core-foundation from mother/Cargo.toml.
   Update storage.rs: remove direct keychain calls, use trait dispatch.

## Direct Code Targets

### Commit 1
- NEW: `mother/src/secrets_authority_backend/identity_backend.rs` — trait definition
- `mother/src/secrets_authority_backend/storage.rs:41-82` — add trait dispatch

### Commit 2
- NEW: `children/keychain-macos/Cargo.toml`
- NEW: `children/keychain-macos/src/lib.rs` — moved from keychain.rs
- `Cargo.toml` (workspace) — add to members

### Commit 3
- `mother/src/secrets_authority_backend/mod.rs` — child registration
- TBD: native child discovery mechanism

### Commit 4
- DELETE: `mother/src/secrets_authority_backend/keychain.rs`
- `mother/Cargo.toml` — remove security-framework, core-foundation
- `mother/src/secrets_authority_backend/storage.rs` — remove direct keychain imports

## Verification Plan

```bash
# Without child
cargo tree -p mother | grep security-framework  # nothing
patina secrets                                   # works (encrypted file)
patina secrets --export-key --stdout --confirm   # works

# With child (after installation)
patina child add keychain-macos
patina secrets                                   # identity via Keychain if console
```

## Build Readiness

- [x] Keychain code analyzed (167 lines, 4 functions)
- [x] Platform deps enumerated (9 crates)
- [x] Session history documenting Keychain limitations
- [ ] Native child runtime pattern defined
- [ ] IdentityBackend trait reviewed
- [ ] Blocked by vault-mother-consolidation

## Open Questions

1. **Native child runtime**: WASM children use a well-defined runtime (load .wasm,
   call exports). Native children need a different mechanism: shared library
   (.dylib/.so), subprocess with IPC, or compiled-in feature flag. This is
   a significant design decision that may deserve its own spec.
   **Recommendation**: Start with a feature flag (`--features keychain`) for
   simplicity. Full dynamic native child loading is a larger scope.

2. **Auto-migration from Keychain**: When the child is installed and a user has
   an identity in Keychain but not in encrypted file, should the child
   auto-migrate to encrypted file (making Keychain a write-through cache)?
   **Recommendation**: Yes — this matches the existing auto-migration behavior
   in storage.rs. Encrypted file is always the canonical store.

3. **Touch ID integration**: If Keychain is opt-in, should Touch ID prompts be
   supported? This was the original motivation for Keychain, but it conflicts
   with the decrypt-on-demand model (no prompts).
   **Recommendation**: No Touch ID in the child. The child provides an alternative
   storage backend, not a gating mechanism. Touch ID was the source of the
   problems documented in the session history.
