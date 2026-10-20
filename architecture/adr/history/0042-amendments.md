# ADR-0042 — amendment history

### Amendment 2026-10-10 — The IWK1 v1 envelope is the identity record's format; the platform cipher is a seam with a closed failure set; SPIKE-009's client-side clauses are steps 7–8's tests

**Trigger.** SPIKE-009 closed PASS on 2026-10-09 and its Result 5 adopted the IWK1 v1 envelope as the Android identity envelope format, saying the contract text — this amendment and `android-key-custody.md` — lands with §20 step 6's batch. p2p-network-dev-02 built the custody path on `feat/android-keystore-custody` (61941264: `crates/transport/embedded/src/custody.rs`, fifteen host tests over real files with a software AES-GCM seam) and named the facts the record states (01a1258c).

**Ruled.** The Decision gains the format and the seam: the 66-byte IWK1 v1 envelope with its associated data and the PeerId re-derived on unwrap; one record file `identity.iwk1` = envelope `|` PeerId (UTF-8, ≤ 128 bytes), exclusive-create at provisioning under the profile lock (provisioning and restore take it without waiting before anything seals, answering `ProfileLocked`; a lock, not a reservation file, since the kernel releases it when the holder dies — a343af4b, after the supply's review found the seal-before-create race), a second provisioning refused; the `SeedCipher` seam (`Send + Sync`, blocking calls allowed) whose failure set is closed and contract — `KeyInvalidated`, `KeyMissing`, `UserNotAuthenticated`, `Authentication`, `Unavailable(String)` — the fifth added so a transient platform error is returned to the caller for its log and decides nothing rather than being forced into one of the four verdicts; unlock → identity or one of `Unprovisioned`, `RecoveryRequired(cause)`, `UserNotAuthenticated`, `Unavailable`, only `RecoveryRequired` entering recovery, none minting, the record unchanged on a refused key and the identity back only through the phrase (the stage gate's client-side clause at the custody path: `an_invalidated_or_missing_key_requires_recovery_and_mints_nothing`); `seal` rotating the Keystore alias; zeroization as a stated limit; the policy-swap bit flip refused as `KeyMissing` or an authentication failure. The Implementation implications' "SPIKE-009 validates …" reads as closed, and the clauses the spike did not establish (among them StrongBox/API 34+ reporting and the D6b re-run; `SPIKES.md` lists all) are carried to steps 7–8 by name — the custody doc's "must verify" sentence likewise. The Keystore implementation of the seam and the instrumented device test are rust-ui-dev's app-shell batch (architect-cto 01a1255f), the §20 Keystore device row carried to it.

**Not changed.** The two unlock policies and their restart diagnostic; the no-HSM claim; recovery tooling offline.

