// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identity a start takes (src/identity.rs), over the real custody
//! path and a fake of the platform's cipher: the first run provisions
//! under the profile's configured policy and every later start unlocks the
//! same `PeerId`; a record that cannot give its identity back is refused
//! for recovery, left as it was, and never replaced.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::cast_possible_truncation
)]
#![cfg(unix)]

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt as _;
use std::sync::Mutex;

use interweave_human_android_platform::{
    AvailabilityMode, Expected, FirstRun, IdentityRefused, KeyUnlockPolicy, ResealFailure,
    Resealed, RestoreRefused, UnlockStatus, locked_status, opening_needs_presence, phrase_peer,
    provision_profile, reseal_identity, restart_requires_user_authentication_at_rest,
    restore_identity, stand_in, start_identity, start_resealed,
};
use interweave_profile_config::{ProfileLock, ProfilePaths, TrustBoundary};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_embedded::custody::{
    self, CipherFailure, IV_LEN, KeyRef, RecoveryCause, SEALED_LEN, SEED_LEN, Sealed, Seed,
    SeedCipher,
};

const PROFILE: &str = "human";

/// A stand-in for the Keystore: one key per alias, a keystream XOR for
/// the seed and a keyed check over the associated data and ciphertext
/// for the tag -- enough for custody to tell a right key and AAD from a
/// wrong one. `fail` makes the next open fail as the platform would, and
/// `seal_fails` the next seal.
#[derive(Default)]
struct FakeKeystore {
    keys: Mutex<HashMap<String, [u8; 32]>>,
    generated: Mutex<Vec<String>>,
    fail: Mutex<Option<CipherFailure>>,
    seal_fails: Mutex<Option<CipherFailure>>,
    retire_fails: Mutex<bool>,
}

fn tag(key: &[u8; 32], aad: &[u8], ct: &[u8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (i, b) in key.iter().chain(aad).chain(ct).enumerate() {
        out[i % 16] = out[i % 16].rotate_left(3) ^ b ^ (i as u8);
    }
    out
}

impl SeedCipher for FakeKeystore {
    fn seal(&self, key: &KeyRef, aad: &[u8], seed: &Seed) -> Result<Sealed, CipherFailure> {
        if let Some(failure) = self.seal_fails.lock().unwrap().take() {
            return Err(failure);
        }
        let alias = key.alias();
        let mut fresh = [0u8; 32];
        for (i, b) in fresh.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(31) ^ (self.generated.lock().unwrap().len() as u8);
        }
        self.keys.lock().unwrap().insert(alias.clone(), fresh);
        self.generated.lock().unwrap().push(alias);
        let ct: Vec<u8> = seed
            .expose()
            .iter()
            .zip(fresh)
            .map(|(s, k)| s ^ k)
            .collect();
        let mut sealed = ct.clone();
        sealed.extend(tag(&fresh, aad, &ct));
        Ok(Sealed {
            iv: vec![7; IV_LEN],
            sealed,
        })
    }

    fn retire(&self, key: &KeyRef) -> Result<(), CipherFailure> {
        if *self.retire_fails.lock().unwrap() {
            return Err(CipherFailure::Unavailable("busy".to_owned()));
        }
        self.keys.lock().unwrap().remove(&key.alias());
        Ok(())
    }

    fn open(
        &self,
        key: &KeyRef,
        _iv: &[u8; IV_LEN],
        sealed: &[u8; SEALED_LEN],
        aad: &[u8],
    ) -> Result<Seed, CipherFailure> {
        if let Some(failure) = self.fail.lock().unwrap().take() {
            return Err(failure);
        }
        let k = *self
            .keys
            .lock()
            .unwrap()
            .get(&key.alias())
            .ok_or(CipherFailure::KeyMissing)?;
        let (ct, t) = sealed.split_at(SEED_LEN);
        if tag(&k, aad, ct) != t {
            return Err(CipherFailure::Authentication);
        }
        let mut seed = [0u8; SEED_LEN];
        for (i, b) in seed.iter_mut().enumerate() {
            *b = ct[i] ^ k[i];
        }
        Ok(Seed::new(seed))
    }
}

struct App {
    _root: tempfile::TempDir,
    dir: std::path::PathBuf,
}

fn app() -> App {
    let app = fresh_install();
    provision_profile(&app.dir, PROFILE).expect("the profile");
    app
}

/// The app's data directory as a fresh install leaves it: nothing in it.
fn fresh_install() -> App {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir");
    let dir = root.path().join("app");
    std::fs::create_dir(&dir).expect("mkdir");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("chmod");
    App { _root: root, dir }
}

fn record(app: &App) -> Vec<u8> {
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("p");
    std::fs::read(custody::custody_file(&paths)).expect("the record")
}

#[test]
fn the_first_run_provisions_and_every_later_start_unlocks_the_same_peer() {
    let app = app();
    let keystore = FakeKeystore::default();
    let first =
        start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("the first run");
    let peer = first.transport_identity().expect("a PeerId");
    assert_eq!(
        *keystore.generated.lock().unwrap(),
        vec![format!("interweave.iwk1.{PROFILE}.background-compatible")],
        "one key, under the profile's configured policy (the default)"
    );
    let stored = record(&app);
    for _ in 0..2 {
        let again =
            start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("a later start");
        assert_eq!(again.transport_identity().expect("a PeerId"), peer);
    }
    assert_eq!(
        keystore.generated.lock().unwrap().len(),
        1,
        "no later start seals"
    );
    assert_eq!(record(&app), stored, "no later start rewrites the record");
}

