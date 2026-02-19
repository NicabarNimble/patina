---
type: fix
id: spec-secrets-keychain-ssh
status: complete
created: 2026-02-18
shipped: 2026-02-18
commits:
- 1df40fff  # fix(secrets): store identity with AlwaysThisDeviceOnly
- a086aeee  # fix(secrets): migrate identity policy in setup-claude
related:
- layer/surface/build/fix/spec-launcher-auth/SPEC.md
beliefs:
- keychain-always-this-device-only
---

# fix: Keychain Access Policy for SSH Sessions

> Change the macOS Keychain access policy for the age identity from
> `WhenUnlocked` (default) to `kSecAttrAccessibleAlwaysThisDeviceOnly`
> so vault decryption works in SSH sessions without requiring a GUI
> session, Touch ID, or any plaintext secret on disk.

## Problem

`spec-launcher-auth` stored the age identity in macOS Keychain using the
default `set_generic_password` API. The default policy is
`kSecAttrAccessibleWhenUnlocked` — the item is readable only when the
login Keychain is unlocked.

The login Keychain is unlocked by the GUI login process (Touch ID,
password on wake). SSH sessions — even on an actively running machine
with a logged-in user — do **not** unlock the Keychain. When the screen
is locked, the Keychain locks with it.

Result: `patina launch` over SSH from Tailscale/Termius calls
`get_identity()` → `security_framework::get_generic_password()` →
`errSecInteractionNotAllowed`. The error is caught silently in
`try_get_claude_token()`, no token is injected, Claude asks to login.

### What Doesn't Work Over SSH

Touch ID is a **local GUI event rendered by WindowServer**. There is no
protocol for an SSH client (Termius on iOS) to proxy a biometric
challenge to the server's macOS Keychain. The authentication UI needs a
display. SSH has no display.

## Alternatives Rejected

### `PATINA_IDENTITY` env var (env/shell config)
Store the age private key (`AGE-SECRET-KEY-1...`) in `~/.zshenv`. This
is plaintext on disk — if you have the key and the vault file, you can
decrypt all secrets. Storing a private key in a shell config file defeats
the entire point of having a vault.

### `kSecAttrAccessibleAfterFirstUnlock`
Accessible after device first unlock, no Touch ID required, works while
running. Fails after reboot until someone logs in once. For a remote Mac
Studio this means: power outage at 3am → must use Screen Sharing to log
in before any SSH session can decrypt. Better than `WhenUnlocked`, still
the wrong tradeoff for a headless server.

### `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`
Same as above but hardware-bound to the Secure Enclave (cannot be synced
or restored). Correct on the device-binding axis, still wrong on the
reboot axis.

### Plaintext file (`~/.config/patina/claude-token`, permissions 600)
Skips the vault entirely. Straightforward, survives reboots. Functionally
equivalent to `PATINA_IDENTITY` in terms of on-disk exposure. Clients do
not want their secrets in plaintext files.

## Solution

`kSecAttrAccessibleAlwaysThisDeviceOnly`:

| Property | Value |
|----------|-------|
| Accessible when | Always — no user presence required |
| Hardware-encrypted | Yes — M-chip Secure Enclave device key |
| iCloud backup / sync | No — item is device-specific |
| Restore to new Mac | No — must export/import manually |
| Works over SSH | Yes |
| Works post-reboot | Yes |
| Requires Touch ID | No |

The Secure Enclave's device-unique key (burned in at manufacturing) is the
encryption factor. The item is encrypted at rest, only decryptable on this
specific device. This is the same policy used by macOS mail clients, sync
daemons, and other apps that need background secret access.

**Why the iOS deprecation does not apply here:**
Apple deprecated `AlwaysThisDeviceOnly` on iOS because stolen phones can
be rebooted and background-accessible items become readable without the
owner's knowledge. A stationary Mac Studio is not a stolen phone. The
device being "always on" is the expected operating mode, not a threat
scenario. The deprecation is in `ProtectionMode` enum of the
`security_framework` crate (iOS lineage) — the underlying API still works
on macOS and the constant is present in `security-framework-sys`.

## Implementation

### Calling the API

`kSecAttrAccessibleAlwaysThisDeviceOnly` is absent from
`security_framework::access_control::ProtectionMode` (not exposed due to
iOS deprecation). Access it directly from `security_framework_sys`:

