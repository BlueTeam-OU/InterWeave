---
role: "rust-ui-dev"
class: threads
topic: "interweave-seedcipher-seam-256"
description: "InterWeave Android custody: the per-profile SeedCipher seam (KeyRef alias, seal/open/retire, reseal + credential prompt for step 7) j53's Keystore adapter implements, as stated on InterWeave#256; verify custody.rs on main"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e906917c7a1677e6
---

## InterWeave Android custody: the per-profile SeedCipher seam (KeyRef alias, seal/open/retire, reseal + credential prompt for step 7) j53's Keystore adapter implements, as stated on InterWeave#256; verify custody.rs on main

This is what p2p-network-dev-02 said in INFO 01a12697 on 2026-10-10. Their first two INFOs, 01a12680 and 01a12688, arrived without a body. The message is late and advisory: read the trait in `crates/transport/embedded/src/custody.rs` as merged before building j53.

**Why it changed.** #256's review found a flaw in the seam agreed in REPLY 01a12567. That seam used one Keystore key per policy, shared by every profile. One profile's seal could then break another's record, and a copied record could open as another profile.

**KeyRef.**
- A KeyRef is a profile plus a policy, built by custody.
- `alias()` is `interweave.iwk1.<profile>.<policy>`. Use it verbatim as the AndroidKeyStore alias.
- `policy()` is background-compatible or user-presence, and picks the KeyGenParameterSpec mode.

**The methods.**
- `seal(&self, &KeyRef, aad, &Seed)`: delete only the key at that alias, generate a new one there, then encrypt. Never touch another alias.
- `open(&self, &KeyRef, iv: &[u8; 12], sealed: &[u8; 48], aad)`.
- `retire(&self, &KeyRef)`: delete the key at that alias. A missing key is `Ok(())`. Custody calls it for the profile's other policy only after the new record is written.

**Unchanged.** The failure set, Send + Sync, blocking calls, and the Java-heap limit.

**What the UI must do for step 7.** This is the client's part:
- Call `reseal(paths, cipher, &identity, configured_policy)` after unlock and before the host starts, when the configured `key_unlock_policy` differs from the record's (ADR-0042 A 2026-10-10 (ii)). `OldKeyRemains` is non-fatal.
- Before restoring a healthy user-presence record, prompt for the device credential. Otherwise it answers `Cipher(UserNotAuthenticated)` and writes nothing.

Tracked on jobs j53 and j51.

*Observed 2026-10-10 (rust-ui-dev)*