#[test]
fn a_record_that_cannot_give_its_identity_back_is_refused_left_as_it_was_and_never_replaced() {
    let app = app();
    let keystore = FakeKeystore::default();
    let peer = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create)
        .expect("provisioned")
        .transport_identity()
        .expect("a PeerId");
    let stored = record(&app);
    for (failure, refused) in [
        (
            CipherFailure::KeyInvalidated,
            IdentityRefused::RecoveryRequired(RecoveryCause::KeyInvalidated),
        ),
        (
            CipherFailure::KeyMissing,
            IdentityRefused::RecoveryRequired(RecoveryCause::KeyMissing),
        ),
        (
            CipherFailure::Authentication,
            IdentityRefused::RecoveryRequired(RecoveryCause::Authentication),
        ),
        (
            CipherFailure::UserNotAuthenticated,
            IdentityRefused::UserNotAuthenticated,
        ),
        (
            CipherFailure::Unavailable("busy".to_owned()),
            IdentityRefused::Unavailable("busy".to_owned()),
        ),
    ] {
        *keystore.fail.lock().unwrap() = Some(failure.clone());
        assert_eq!(
            start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).map(|_| ()),
            Err(refused),
            "{failure:?}"
        );
        assert_eq!(
            record(&app),
            stored,
            "{failure:?} leaves the record as it was"
        );
        assert_eq!(
            keystore.generated.lock().unwrap().len(),
            1,
            "{failure:?} seals nothing"
        );
    }
    let again = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create)
        .expect("the key answers again");
    assert_eq!(again.transport_identity().expect("a PeerId"), peer);
}

#[test]
fn a_record_a_new_key_does_not_open_is_recovery_never_a_second_identity() {
    let app = app();
    start_identity(
        &app.dir,
        PROFILE,
        &FakeKeystore::default(),
        FirstRun::Create,
    )
    .expect("provisioned");
    // A keystore that holds none of this record's keys: the app reinstalled
    // over its old data, or the key deleted under it.
    let empty = FakeKeystore::default();
    assert_eq!(
        start_identity(&app.dir, PROFILE, &empty, FirstRun::Create).map(|_| ()),
        Err(IdentityRefused::RecoveryRequired(RecoveryCause::KeyMissing))
    );
    assert!(
        empty.generated.lock().unwrap().is_empty(),
        "nothing generated"
    );
}

#[test]
fn a_build_that_refuses_first_runs_answers_unprovisioned_and_writes_nothing() {
    let app = app();
    let keystore = FakeKeystore::default();
    assert_eq!(
        start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse).map(|_| ()),
        Err(IdentityRefused::Unprovisioned)
    );
    assert!(
        keystore.generated.lock().unwrap().is_empty(),
        "nothing generated"
    );
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("p");
    assert!(!custody::custody_file(&paths).exists(), "no record written");
}

#[test]
fn a_first_run_whose_old_key_could_not_be_retired_is_provisioned_and_starts() {
    let app = app();
    let keystore = FakeKeystore::default();
    *keystore.retire_fails.lock().unwrap() = true;
    let first = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create)
        .expect("the record is written, so the first run starts");
    let peer = first.transport_identity().expect("a PeerId");
    *keystore.retire_fails.lock().unwrap() = false;
    let again = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse).expect("unlocks");
    assert_eq!(again.transport_identity().expect("a PeerId"), peer);
}

/// The profile's configuration rewritten to name `policy`, as an operator
/// changing `key_unlock_policy` does: the key in the first-run document's
/// `runtime.android` block, set again where an earlier call set it.
fn configure_policy(app: &App, policy: &str) {
    let file = paths(app).config_file();
    let text = std::fs::read_to_string(&file).expect("the profile");
    let line = format!("    key_unlock_policy: {policy}\n");
    let rewritten = match text.find("    key_unlock_policy: ") {
        Some(at) => {
            let end = at + text[at..].find('\n').expect("a line") + 1;
            format!("{}{line}{}", &text[..at], &text[end..])
        }
        None => text.replacen("  android:\n", &format!("  android:\n{line}"), 1),
    };
    assert_ne!(rewritten, text, "the document has a runtime.android block");
    stand_in::provision_replacing(&app.dir, PROFILE, &rewritten).expect("rewritten");
}

fn holds_key(keystore: &FakeKeystore, app: &App, policy: KeyUnlockPolicy) -> bool {
    keystore
        .keys
        .lock()
        .unwrap()
        .contains_key(&KeyRef::new(&paths(app), policy).alias())
}

/// ADR-0042 A 2026-10-10 (ii): a record sealed under the policy the
/// profile named then still opens after the operator changes it, and the
/// start's reseal seals it again under the new one and deletes the old
/// key -- after which only the new key opens it, and a later start
/// changes nothing.
#[test]
fn a_changed_unlock_policy_is_applied_by_the_next_starts_reseal() {
    let app = app();
    let keystore = FakeKeystore::default();
    let first = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("first");
    let peer = first.transport_identity().expect("a PeerId");
    assert_eq!(
        reseal_identity(&app.dir, PROFILE, &keystore, &first),
        Ok(Resealed::Unchanged),
        "the default policy is the record's"
    );
    let before = record(&app);

    configure_policy(&app, "user-presence");
    let started = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse)
        .expect("the record's own policy opens it");
    assert_eq!(
        reseal_identity(&app.dir, PROFILE, &keystore, &started),
        Ok(Resealed::Changed)
    );
    assert_ne!(record(&app), before, "sealed again");
    assert!(holds_key(&keystore, &app, KeyUnlockPolicy::UserPresence));
    assert!(
        !holds_key(&keystore, &app, KeyUnlockPolicy::BackgroundCompatible),
        "the old policy's key is deleted"
    );
    let again = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse).expect("opens");
    assert_eq!(again.transport_identity().expect("a PeerId"), peer);
    assert_eq!(
        reseal_identity(&app.dir, PROFILE, &keystore, &again),
        Ok(Resealed::Unchanged)
    );
}