```rust
use security_framework::access_control::SecAccessControl;
use security_framework_sys::access_control::{
    kSecAttrAccessibleAlwaysThisDeviceOnly, SecAccessControlCreateWithFlags,
};

let ac = unsafe {
    use core_foundation::base::TCFType;
    let ac_ref = SecAccessControlCreateWithFlags(
        std::ptr::null(),                                    // kCFAllocatorDefault
        kSecAttrAccessibleAlwaysThisDeviceOnly as *const _, // CFStringRef → CFTypeRef
        0,                                                   // no user-presence flags
        std::ptr::null_mut(),
    );
    if ac_ref.is_null() {
        return Err(anyhow::anyhow!("SecAccessControlCreateWithFlags returned null"));
    }
    SecAccessControl::wrap_under_create_rule(ac_ref)
};
```

`TCFType` must be in scope for `wrap_under_create_rule`. Add
`core-foundation = "0.10"` under `[target.'cfg(target_os = "macos")'.dependencies]`
in `Cargo.toml` — it is already a transitive dep, this makes it explicit.

### Delete Before Add

`set_generic_password_options` (which calls `SecItemAdd`) fails with
`errSecDuplicateItem` if the item exists. Delete the old item first:

```rust
let _ = delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);
```

Ignore errors — if the item doesn't exist (fresh install), the delete
fails silently and the add proceeds. This also handles the migration case:
existing items stored with the wrong policy are deleted and re-added.

### Migration in `setup-claude`

Existing installations have the identity stored with the old `WhenUnlocked`
policy. Calling `store_identity` again re-stores with the correct policy.
The migration is triggered in `setup_claude()` after saving the token:

```rust
if let Ok(key) = secrets::export_identity() {
    let _ = secrets::import_identity(&key);
}
```

`export_identity` reads from Keychain (accessible in the interactive session
where setup-claude runs), `import_identity` validates and re-stores with the
new policy. The migration is silent — no output on success, errors ignored
(if identity isn't set up yet on a fresh machine, the vault init path via
`add_secret` → `init_vault` → `store_identity` already uses the new policy).

## Files Changed

| File | Change |
|------|--------|
| `Cargo.toml` | Add `security-framework-sys = "2.16"` and `core-foundation = "0.10"` under macOS deps |
| `src/secrets/keychain.rs` | `store_identity`: use `AlwaysThisDeviceOnly` via raw FFI |
| `src/commands/secrets.rs` | `setup_claude`: add identity migration after token store |

## Security Model

After this change, the age identity lives in Keychain encrypted by the
Mac Studio's Secure Enclave hardware key. The threat model:

| Threat | Protection |
|--------|-----------|
| Someone copies `~/.patina/vault.age` to another machine | Cannot decrypt — age identity not present |
| Someone copies Keychain database to another machine | Cannot decrypt — `ThisDeviceOnly` items use device-specific Secure Enclave key |
| Someone SSHes in as the user | Can decrypt vault (they have full user access — same as local) |
| Someone steals the Mac Studio | Physical device = decryption access. Same as any unencrypted disk. Use FileVault. |
| Mac reboots, user SSHes in before login | Decrypts fine — `AlwaysThisDeviceOnly` requires no login |

FileVault provides the disk-at-rest protection layer. The Keychain policy
provides the "accessible to background processes but hardware-bound"
layer.

## Machine Migration

`ThisDeviceOnly` means the Keychain item cannot auto-migrate to a new Mac
via iCloud or Migration Assistant. Manual steps required:

```bash
# On old Mac (must have Keychain access):
patina secrets --export-key --stdout --confirm > identity.age
# Copy identity.age to new Mac, then:
patina secrets --import-key < identity.age
rm identity.age
# Also copy ~/.patina/vault.age to new machine
```

The export/import CLI still works — `AlwaysThisDeviceOnly` items are
readable on the device that stores them. The restriction is on
automatic transfer, not manual export.

## Stale Documentation

`spec-launcher-auth/SPEC.md` Platform Support table had:
> macOS (SSH): Touch ID may not work over SSH; session cache avoids it
> Linux: PATINA_IDENTITY env var

The macOS SSH row is now resolved by this fix — no session cache or env
var needed for SSH on macOS. The Linux row is unchanged.
