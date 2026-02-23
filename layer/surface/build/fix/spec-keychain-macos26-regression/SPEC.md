---
type: fix
id: spec-keychain-macos26-regression
status: abandoned
created: 2026-02-20
sessions:
- 20260218-225007
- 20260219-083531
- 20260220-120045
related:
- layer/surface/build/fix/spec-secrets-keychain-ssh/SPEC.md
- layer/surface/build/fix/spec-launcher-auth/SPEC.md
beliefs:
- raw-keychain-over-access-control
- keychain-always-this-device-only
---

# fix: Keychain SSH Access via Raw SecItemAdd (macOS 26 Regression)

> Fix SSH keychain access on macOS 26 (Tahoe) by using raw `SecItemAdd`
> with `kSecAttrAccessibleAlwaysThisDeviceOnly`, bypassing the broken
> `SecAccessControlCreateWithFlags` API while preserving the correct
> accessibility policy for headless/SSH workflows.

## Problem

### Timeline of Breakage

**Feb 18, 2026** - Original fix (spec-secrets-keychain-ssh):
- Implemented `kSecAttrAccessibleAlwaysThisDeviceOnly` via `SecAccessControlCreateWithFlags`
- Commits: 1df40fff, a086aeee
- Result: SSH keychain access worked ✅

**Feb 19, 2026** - macOS 26 (Tahoe) breakage:
- `SecAccessControlCreateWithFlags` started returning error -34018 for ad-hoc signed binaries
- Root cause: macOS 26 rejects `SecAccessControlCreateWithFlags` for non-codesigned binaries
- All keychain policies fail with -34018, regardless of which policy constant is passed

**Feb 19, 2026** - Attempted fix (commit 1cca67ed):
- Switched to raw `SecItemAdd` with `kSecAttrAccessible` (correct ✅)
- BUT changed policy from `AlwaysThisDeviceOnly` → `AfterFirstUnlockThisDeviceOnly` (regression ❌)
- Reason given: "Not deprecated (unlike AlwaysThisDeviceOnly)"
- Result: SSH keychain access broke again - requires GUI login after reboot

**Feb 20, 2026** - User report:
- `patina launch` over Tailscale SSH shows login prompt in new sessions
- Long-lived Claude token not being injected from vault
- Vault decryption fails because keychain identity inaccessible over SSH

### The Regression

The Feb 19 fix solved the macOS 26 API breakage but introduced a different
problem by changing the accessibility policy:

| Policy | SSH Access | Reboot Behavior | Deprecation |
|--------|-----------|-----------------|-------------|
| `AlwaysThisDeviceOnly` | ✅ Always | ✅ Works immediately | iOS only (stolen phone) |
| `AfterFirstUnlockThisDeviceOnly` | ⚠️ After GUI login | ❌ Requires login first | Not deprecated |

For a **headless Mac Studio** accessed via SSH/Tailscale:
- `AlwaysThisDeviceOnly` is correct - no GUI login dependency
- `AfterFirstUnlockThisDeviceOnly` breaks the use case - SSH before login fails

The deprecation warning is **iOS-specific** (stolen phone threat model).
A stationary Mac Studio is not a stolen phone. See spec-secrets-keychain-ssh
lines 91-98 for full rationale.

## Solution

Use raw `SecItemAdd` with `kSecAttrAccessibleAlwaysThisDeviceOnly`:

**Combines:**
- ✅ Raw `SecItemAdd` with `kSecAttrAccessible` (bypasses broken `SecAccessControlCreateWithFlags`)
- ✅ `AlwaysThisDeviceOnly` policy (preserves SSH/headless access)

**API Approach:**
```rust
// Declare kSecAttrAccessible (not exported by security-framework-sys)
extern "C" {
    static kSecAttrAccessible: CFStringRef;
}

// Use raw SecItemAdd, bypassing SecAccessControlCreateWithFlags
let keys = unsafe {
    [
        CFString::wrap_under_get_rule(kSecClass),
        CFString::wrap_under_get_rule(kSecAttrService),
        CFString::wrap_under_get_rule(kSecAttrAccount),
        CFString::wrap_under_get_rule(kSecValueData),
        CFString::wrap_under_get_rule(kSecAttrAccessible),
    ]
};
let values: Vec<CFType> = unsafe {
    vec![
        CFString::wrap_under_get_rule(kSecClassGenericPassword).into_CFType(),
        CFString::from(KEYCHAIN_SERVICE).into_CFType(),
        CFString::from(KEYCHAIN_ACCOUNT).into_CFType(),
        CFData::from_buffer(identity.as_bytes()).into_CFType(),
        CFString::wrap_under_get_rule(kSecAttrAccessibleAlwaysThisDeviceOnly)
            .into_CFType(),
    ]
};

let dict = CFDictionary::from_CFType_pairs(&keys.iter().cloned().zip(values).collect::<Vec<_>>());
let status = unsafe { SecItemAdd(dict.as_concrete_TypeRef(), std::ptr::null_mut()) };
```

## Why Both Changes Are Necessary

**Can't use `SecAccessControlCreateWithFlags` on macOS 26:**
- Returns -34018 for ad-hoc signed binaries
- Would require proper code signing (Apple Developer ID)
- Breaks `cargo install` distribution model