/// An `OldKeyRemains` left by a provision is retried at the next start
/// (design 2026-10-19-android-identity.md section 6): the record is
/// already under the configured policy, and the other policy's key is
/// deleted all the same.
#[test]
fn a_key_a_first_run_could_not_retire_is_retired_by_the_next_starts_reseal() {
    let app = app();
    let keystore = FakeKeystore::default();
    let stale = KeyRef::new(&paths(&app), KeyUnlockPolicy::UserPresence).alias();
    keystore.keys.lock().unwrap().insert(stale, [9; 32]);
    *keystore.retire_fails.lock().unwrap() = true;
    let first = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("first");
    assert!(
        holds_key(&keystore, &app, KeyUnlockPolicy::UserPresence),
        "left"
    );

    *keystore.retire_fails.lock().unwrap() = false;
    let before = record(&app);
    assert_eq!(
        reseal_identity(&app.dir, PROFILE, &keystore, &first),
        Ok(Resealed::Unchanged)
    );
    assert!(
        !holds_key(&keystore, &app, KeyUnlockPolicy::UserPresence),
        "retired"
    );
    assert_eq!(record(&app), before, "nothing sealed");
}

/// The new policy's key needs the person: the start is refused as
/// `UserNotAuthenticated`, which the app answers with the prompt and a
/// retried start (ADR-0042 A 2026-10-20), and the record is untouched.
#[test]
fn a_reseal_that_needs_the_person_refuses_the_start_and_leaves_the_record() {
    let app = app();
    let keystore = FakeKeystore::default();
    start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("first");
    configure_policy(&app, "user-presence");
    let before = record(&app);
    let started = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse).expect("opens");
    *keystore.seal_fails.lock().unwrap() = Some(CipherFailure::UserNotAuthenticated);
    assert_eq!(
        reseal_identity(&app.dir, PROFILE, &keystore, &started),
        Err(IdentityRefused::UserNotAuthenticated)
    );
    assert_eq!(record(&app), before);
    assert!(holds_key(
        &keystore,
        &app,
        KeyUnlockPolicy::BackgroundCompatible
    ));
    assert_eq!(
        reseal_identity(&app.dir, PROFILE, &keystore, &started),
        Ok(Resealed::Changed),
        "the retry after the prompt applies it"
    );
}

/// Any other refusal leaves the record opening to the identity, so the
/// start goes on: a seal the platform could not make writes nothing, and
/// a retire that failed after the write leaves the new record opening.
#[test]
fn a_reseal_that_cannot_finish_is_pending_and_the_record_still_opens() {
    let app = app();
    let keystore = FakeKeystore::default();
    let first = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("first");
    let peer = first.transport_identity().expect("a PeerId");
    configure_policy(&app, "user-presence");
    let before = record(&app);

    *keystore.seal_fails.lock().unwrap() = Some(CipherFailure::Unavailable("busy".to_owned()));
    assert!(matches!(
        reseal_identity(&app.dir, PROFILE, &keystore, &first),
        Ok(Resealed::Pending {
            why: ResealFailure::Keystore,
            ..
        })
    ));
    assert_eq!(record(&app), before, "nothing written");

    *keystore.retire_fails.lock().unwrap() = true;
    assert!(matches!(
        reseal_identity(&app.dir, PROFILE, &keystore, &first),
        Ok(Resealed::Pending {
            why: ResealFailure::OldKeyRemains,
            ..
        })
    ));
    assert_ne!(record(&app), before, "the new record is written");
    let again = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse).expect("opens");
    assert_eq!(again.transport_identity().expect("a PeerId"), peer);
}

/// A profile another holder has is not a reseal's outcome (ADR-0042 A
/// 2026-11-05): nothing is tried, the record is untouched, and the start's
/// own take of the lock answers it.
#[test]
fn a_held_profile_is_not_resealed_and_the_record_is_left() {
    let app = app();
    let keystore = FakeKeystore::default();
    let first = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("first");
    configure_policy(&app, "user-presence");
    let before = record(&app);
    let held = ProfileLock::acquire(&paths(&app), std::time::Duration::ZERO).expect("held");
    assert_eq!(
        reseal_identity(&app.dir, PROFILE, &keystore, &first),
        Ok(Resealed::Held)
    );
    assert_eq!(record(&app), before);
    drop(held);
    assert_eq!(
        reseal_identity(&app.dir, PROFILE, &keystore, &first),
        Ok(Resealed::Changed)
    );
}

/// Sets the identity directory's mode for as long as it lives, and puts
/// it back after, so the tempdir can be removed.
struct DirMode(std::path::PathBuf);

impl DirMode {
    fn set(app: &App, mode: u32) -> Self {
        let dir = custody::custody_file(&paths(app))
            .parent()
            .expect("the identity directory")
            .to_path_buf();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(mode)).expect("chmod");
        Self(dir)
    }
}

impl Drop for DirMode {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
    }
}