**Keyed per profile (2026-10-10, #256's review F1, P2; judged real by a second review-class pass).** The seam's key was one per policy, shared by every profile: sealing one profile rotated the key under which every other profile's record was sealed, and a record copied from another profile's directory unlocked as that profile, since the PeerId it is checked against is read from the record. Nothing in the tree reached it (the app runs one fixed profile), but the custody API promised an isolation it could not give. Fixed in cccc175a: `KeyRef = (profile, policy)` with the alias `interweave.iwk1.<profile>.<policy>` computed by custody; `seal`/`open` take a `&KeyRef`; `seal` deletes every key of its profile and no other profile's, then generates the new one — which also retires the old record on a restore under the other policy (F3); restore decides whose record it is from the PeerId field alone and refuses a link as `Unreadable` (F2). The envelope, its associated data and the failure set are unchanged. Ruled with it, for step 7's caller: the record's authenticated policy byte governs opening; a configured-policy mismatch is re-sealed under the configured policy after a successful unlock, never refused.

**Retire after the write (2026-10-10, #256's re-review R1, P2, reproduced by a test before the fix).** As first fixed, a seal deleted every key of its profile before the new record was written, so a reseal or cross-policy restore whose write failed left the record on disk with no key and sent a working profile to recovery. d3c67b26: `seal` replaces only the key at its own alias; the seam gains `retire(&KeyRef)` (no key there is success); custody retires the profile's other-policy key only after the new record is written; a failed retire answers `CustodyRefused::OldKeyRemains` — the record is written and opens, a pre-change copy may still open until the key is deleted, and `reseal` retries the deletion even when the policy already matches. Tests: `a_write_that_fails_after_the_seal_leaves_the_record_openable`, `an_old_key_that_cannot_be_retired_is_said_and_reseal_retries_it`.

**Restore keeps a same-policy record that still opens (2026-10-10, #256's second re-review, pre-existing against the retire-after-write sentence).** A restore under the policy the record already had sealed over the key that record opened under, so a failed write stranded a healthy record. 3f2f5414: restore first tries such a record — kept if it opens to the expected PeerId (bytes rewritten owner-only if readable by others), the seal losing nothing if its key is invalidated, missing or another's, `CustodyRefused::Cipher` and no write if the key cannot be asked; test `a_same_policy_restore_keeps_a_record_that_still_opens`, four mutations each caught. The seam is seal, open, retire (S3).

**Alias versus key (2026-10-10, #256's third re-review, P3).** "Never reuses the invalidated alias" said the wrong noun: the alias `interweave.iwk1.<profile>.<policy>` is kept across a restore; what is never reused is the invalidated key under it, deleted and regenerated at that alias. Corrected in both documents. The same review's risk for step 7 is recorded in the custody doc: a restore of a healthy user-presence record opens it first and so needs an authentication window.

### Amendment 2026-10-11 — The user-presence key is a timed key with a 60 s window

**Trigger.** Plan §20 step 6 builds the Keystore adapter
(`KeystoreCipher.kt`, rust-ui-dev's batch-2 branch). SPIKE-009 ran its
user-presence key as a timed key with the window as a parameter (D1:
"credential or Class-3 biometric, 60 s"); the Decision, the custody
document and the spike's Result named no value, and the adapter carried
a 30 s placeholder. rust-ui-dev asked what window the key carries and
whether `AUTH_DEVICE_CREDENTIAL | AUTH_BIOMETRIC_STRONG` is the set
(QUESTION 01a129e3-376e-7c6c-b6bc-d476f2f016ee).

**What changed.**

- The Decision names the mode: a timed key,
  `setUserAuthenticationRequired(true)` and
  `setUserAuthenticationParameters(USER_PRESENCE_WINDOW_S,
  AUTH_DEVICE_CREDENTIAL | AUTH_BIOMETRIC_STRONG)`. The set was already
  the custody document's "current device credential/strong biometric
  authorization"; it is now in the record. A Class 2 biometric
  (`BiometricManager.Authenticators.BIOMETRIC_WEAK`) has no Keystore
  authenticator type, and the custody document never admitted it.
- `USER_PRESENCE_WINDOW_S` = 60 s, the value SPIKE-009 exercised on the
  device: D1 generated it, D4 (process) showed a killed and restarted
  process unwrapping inside it, D4 (reboot) showed it surviving a
  reboot and refusing once the window had passed, D5 refused at 75 s
  with the background key as the control; no start duration was
  measured, and 30 s was never exercised. The
  constant is named once by the Keystore adapter and is not a
  configuration setting: `key_unlock_policy` chooses the mode, the
  record chooses the time.
- What the window is for, stated so a later change has something to
  argue with: it lets the key operation that follows an authentication
  complete — the start's unwrap, a provisioning seal, a restore (the
  custody document already has step 7 prompting before a restore), a
  reseal. It is not a convenience window. A
  Keystore key belongs to the app uid, so the window grants no other
  application anything; what it grants is an unattended restart of this
  app's own service within the window of an unlock, which the
  `background_restart_requires_user_authentication` diagnostic already
  describes.
- The prompt: the Service never prompts. A `UserNotAuthenticatedException`
  is answered by the Activity with a device-credential or Class-3
  biometric prompt bound to no operation — any success opens the window
  — followed by the operation; a denied or cancelled prompt leaves the
  person offline (the custody document's failure row, unchanged).
- The per-operation mode (`t = 0`, a `BiometricPrompt` `CryptoObject`
  per use; as SPIKE-009 D1 built it, Class 3 only and invalidated by
  every biometric enrollment, D6b) is not used: it needs a prompt bound to each operation,
  which the Service cannot show, and it is the "require biometric on
  every network operation" alternative the record already rejects.
- Security implications: the unattended restart inside the window is
  named as the one exception to "sacrifices unattended restart", and
  60 s bounds it.

Propagated to `android-key-custody.md` §user-presence (its two
"cannot resume/become reachable until the user authenticates" sentences
gain the within-the-window exception), `RISKS.md` and the digest.
Landing with the pull request rust-ui-dev's batch 2 opens.

### Amendment 2026-10-20 — A refusal at a return to a live Activity is answered by the locked notice and its Unlock action, not by the prompt

**Trigger.** rust-ui-dev-01 asked ux-designer-01 whether a return to the
same Activity from Home or Recents after a cancelled prompt is "a launch
of the app's Activity by the person" (QUESTION 01a155b3-448f), the case
the custody document's 2026-10-19 sentence made presence. ux-designer-01
put it to architect-cto (QUESTION 01a158ad-de5e); the ruling went into
`android-key-custody.md` on #304 as a6fd2c3f, which called itself a
narrowing of the Decision and recorded nothing here. #304's round-3
review (P2) read it as a change of this record's "the Activity answers
the refusal with a … prompt": at a return outside the window the
unwrap is refused and the Activity raises no prompt.

**Ruled.** It is a narrowing, and this is its record. The
operation-free prompt answers a refusal at a launch of the Activity —
its creation: a cold start, a new instance, the creation that follows a
process the system killed (which has no live Activity); not a
resumption, and not a recreation the system performs for a
configuration change — and on the locked notice's Unlock action. At any
other continuation of a live Activity — a return from Home or Recents
after a cancel, a recreation the system performs for a configuration
change (the supplier review's P3: the first wording excluded it from a
launch and assigned it nothing) — the refusal is answered by the locked
notice and its Unlock action, which raises that prompt; the
continuation binds without a prompt only where the window has opened
meanwhile. Why: the custody requirement is that no
key operation succeeds outside the window and that an authentication is
always offered, never that a person who declined is asked again each
time they switch back in — a prompt at every return would be the
"biometric on every network operation" posture this record rejects,
worn by the person instead of the Service. Identity acceptance 10 ("a
return … binds silently if the window has opened meanwhile, and
otherwise keeps the notice and Unlock") stands unchanged.

**Settled with it (the review's open risk).** The Activity keeps no
clock of the window — `USER_PRESENCE_WINDOW_S` is the Keystore
adapter's constant and the Keystore's fact: the key operation's
outcome — the unwrap at a start, the seal at a provisioning or a
restore — is the Activity's only reading of whether the window is
open, so every case starts with the operation and the cases differ
only in how a refusal is answered. "Raises the prompt before it binds" in the custody
document reads so: the start's unwrap is refused, the prompt answers
it, and the service is bound with the identity on the retry.

**Not changed.** The window and its value; the Service never prompts;
denied or cancelled is offline; the per-operation mode is not used.

Propagated to `android-key-custody.md` §user-presence (the sentence now
cites this amendment instead of calling itself a narrowing) and the
digest. `RISKS.md` is untouched: its row is about restart, not the
return. Landing on #304 (ux-designer-01's branch), the pull request the
custody sentence is on.