**Can't use `AfterFirstUnlockThisDeviceOnly` for SSH:**
- Requires GUI login at least once after boot
- SSH session before login → keychain locked → vault decryption fails
- Breaks spec-launcher-auth token injection

**Must use both:**
- Raw `SecItemAdd` (works on macOS 26)
- `AlwaysThisDeviceOnly` (works over SSH anytime)

## Implementation

### Files Changed

| File | Change |
|------|--------|
| `src/secrets/keychain.rs:40-48` | Update doc comment: AfterFirstUnlock → Always |
| `src/secrets/keychain.rs:57` | Import: `kSecAttrAccessibleAlwaysThisDeviceOnly` |
| `src/secrets/keychain.rs:92` | Use: `AlwaysThisDeviceOnly` in SecItemAdd dict |
| `src/secrets/keychain.rs:101` | Log message: AfterFirstUnlock → Always |
| `src/secrets/keychain.rs:117-119` | Update get_identity() doc comment |

### Migration

Existing identities stored with `AfterFirstUnlockThisDeviceOnly` need
migration. The `store_identity()` function already handles this - it deletes
the old item before adding the new one (lines 69-71).

Users can migrate by:
```bash
# Export old identity, re-import with new policy
patina secrets --export-key --confirm --stdout | patina secrets --import-key

# Or re-run setup (asks for token again)
patina secrets setup-claude
```

The migration can run from any session where the keychain is accessible
(GUI login, screen sharing, or existing SSH session if device was unlocked).

## Testing

### Exit Criteria

1. **Local vault decryption works:**
   ```bash
   patina secrets run -- printenv CLAUDE_CODE_OAUTH_TOKEN
   # Should inject token successfully
   ```

2. **SSH keychain access works (new session before GUI login):**
   ```bash
   # From MacBook Air → Mac Studio via Tailscale SSH
   ssh mac-studio
   cd /path/to/project
   patina launch
   # Should inject token, NOT ask for login
   ```

3. **Migration preserves vault:**
   - Export/import completes successfully
   - Vault decryption still works
   - Same public key before/after migration

4. **Token injection unblocked:**
   - spec-launcher-auth can proceed
   - New tmux sessions get authenticated Claude

### Test Environment

- macOS 26 (Tahoe) on Mac Studio
- Ad-hoc signed binary (`cargo install --path .`)
- SSH access via Tailscale from MacBook Air
- Global vault at `~/.patina/vault.age` with `claude-oauth` secret

## Beliefs Updated

### raw-keychain-over-access-control

**Current state (Feb 19):**
- Says to use `AfterFirstUnlockThisDeviceOnly`
- Says this replaced `AlwaysThisDeviceOnly`
- Attacks relationship is backwards

**Needs update:**
- Should say to use `AlwaysThisDeviceOnly`
- Should clarify: raw SecItemAdd (good) + correct policy (AlwaysThisDeviceOnly)
- The Feb 19 version had the right API, wrong policy

### keychain-always-this-device-only

**Current state (Feb 18):**
- Correctly identifies `AlwaysThisDeviceOnly` as the right policy for SSH
- References the old `SecAccessControlCreateWithFlags` approach

**Needs update:**
- Should reference raw `SecItemAdd` approach (not SecAccessControlCreateWithFlags)
- Should link to this spec as the current implementation
- Attacked-by section should be updated: deprecation concern was addressed but incorrectly

## Related Work

### spec-secrets-keychain-ssh (complete, Feb 18)
- Original problem statement: correct ✅
- Original solution: correct but broke on macOS 26 ❌
- This spec supersedes the implementation, not the rationale

### spec-launcher-auth (active, Feb 18)
- Depends on keychain SSH access working
- Blocked since Feb 19 by the regression
- Unblocked by this fix

### Session Notes
- **20260218-225007**: Original SSH keychain problem
- **20260219-083531**: macOS 26 breakage, attempted fix, introduced regression
- **20260220-120045**: User report, diagnosis, proper fix

## Rollback & Safety

If this breaks:
1. Revert `src/secrets/keychain.rs` to commit 1cca67ed (AfterFirstUnlock)
2. Accessibility: works after GUI login (acceptable for screen-sharing workflow)
3. No data loss - identity and vault unchanged

The proper code signing path (Apple Developer ID) would make
`SecAccessControlCreateWithFlags` work again, but that's a distribution
model change (notarized releases vs `cargo install`). Out of scope.

## Exit Criteria

- [ ] Code changes applied to `src/secrets/keychain.rs`
- [ ] Build and install: `cargo build --release && cargo install --path .`
- [ ] Migration completed: `patina secrets --export-key | --import-key`
- [ ] Local vault test: `patina secrets run -- printenv CLAUDE_CODE_OAUTH_TOKEN`
- [ ] SSH test: New session over Tailscale, `patina launch` works
- [ ] Beliefs updated: raw-keychain-over-access-control, keychain-always-this-device-only
- [ ] Session notes linked: 20260218-225007, 20260219-083531, 20260220-120045
- [ ] Commit with proper message and Co-Authored-By
- [ ] spec-launcher-auth unblocked (can proceed with token injection testing)