/// A reseal whose write landed but whose directory could not be synced has
/// the record under the configured policy and the old key not retired:
/// `OldKeyRemains`, never `Storage`, which means nothing was written
/// (design 2026-11-05-unlock-policy-pending.md section 3). The control: a
/// directory the write cannot create in is `Storage`, the record as it
/// was.
#[test]
fn an_unsynced_reseal_is_old_key_remains_and_a_failed_write_is_storage() {
    let app = app();
    let keystore = FakeKeystore::default();
    let first = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("first");
    configure_policy(&app, "user-presence");
    let before = record(&app);

    let unwritable = DirMode::set(&app, 0o500);
    let refused = reseal_identity(&app.dir, PROFILE, &keystore, &first);
    drop(unwritable);
    assert!(
        matches!(
            refused,
            Ok(Resealed::Pending {
                why: ResealFailure::Storage,
                ..
            })
        ),
        "{refused:?}"
    );
    assert_eq!(record(&app), before, "nothing written");

    let unsyncable = DirMode::set(&app, 0o300);
    if std::fs::File::open(&unsyncable.0).is_ok() {
        println!("skipped: 0o300 did not block opening the directory here");
        return;
    }
    let unsynced = reseal_identity(&app.dir, PROFILE, &keystore, &first);
    drop(unsyncable);
    assert!(
        matches!(
            unsynced,
            Ok(Resealed::Pending {
                why: ResealFailure::OldKeyRemains,
                ref detail,
            }) if detail.contains("Unsynced")
        ),
        "{unsynced:?}"
    );
    assert_ne!(record(&app), before, "the new record is in place");
    assert_eq!(
        custody::recorded_policy(&paths(&app)),
        Ok(Some(KeyUnlockPolicy::UserPresence))
    );
    assert!(
        holds_key(&keystore, &app, KeyUnlockPolicy::BackgroundCompatible),
        "the old key is not retired"
    );
}

