---
type: explore
id: mother-password-unlock
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-064905-376539000
exit_criteria:
  - Decision on password-based vs machine-bound identity
  - Key wrapping approach chosen (Argon2/scrypt + ChaCha20)
  - Session unlock lifecycle designed
  - Migration path from identity.enc to identity.wrapped
---
# explore: Mother password-based unlock for vault identity

> Replace machine-bound identity.enc with password-wrapped identity. User sets password once, Mother unlocks per session. Portable, recoverable, follows 1Password model.

## Question

Can we replace the machine-bound identity storage (`identity.enc` encrypted
with hardware machine ID) with a password-wrapped identity that's portable
and recoverable, where Mother manages the unlock lifecycle?

## Findings

### Current model is fragile and non-recoverable

The age identity (private key that decrypts the vault) is stored in
`~/.patina/identity.enc`, encrypted with ChaCha20-Poly1305 using a key
derived from the machine's hardware ID (IOPlatformUUID on macOS,
/etc/machine-id on Linux).

Problems discovered in session 20260402:
- **Not portable** — identity.enc is bound to the machine. New machine = new
  identity = can't decrypt old vault.
- **No recovery** — if identity.enc is lost or machine ID changes (OS reinstall,
  hardware swap), secrets are gone. The user was never prompted to back up.
- **Identity drift** — auto-generation created three different identities across
  sessions without the user knowing. No password = no user checkpoint.
- **Silent setup** — `init_vault` generates the identity silently. User has no
  idea it happened, no idea they should back it up.

### Proposed model: password-wrapped identity

```
Setup (once):
  patina secrets init  (or first vault access)
  → "Set a password for your secrets vault:"
  → Mother generates age identity
  → Mother wraps identity with password-derived key (Argon2id + ChaCha20-Poly1305)
  → Saves ~/.patina/mother/identity.wrapped
  → Password is zeroized from memory after wrapping
  → identity.wrapped is portable — copy to any machine

Daily use:
  patina mother start  (or first vault access)
  → "Vault password:"
  → Mother derives key from password → unwraps identity
  → Holds identity in memory for session
  → All vault operations work without re-prompting
  
Lock:
  patina secrets --lock  (or timeout, or mother stop)
  → Zeroize identity from memory
  → Next vault access re-prompts

Recovery:
  Copy identity.wrapped to new machine
  Enter same password → works. No machine binding.
```

### How this maps to 1Password

| 1Password | Patina (proposed) |
|-----------|-------------------|
| Master password | Vault password |
| Secret key (device-bound, synced) | identity.wrapped (portable file) |
| Vault items | Secrets in vault.age |
| Agent holds keys in memory | Mother holds identity in memory |
| Lock on timeout | `--lock` or timeout |
| Emergency Kit (paper backup) | identity.wrapped file (user copies) |

### What Mother's role becomes

Mother is the sole component that:
1. Asks for the password (UX boundary)
2. Derives the wrapping key (crypto)
3. Holds the unwrapped identity in memory (session state)
4. Performs vault encrypt/decrypt (authority)
5. Zeroizes on lock/stop (cleanup)

The user never sees age keys, recipients, or vault internals. They set one
password and Mother handles everything. CLI remains a thin IPC client.

### Session unlock lifecycle

```
State machine:
  LOCKED → (password entry) → UNLOCKED → (lock/timeout/stop) → LOCKED

LOCKED state:
  - Vault read/write operations fail with "Vault is locked. Run: patina secrets unlock"
  - Status shows "locked"
  - No identity in memory

UNLOCKED state:
  - All vault operations work
  - Identity held in memory (zeroize on transition out)
  - Optional auto-lock timeout (configurable, default 30 min?)

Transitions:
  patina secrets unlock  → prompt → UNLOCKED
  patina secrets --lock  → LOCKED
  patina mother stop     → LOCKED (implicit)
  Timeout                → LOCKED (if configured)
  First vault op when locked → auto-prompt (UX convenience)
```

### Key derivation

Password → wrapping key should use Argon2id (memory-hard, GPU-resistant):
- Argon2id with high memory cost (64MB+)
- Random salt stored alongside wrapped identity
- Output: 256-bit key for ChaCha20-Poly1305

File format for `identity.wrapped`:
```
[PATINA][0x02][salt: 32 bytes][argon2_params: 12 bytes][nonce: 12 bytes][ciphertext+tag]
```

Version 0x02 distinguishes from current identity.enc (version 0x01).

### Dependency impact

- Need `argon2` crate (RustCrypto, well-maintained) — or reuse scrypt if
  already available. Argon2id is the modern recommendation (OWASP, NIST).
- `chacha20poly1305` already in tree
- No new non-crypto deps

### What stays vs what changes

| Component | Change |
|-----------|--------|
| `vault.age` format | Unchanged — still age-encrypted |
| `recipient.txt` | Unchanged |
| `secrets.toml` registry | Unchanged |
| `identity.enc` (machine-bound) | Replaced by `identity.wrapped` (password-bound) |
| Mother vault operations | Add unlock/lock state machine |
| CLI commands | Add `unlock`, modify `--lock` |
| `PATINA_IDENTITY` env var | Still works as CI/headless bypass |

### Migration from identity.enc

If identity.enc exists when user first runs under new model:
1. Auto-decrypt identity.enc (machine-bound, no password needed)
2. Prompt: "Set a password for your secrets vault:"
3. Wrap identity with password → save identity.wrapped
4. Delete identity.enc
5. User now has password-based recovery

One-time, automatic, no data loss.

## Conclusions

**The design is sound.** It follows a proven model (1Password), solves real
problems (portability, recovery, silent key loss), uses the existing vault
format, and aligns with Mother-as-authority. The identity key becomes a
managed resource instead of a hidden implementation detail.

### Open questions for implementation

1. **Auto-lock timeout** — should Mother auto-lock after inactivity? Default?
   Configurable? 1Password defaults to 15 minutes.

2. **Biometric option** — on macOS, could the password be optionally replaced
   by Touch ID? This re-introduces the Keychain dependency we just made
   optional. Maybe: password required for setup, Touch ID for unlock (like
   1Password).

3. **PATINA_IDENTITY compatibility** — the env var bypass still works for CI.
   But should there also be a `PATINA_VAULT_PASSWORD` env var for headless
   unlock? Or is PATINA_IDENTITY sufficient?

4. **Multiple devices** — if user copies identity.wrapped to two machines,
   both can decrypt the same vault.age. But vault.age is local. Vault sync
   is a separate problem (future: cross-Mother sharing).

5. **Password change** — re-wrap identity with new password. Vault.age doesn't
   change (it's encrypted to the age recipient, not the password). Only
   identity.wrapped changes. Clean separation.

### Recommended next step

Promote to feat spec when ready to build. Estimated 2-3 sessions. The
`move-vault-to-mother` refactor (just completed) is the prerequisite — all
vault code now lives in Mother, so the unlock state machine has one place
to live.