/// The policy in force after each kind of start (ADR-0041 A 2026-11-05):
/// the configured one where the reseal wrote the record, the opened
/// record's where it did not -- so a change left pending keeps the old
/// policy in force, and the restart diagnostic follows it, never the
/// configuration.
#[test]
fn the_unlock_status_is_the_record_in_force_and_the_diagnostic_follows_it() {
    let app = app();
    let keystore = FakeKeystore::default();
    let first = start_resealed(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("first");
    assert_eq!(first.resealed, Resealed::Unchanged);
    let background = UnlockStatus {
        in_force: KeyUnlockPolicy::BackgroundCompatible,
        configured: Some(KeyUnlockPolicy::BackgroundCompatible),
        pending: None,
    };
    assert_eq!(first.unlock, background);

    // A seal the Keystore refused: the old policy stays in force.
    configure_policy(&app, "user-presence");
    *keystore.seal_fails.lock().unwrap() = Some(CipherFailure::Unavailable("busy".to_owned()));
    let pending = start_resealed(&app.dir, PROFILE, &keystore, FirstRun::Refuse).expect("starts");
    let left = UnlockStatus {
        in_force: KeyUnlockPolicy::BackgroundCompatible,
        configured: Some(KeyUnlockPolicy::UserPresence),
        pending: Some(ResealFailure::Keystore),
    };
    assert_eq!(pending.unlock, left);
    assert!(
        !left.restart_requires_user_authentication(AvailabilityMode::StayReachable),
        "the record in force needs nobody, whatever is configured"
    );

    // Held: nothing tried, so the opened record's policy, nothing pending
    // by class -- the mismatch says it.
    let held = ProfileLock::acquire(&paths(&app), std::time::Duration::ZERO).expect("held");
    let by_another = start_resealed(&app.dir, PROFILE, &keystore, FirstRun::Refuse).expect("ok");
    drop(held);
    assert_eq!(by_another.resealed, Resealed::Held);
    assert_eq!(
        by_another.unlock,
        UnlockStatus {
            pending: None,
            ..left
        }
    );

    // A retire that failed after the write: the new policy is in force.
    *keystore.retire_fails.lock().unwrap() = true;
    let written = start_resealed(&app.dir, PROFILE, &keystore, FirstRun::Refuse).expect("ok");
    let in_force = UnlockStatus {
        in_force: KeyUnlockPolicy::UserPresence,
        configured: Some(KeyUnlockPolicy::UserPresence),
        pending: Some(ResealFailure::OldKeyRemains),
    };
    assert_eq!(written.unlock, in_force);
    assert!(in_force.restart_requires_user_authentication(AvailabilityMode::StayReachable));
    assert!(!in_force.restart_requires_user_authentication(AvailabilityMode::ForegroundOnly));

    // Applied: nothing pending.
    *keystore.retire_fails.lock().unwrap() = false;
    let applied = start_resealed(&app.dir, PROFILE, &keystore, FirstRun::Refuse).expect("ok");
    assert_eq!(
        applied.unlock,
        UnlockStatus {
            pending: None,
            ..in_force
        }
    );

    // Back the other way, the configuration unreadable: in force is the
    // opened record's, configured unknown.
    std::fs::write(paths(&app).config_file(), b"not: [valid").expect("broken");
    let unknown = start_resealed(&app.dir, PROFILE, &keystore, FirstRun::Refuse).expect("ok");
    assert_eq!(
        unknown.unlock,
        UnlockStatus {
            in_force: KeyUnlockPolicy::UserPresence,
            configured: None,
            pending: Some(ResealFailure::Other),
        }
    );
}

/// A start refused `UserNotAuthenticated` opened nothing: its policy in
/// force is the record header's (ADR-0041 A 2026-11-05), configured beside
/// it; a profile with no record has none. What is pending is the last
/// start's while the record is still under the policy it left in force,
/// and nothing once it is not; the restart diagnostic reads the mode from
/// disk.
#[test]
fn a_locked_start_takes_the_record_headers_policy() {
    let app = app();
    assert_eq!(
        locked_status(&app.dir, PROFILE, None),
        None,
        "no record yet"
    );
    let keystore = FakeKeystore::default();
    start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("first");
    configure_policy(&app, "user-presence");
    let header = UnlockStatus {
        in_force: KeyUnlockPolicy::BackgroundCompatible,
        configured: Some(KeyUnlockPolicy::UserPresence),
        pending: None,
    };
    assert_eq!(locked_status(&app.dir, PROFILE, None), Some(header));

    let last = UnlockStatus {
        pending: Some(ResealFailure::Keystore),
        ..header
    };
    assert_eq!(
        locked_status(&app.dir, PROFILE, Some(last)),
        Some(last),
        "the record unchanged since, so is why"
    );
    let other = UnlockStatus {
        in_force: KeyUnlockPolicy::UserPresence,
        ..last
    };
    assert_eq!(
        locked_status(&app.dir, PROFILE, Some(other)),
        Some(header),
        "a record under another policy than the last start left"
    );

    assert_eq!(
        restart_requires_user_authentication_at_rest(&app.dir, PROFILE, &other),
        Some(false),
        "the first-run document's mode is foreground-only"
    );
}

/// Opening the record needs the person by the record's own policy, not
/// the configured one: a pending change in either direction answers by the
/// record; with no record, the configured policy a first run seals under.
#[test]
fn opening_the_record_needs_the_person_by_its_own_policy() {
    let app = app();
    let keystore = FakeKeystore::default();
    configure_policy(&app, "user-presence");
    assert!(
        opening_needs_presence(&app.dir, PROFILE),
        "no record: the configured policy"
    );
    configure_policy(&app, "background-compatible");
    let first = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("first");
    configure_policy(&app, "user-presence");
    assert!(
        !opening_needs_presence(&app.dir, PROFILE),
        "pending toward user-presence: the record needs nobody yet"
    );
    assert_eq!(
        reseal_identity(&app.dir, PROFILE, &keystore, &first),
        Ok(Resealed::Changed)
    );
    configure_policy(&app, "background-compatible");
    assert!(
        opening_needs_presence(&app.dir, PROFILE),
        "pending away from user-presence: the record still needs the person"
    );
}

/// Plan section 20 gate (a), the client clauses: a key the platform lost
/// enters recovery, mints nothing, and the person's phrase restores the
/// `PeerId` the record names -- which then unlocks at every later start.
#[test]
fn recovery_restores_the_recorded_peer_from_the_phrase_and_it_unlocks_after() {
    let app = app();
    let identity = start_identity(
        &app.dir,
        PROFILE,
        &FakeKeystore::default(),
        FirstRun::Create,
    )
    .expect("provisioned");
    let peer = identity.transport_identity().expect("a PeerId");
    let phrase = identity.recovery_phrase().expect("its phrase");
    // The keys are gone: the app reinstalled over its data, or the key
    // deleted under it.
    let keystore = FakeKeystore::default();
    assert_eq!(
        start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).map(|_| ()),
        Err(IdentityRefused::RecoveryRequired(RecoveryCause::KeyMissing))
    );

    let restored = restore_identity(
        &app.dir,
        PROFILE,
        &keystore,
        &phrase,
        Expected::Recorded,
        None,
    )
    .expect("the phrase restores");
    assert_eq!(restored.transport_identity().expect("a PeerId"), peer);
    let again = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse)
        .expect("the restored record unlocks");
    assert_eq!(again.transport_identity().expect("a PeerId"), peer);
}

/// BIP-39's checksum is eight bits: a phrase that parses can be another
/// identity's. It is refused and the record stays as it was, so the
/// person's own phrase still restores it.
#[test]
fn another_identitys_phrase_is_refused_and_the_record_is_left_as_it_was() {
    let app = app();
    let identity = start_identity(
        &app.dir,
        PROFILE,
        &FakeKeystore::default(),
        FirstRun::Create,
    )
    .expect("provisioned");
    let before = record(&app);
    let keystore = FakeKeystore::default();
    let stranger = ProfileIdentity::generate()
        .recovery_phrase()
        .expect("a phrase");

    assert_eq!(
        restore_identity(
            &app.dir,
            PROFILE,
            &keystore,
            &stranger,
            Expected::Recorded,
            None
        )
        .map(|_| ()),
        Err(RestoreRefused::OtherIdentity)
    );
    assert_eq!(record(&app), before, "the record is untouched");
    assert!(keystore.generated.lock().unwrap().is_empty(), "no key made");
    // Control: the right phrase still restores over the same record.
    restore_identity(
        &app.dir,
        PROFILE,
        &keystore,
        &identity.recovery_phrase().expect("its phrase"),
        Expected::Recorded,
        None,
    )
    .expect("the person's own phrase restores");
}

/// A fresh install has no record to say which `PeerId` to restore, and
/// none is taken on trust: nothing is written.
#[test]
fn with_no_record_nothing_is_restored_without_a_confirmed_peer() {
    let app = app();
    let keystore = FakeKeystore::default();
    let phrase = ProfileIdentity::generate()
        .recovery_phrase()
        .expect("a phrase");

    assert_eq!(
        restore_identity(
            &app.dir,
            PROFILE,
            &keystore,
            &phrase,
            Expected::Recorded,
            None
        )
        .map(|_| ()),
        Err(RestoreRefused::NoRecord)
    );
    assert!(
        !custody::custody_file(&paths(&app)).exists(),
        "no record written"
    );
    assert!(keystore.generated.lock().unwrap().is_empty(), "no key made");
}

/// The fresh install's restore: the app shows the `PeerId` the phrase
/// derives to, the person confirms it, and that is what is restored and
/// unlocks after. A confirmation of another `PeerId` restores nothing.
#[test]
fn a_fresh_install_restores_the_peer_the_person_confirmed_and_no_other() {
    let app = app();
    let keystore = FakeKeystore::default();
    let original = ProfileIdentity::generate();
    let phrase = original.recovery_phrase().expect("a phrase");
    let shown = phrase_peer(&phrase).expect("the derived PeerId");
    assert_eq!(shown, original.transport_identity().expect("a PeerId"));

    let other = ProfileIdentity::generate()
        .transport_identity()
        .expect("a PeerId");
    assert_eq!(
        restore_identity(
            &app.dir,
            PROFILE,
            &keystore,
            &phrase,
            Expected::Confirmed(&other),
            None
        )
        .map(|_| ()),
        Err(RestoreRefused::OtherIdentity)
    );
    assert!(
        !custody::custody_file(&paths(&app)).exists(),
        "no record written"
    );

    let restored = restore_identity(
        &app.dir,
        PROFILE,
        &keystore,
        &phrase,
        Expected::Confirmed(&shown),
        None,
    )
    .expect("the confirmed PeerId restores");
    assert_eq!(restored.transport_identity().expect("a PeerId"), shown);
    let again = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse)
        .expect("the new record unlocks");
    assert_eq!(again.transport_identity().expect("a PeerId"), shown);
}

/// A confirmed `PeerId` never stands in for a record: over an established
/// identity, even its own confirmed `PeerId` is refused and the record is
/// left as it was -- that profile restores as its record names it.
#[test]
fn a_confirmed_peer_never_replaces_an_established_record() {
    let app = app();
    let identity = start_identity(
        &app.dir,
        PROFILE,
        &FakeKeystore::default(),
        FirstRun::Create,
    )
    .expect("provisioned");
    let before = record(&app);
    let phrase = identity.recovery_phrase().expect("its phrase");
    let peer = identity.transport_identity().expect("a PeerId");

    assert_eq!(
        restore_identity(
            &app.dir,
            PROFILE,
            &FakeKeystore::default(),
            &phrase,
            Expected::Confirmed(&peer),
            None
        )
        .map(|_| ()),
        Err(RestoreRefused::RecordExists)
    );
    assert_eq!(record(&app), before, "the record is untouched");
}

/// A record present whose `PeerId` field names nobody is an established
/// profile's, not "no record": neither restore is offered over it, and
/// nothing is written (android-key-custody.md, Recovery).
#[test]
fn a_record_naming_nobody_is_restored_by_neither_path_and_left_as_it_was() {
    let app = app();
    let keystore = FakeKeystore::default();
    let identity =
        start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("provisioned");
    // An envelope's length with no PeerId after it.
    std::fs::write(custody::custody_file(&paths(&app)), [0u8; 66]).expect("overwritten");
    let before = record(&app);
    let phrase = identity.recovery_phrase().expect("its phrase");
    let peer = identity.transport_identity().expect("a PeerId");

    for expected in [Expected::Recorded, Expected::Confirmed(&peer)] {
        assert!(
            matches!(
                restore_identity(&app.dir, PROFILE, &keystore, &phrase, expected, None),
                Err(RestoreRefused::RecordUnreadable(_))
            ),
            "{expected:?}"
        );
    }
    assert_eq!(record(&app), before, "the record is untouched");
}

fn paths(app: &App) -> ProfilePaths {
    ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b")).expect("p")
}

mod onboarding {
    use super::*;
    use interweave_human_android_platform::onboarding::{
        self, Creation, PHRASE_WORDS, WriteRefused, english,
    };

    /// C1 to C4: the words come from BIP-39's list and name the `PeerId`
    /// shown; nothing is on disk until the seal; the sealed record unlocks
    /// to that `PeerId`.
    #[test]
    fn a_creation_writes_nothing_until_the_seal_and_unlocks_the_peer_shown() {
        let app = app();
        let keystore = FakeKeystore::default();
        let (creation, words, peer, check) = Creation::new().expect("a creation");
        assert!(words.iter().all(|w| english().binary_search(w).is_ok()));
        assert_ne!(check[0], check[1]);
        assert!(check.iter().all(|&p| p < PHRASE_WORDS));
        assert_eq!(
            onboarding::check(words.join(" ")),
            Some(peer.clone()),
            "the words shown derive the PeerId shown"
        );
        assert!(
            !custody::custody_file(&paths(&app)).exists(),
            "nothing sealed yet"
        );

        onboarding::seal(&app.dir, PROFILE, &keystore, &creation, None).expect("sealed");
        let started = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse)
            .expect("the record unlocks");
        assert_eq!(started.transport_identity().expect("a PeerId"), peer);
    }

    /// A release first run (j96): the seal reads the unlock policy from
    /// the profile's document, which a fresh install does not have until
    /// the app's provisioning writes it -- refused before, sealed after,
    /// and the release build's start (`FirstRun::Refuse`, which sends the
    /// person to onboarding) then unlocks the `PeerId` shown.
    #[test]
    fn a_release_first_run_writes_the_profile_onboarding_seals_under() {
        let app = fresh_install();
        let keystore = FakeKeystore::default();
        let (creation, _, peer, _) = Creation::new().expect("a creation");
        assert!(
            matches!(
                onboarding::seal(&app.dir, PROFILE, &keystore, &creation, None),
                Err(WriteRefused::Storage(why)) if why.starts_with("the profile:")
            ),
            "the control: with no document the seal is refused"
        );

        provision_profile(&app.dir, PROFILE).expect("the first-run profile");
        assert_eq!(
            start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse).map(|_| ()),
            Err(IdentityRefused::Unprovisioned),
            "onboarding's state"
        );
        onboarding::seal(&app.dir, PROFILE, &keystore, &creation, None).expect("sealed");
        let started = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse)
            .expect("the record unlocks");
        assert_eq!(started.transport_identity().expect("a PeerId"), peer);
    }

    /// The flow's lock (android-key-custody.md, Recovery): taken before
    /// the first word, it is lent to the seal -- which, taking its own,
    /// would answer Busy -- and a refused seal leaves it held for the
    /// read-back and a retry; only the flow lets it go.
    #[test]
    fn the_flows_lock_is_lent_to_the_seal_and_held_after_a_refusal() {
        let app = app();
        let keystore = FakeKeystore::default();
        let lock = onboarding::hold(&app.dir, PROFILE).expect("held");
        assert_eq!(
            onboarding::hold(&app.dir, PROFILE).err(),
            Some(WriteRefused::Busy),
            "one holder"
        );
        let (creation, _, peer, _) = Creation::new().expect("a creation");
        assert_eq!(
            onboarding::seal(&app.dir, PROFILE, &keystore, &creation, None),
            Err(WriteRefused::Busy),
            "a seal of its own waits for no lock the flow holds"
        );
        onboarding::seal(&app.dir, PROFILE, &keystore, &creation, Some(&lock))
            .expect("sealed under the flow's lock");
        assert_eq!(
            onboarding::seal(&app.dir, PROFILE, &keystore, &creation, Some(&lock)),
            Err(WriteRefused::AlreadyProvisioned)
        );
        assert!(
            ProfileLock::is_held(&paths(&app)).expect("asked"),
            "still held after the refusal"
        );
        drop(lock);
        assert!(!ProfileLock::is_held(&paths(&app)).expect("asked"));
        let started = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse)
            .expect("the record unlocks");
        assert_eq!(started.transport_identity().expect("a PeerId"), peer);
    }

    /// A restore runs under the flow's lock too, where one of its own
    /// would be refused.
    #[test]
    fn a_restore_runs_under_the_flows_lock() {
        let app = app();
        let keystore = FakeKeystore::default();
        let (creation, words, ..) = Creation::new().expect("a creation");
        onboarding::seal(&app.dir, PROFILE, &keystore, &creation, None).expect("sealed");
        let lock = onboarding::hold(&app.dir, PROFILE).expect("held");
        assert_eq!(
            onboarding::restore(
                &app.dir,
                PROFILE,
                &keystore,
                words.join(" "),
                Expected::Recorded,
                None,
            ),
            Err(WriteRefused::Busy)
        );
        assert_eq!(
            onboarding::restore(
                &app.dir,
                PROFILE,
                &keystore,
                words.join(" "),
                Expected::Recorded,
                Some(&lock),
            ),
            Ok(())
        );
        assert!(ProfileLock::is_held(&paths(&app)).expect("asked"));
    }

    /// S2: the sealed record gives back the words and the `PeerId` the
    /// creation showed; with no record nothing is shown.
    #[test]
    fn show_my_phrase_gives_back_the_words_sealed() {
        let app = app();
        let keystore = FakeKeystore::default();
        assert!(matches!(
            onboarding::shown_phrase(&app.dir, PROFILE, &keystore),
            Err(WriteRefused::Storage(_))
        ));
        let (creation, words, peer, _) = Creation::new().expect("a creation");
        onboarding::seal(&app.dir, PROFILE, &keystore, &creation, None).expect("sealed");
        let (shown, shown_peer) =
            onboarding::shown_phrase(&app.dir, PROFILE, &keystore).expect("shown");
        assert_eq!(shown, words);
        assert_eq!(shown_peer, peer);
    }

    /// Another first run sealed first: the words shown were never used,
    /// and the record stays the other identity's.
    #[test]
    fn a_seal_over_a_provisioned_profile_is_refused_and_changes_nothing() {
        let app = app();
        let keystore = FakeKeystore::default();
        start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("provisioned");
        let before = record(&app);
        let (creation, ..) = Creation::new().expect("a creation");
        assert_eq!(
            onboarding::seal(&app.dir, PROFILE, &keystore, &creation, None),
            Err(WriteRefused::AlreadyProvisioned)
        );
        assert_eq!(record(&app), before);
    }

    /// The read-back (#303 rounds 2 and 3): a seal refused after the
    /// publish -- here a key of the other policy that could not be retired
    /// -- is success when the record opens to the identity just shown.
    #[test]
    fn a_seal_refused_after_its_publish_is_read_back_and_succeeds() {
        let app = app();
        let keystore = FakeKeystore::default();
        *keystore.retire_fails.lock().unwrap() = true;
        let (creation, _, peer, _) = Creation::new().expect("a creation");
        assert_eq!(
            onboarding::seal(&app.dir, PROFILE, &keystore, &creation, None),
            Ok(())
        );
        *keystore.retire_fails.lock().unwrap() = false;
        let started = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse)
            .expect("the record unlocks");
        assert_eq!(started.transport_identity().expect("a PeerId"), peer);
    }

    /// A second Continue over the record just sealed is not read back: it
    /// is a provisioned profile, whatever the record names.
    #[test]
    fn a_seal_over_its_own_record_is_already_provisioned() {
        let app = app();
        let keystore = FakeKeystore::default();
        let (creation, ..) = Creation::new().expect("a creation");
        onboarding::seal(&app.dir, PROFILE, &keystore, &creation, None).expect("sealed");
        assert_eq!(
            onboarding::seal(&app.dir, PROFILE, &keystore, &creation, None),
            Err(WriteRefused::AlreadyProvisioned)
        );
    }

    /// #303 round 3's P1: at a recovery the old record already names the
    /// expected `PeerId`, so a restore whose write fails before it
    /// publishes must not read as done.
    #[test]
    fn a_recovery_whose_write_fails_before_publishing_is_not_reported_done() {
        let app = app();
        let identity = start_identity(
            &app.dir,
            PROFILE,
            &FakeKeystore::default(),
            FirstRun::Create,
        )
        .expect("provisioned");
        let words = onboarding::words_of(&identity.recovery_phrase().expect("a phrase"))
            .expect("its words");
        let keystore = FakeKeystore::default();
        let before = record(&app);
        let identity_dir = paths(&app).identity_dir().to_owned();
        std::fs::set_permissions(&identity_dir, std::fs::Permissions::from_mode(0o500))
            .expect("read-only");
        let outcome = onboarding::restore(
            &app.dir,
            PROFILE,
            &keystore,
            words.join(" "),
            Expected::Recorded,
            None,
        );
        std::fs::set_permissions(&identity_dir, std::fs::Permissions::from_mode(0o700))
            .expect("writable again");
        assert!(
            matches!(outcome, Err(WriteRefused::Storage(_) | WriteRefused::Busy)),
            "{outcome:?}"
        );
        // Not by unlocking again: this fake makes the same key again for
        // an alias, which a Keystore never does.
        assert_eq!(record(&app), before, "nothing was restored");
    }

    #[test]
    fn a_phrase_that_fails_its_checksum_derives_nothing() {
        assert_eq!(onboarding::check(["abandon"; 24].join(" ")), None);
        assert_eq!(onboarding::check("not a phrase".to_owned()), None);
    }

    /// R1 to R4 on a fresh install, from the typed text.
    #[test]
    fn a_fresh_install_restores_the_confirmed_peer_from_the_typed_words() {
        let app = app();
        let keystore = FakeKeystore::default();
        let (_, words, peer, _) = Creation::new().expect("an identity to restore");
        let derived = onboarding::check(words.join(" ")).expect("a phrase");
        assert_eq!(derived, peer);
        onboarding::restore(
            &app.dir,
            PROFILE,
            &keystore,
            words.join(" "),
            Expected::Confirmed(&derived),
            None,
        )
        .expect("restored");
        let started = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse)
            .expect("the record unlocks");
        assert_eq!(started.transport_identity().expect("a PeerId"), peer);
    }

    /// The marker holds nothing but its existence, and clearing an absent
    /// one is no error.
    #[test]
    fn the_void_paper_marker_is_an_empty_flag() {
        let app = app();
        assert!(!onboarding::void_paper(&app.dir, PROFILE));
        onboarding::set_void_paper(&app.dir, PROFILE, true).expect("set");
        assert!(onboarding::void_paper(&app.dir, PROFILE));
        let marker = paths(&app).human_dir().join("void-paper");
        assert_eq!(std::fs::read(&marker).expect("the marker").len(), 0);
        assert_eq!(
            std::fs::metadata(&marker)
                .expect("its mode")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        onboarding::set_void_paper(&app.dir, PROFILE, false).expect("cleared");
        onboarding::set_void_paper(&app.dir, PROFILE, false).expect("cleared again");
        assert!(!onboarding::void_paper(&app.dir, PROFILE));
    }

    /// Connect's marker is a flag of its own: set and cleared apart from
    /// the void-paper one, empty and private, and clearing an absent one
    /// is no error.
    #[test]
    fn the_connect_owed_marker_is_its_own_empty_flag() {
        let app = app();
        assert!(!onboarding::connect_owed(&app.dir, PROFILE));
        onboarding::set_connect_owed(&app.dir, PROFILE, true).expect("set");
        assert!(onboarding::connect_owed(&app.dir, PROFILE));
        assert!(!onboarding::void_paper(&app.dir, PROFILE), "not the other");
        let marker = paths(&app).human_dir().join("connect-owed");
        assert_eq!(std::fs::read(&marker).expect("the marker").len(), 0);
        assert_eq!(
            std::fs::metadata(&marker)
                .expect("its mode")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        onboarding::set_void_paper(&app.dir, PROFILE, true).expect("the other set");
        onboarding::set_void_paper(&app.dir, PROFILE, false).expect("the other cleared");
        assert!(onboarding::connect_owed(&app.dir, PROFILE), "still owed");
        onboarding::set_connect_owed(&app.dir, PROFILE, false).expect("cleared");
        onboarding::set_connect_owed(&app.dir, PROFILE, false).expect("cleared again");
        assert!(!onboarding::connect_owed(&app.dir, PROFILE));
    }
}
