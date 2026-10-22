// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The offline identity commands (plan §16 (10), IDENTITY-RECOVERY.md):
//! never over IPC, never a socket, never the network (ADR-0033).

use std::io::{IsTerminal as _, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use interweave_profile_config::{
    LoadError, PersistError, ProfileConfig, ProfileLock, ProfilePaths, XdgRoots,
};
use interweave_profile_identity::{IdentityError, ProfileIdentity, RecoveryRecord};
use zeroize::Zeroizing;

use crate::Failure;
use crate::cli::{Identity, RestoreMode};
use crate::phrase;

/// Run an identity command; what to print on success.
///
/// # Errors
/// [`Failure::Refused`], naming why -- and never a word of the phrase.
pub(crate) fn run(command: Identity) -> Result<String, Failure> {
    match command {
        Identity::Init { profile } => init(&profile),
        Identity::Show { profile } => show(&profile),
        Identity::Verify { expected } => verify(&phrase::read(expected)?),
        Identity::Backup { profile, to_file } => backup(&profile, to_file.as_deref()),
        Identity::Restore {
            profile,
            mode,
            expected,
        } => restore(&profile, &mode, expected),
    }
}

/// Room for a pretty-printed record: 24 words of at most 8 letters, a
/// `PeerId` and the labels fit in well under this.
const RECORD_BUFFER: usize = 4096;

/// The profile's paths, offline (no runtime directory needed), and its
/// key file as its configuration names it.
fn profile(name: &str) -> Result<(ProfilePaths, PathBuf), Failure> {
    let refused = |what: &str, e: &dyn std::fmt::Display| Failure::Refused(format!("{what}: {e}"));
    let roots = XdgRoots::from_env().map_err(|e| refused("the XDG directories", &e))?;
    let paths = ProfilePaths::resolve_offline(name, &roots).map_err(crate::profile_refusal)?;
    let config = ProfileConfig::load(&paths).map_err(|e| refused("the profile", &e))?;
    let key_file = config.identity.key_file_in(&paths);
    Ok((paths, key_file))
}

/// The profile's paths, offline, and its key file -- as its configuration
/// names it, or, while the profile has no configuration yet, the
/// profile's default identity file (`ProfilePaths::identity_file`), which
/// is where a configuration that names no `key_file` keeps it. That is the
/// fleet's order (ADR-0055 decision 4): the key is minted first, its
/// `PeerId` goes into the roster, and the configuration is rendered from
/// the roster after. A configuration that is THERE but does not load is
/// refused, never read around: it may name another key file.
fn profile_for_init(name: &str) -> Result<(ProfilePaths, PathBuf), Failure> {
    let refused = |what: &str, e: &dyn std::fmt::Display| Failure::Refused(format!("{what}: {e}"));
    let roots = XdgRoots::from_env().map_err(|e| refused("the XDG directories", &e))?;
    let paths = ProfilePaths::resolve_offline(name, &roots).map_err(crate::profile_refusal)?;
    match ProfileConfig::load(&paths) {
        Ok(config) => {
            let key_file = config.identity.key_file_in(&paths);
            Ok((paths, key_file))
        }
        Err(LoadError::Read(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            let key_file = paths.identity_file();
            Ok((paths, key_file))
        }
        Err(e) => Err(refused(
            &format!("the profile {}", paths.config_file().display()),
            &e,
        )),
    }
}

/// Mint the profile's key and print its `PeerId`, the key's only public
/// part (ADR-0055 decision 4). Under the lock, so never while its daemon
/// runs; never over an existing key -- the exclusive create refuses one,
/// so two inits racing past the lock still cannot both write. The key
/// written is loaded back before its `PeerId` is printed, as a restore's.
fn init(name: &str) -> Result<String, Failure> {
    let refused = |what: &str, e: &dyn std::fmt::Display| Failure::Refused(format!("{what}: {e}"));
    let (paths, key_file) = profile_for_init(name)?;
    let _lock = lock(&paths)?;
    ProfileIdentity::generate()
        .save(&key_file)
        .map_err(|e| match e {
            IdentityError::AlreadyExists => Failure::Refused(format!(
                "profile {name:?} already has a key: identity show prints its PeerId"
            )),
            other => refused("creating the identity key", &other),
        })?;
    let peer = ProfileIdentity::load(&key_file)
        .and_then(|identity| identity.transport_identity())
        .map_err(|e| refused("reloading the new key", &e))?;
    Ok(format!("{}\n", peer.as_str()))
}

/// Print the `PeerId` of the profile's key, read under the lock as the
/// other verbs read it -- so refused while its daemon runs, whose
/// `status` says the same -- and nothing else of the key.
fn show(name: &str) -> Result<String, Failure> {
    let refused = |what: &str, e: &dyn std::fmt::Display| Failure::Refused(format!("{what}: {e}"));
    let (paths, key_file) = profile_for_init(name)?;
    let _lock = lock(&paths)?;
    let peer = ProfileIdentity::load(&key_file)
        .and_then(|identity| identity.transport_identity())
        .map_err(|e| refused("the identity key", &e))?;
    Ok(format!("{}\n", peer.as_str()))
}

/// The profile lock, taken at once or not at all: held, it is a running
/// daemon or another identity command, and the key is not touched under
/// it (plan §16 (6), (10)).
fn lock(paths: &ProfilePaths) -> Result<ProfileLock, Failure> {
    ProfileLock::acquire(paths, Duration::ZERO).map_err(|e| match e {
        PersistError::ProfileLocked { .. } => Failure::Refused(format!(
            "profile {:?}'s lock is held -- stop its daemon first ({e})",
            paths.profile()
        )),
        e => Failure::Refused(format!(
            "profile {:?}'s lock cannot be taken: {e}",
            paths.profile()
        )),
    })
}

/// Emit the profile's recovery record (IDENTITY-RECOVERY.md §Export):
/// under the lock; to stdout only when it is a terminal, otherwise only
/// to a NEW file created owner-only. Never to a pipe, where it would end
/// up in whatever reads it.
fn backup(name: &str, to_file: Option<&Path>) -> Result<String, Failure> {
    let refused = |what: &str, e: &dyn std::fmt::Display| Failure::Refused(format!("{what}: {e}"));
    if to_file.is_none() && !std::io::stdout().is_terminal() {
        return Err(Failure::Refused(
            "stdout is not a terminal: the record is a secret, and goes to a terminal or to \
             --to-file <new path>"
                .to_owned(),
        ));
    }
    let (paths, key_file) = profile(name)?;
    let _lock = lock(&paths)?;
    let identity = ProfileIdentity::load(&key_file).map_err(|e| refused("the identity key", &e))?;
    let mut record =
        RecoveryRecord::of(&identity).map_err(|e| refused("the recovery record", &e))?;
    let peer = record.expected_peer_id.clone().unwrap_or_default();
    // Written into a buffer sized up front, so it never reallocates and
    // leaves an unzeroed copy behind; the record's words zeroed with it.
    let mut text = Zeroizing::new(Vec::with_capacity(RECORD_BUFFER));
    let written = serde_json::to_writer_pretty(&mut *text, &record);
    let words = Zeroizing::new(std::mem::take(&mut record.words));
    drop(words);
    written.map_err(|e| refused("the recovery record", &e))?;
    text.push(b'\n');
    debug_assert_eq!(
        text.capacity(),
        RECORD_BUFFER,
        "the record never grew past its first allocation"
    );
    if let Some(path) = to_file {
        // create_new: never over an existing file; 0600 at creation,
        // so the record is never readable by anyone else, umask or not.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| refused("creating the --to-file path", &e))?;
        file.write_all(&text)
            .and_then(|()| file.sync_all())
            .map_err(|e| refused("writing the --to-file path", &e))?;
        Ok(format!(
            "recovery record for {peer} written to {}\n",
            path.display()
        ))
    } else {
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(&text)
            .and_then(|()| stdout.flush())
            .map_err(|e| refused("writing the record", &e))?;
        Ok(String::new())
    }
}

/// Restore the profile's key from a phrase (IDENTITY-RECOVERY.md
/// §Restore): under the lock, taken BEFORE the phrase is asked for, so
/// nobody types a secret only to be refused; into an empty profile, or
/// over the established key named by `--replacing`; then the written key
/// is loaded back and its `PeerId` checked again before anything is said
/// to have worked (restore item 10).
///
/// The profile's configuration must exist: it is where the key file is
/// named, and the phrase restores the key alone, never the trust and the
/// endpoints a configuration carries (§Complete profile disaster-recovery).
fn restore(
    name: &str,
    mode: &RestoreMode,
    expected: Option<interweave_transport_api::TransportIdentity>,
) -> Result<String, Failure> {
    let refused = |what: &str, e: &dyn std::fmt::Display| Failure::Refused(format!("{what}: {e}"));
    let (paths, key_file) = profile(name)?;
    let _lock = lock(&paths)?;
    warn_active_elsewhere();
    let recovery = phrase::read(expected)?;
    let message = match mode {
        RestoreMode::New => {
            ProfileIdentity::restore_new(&key_file, &recovery.phrase, &recovery.expected).map_err(
                |e| match e {
                    IdentityError::AlreadyExists => Failure::Refused(format!(
                        "profile {name:?} already has a key: replacing it is --replace \
                         --replacing <its PeerId>"
                    )),
                    other => mismatch(other),
                },
            )?;
            format!(
                "restored {} into profile {name:?}\n",
                recovery.expected.as_str()
            )
        }
        RestoreMode::Replace { replacing } => {
            let (_, rotation) = ProfileIdentity::restore_replace(
                &key_file,
                &recovery.phrase,
                &recovery.expected,
                replacing,
            )
            .map_err(mismatch)?;
            format!(
                "replaced {} with {} in profile {name:?}\n",
                rotation.previous.as_str(),
                rotation.current.as_str()
            )
        }
    };
    let reloaded = ProfileIdentity::load(&key_file)
        .and_then(|identity| identity.transport_identity())
        .map_err(|e| refused("reloading the restored key", &e))?;
    if reloaded != recovery.expected {
        return Err(Failure::Refused(format!(
            "the key written reloads as {}, not {}: do not start the daemon",
            reloaded.as_str(),
            recovery.expected.as_str()
        )));
    }
    Ok(message)
}

/// What a restore says before it asks for the phrase.
const ACTIVE_ELSEWHERE: &str = "transportctl: warning: if this identity is still running on \
another node, restoring its phrase here puts one PeerId on two nodes, which ADR-0043 forbids; \
this host cannot check that\n";

/// ADR-0043's "recovery UX warns when importing an identity expected to
/// still be active elsewhere" (IDENTITY-RECOVERY.md section Restore
/// semantics item 11), on every restore, both forms, every input path:
/// written before [`phrase::read`] is called, so a pipe or a file is
/// warned as a terminal is. Nothing on this host can tell whether the
/// phrase is live on another node, so the warning is unconditional, and
/// it is a line rather than a question -- a restore stays one command
/// line. On stderr, where the terminal prompt goes; stdout carries the
/// result. `apps/transportctl/tests/identity.rs`
/// `a_restore_warns_before_it_reads_the_phrase` and
/// `on_a_terminal_the_warning_precedes_the_prompt` hold it.
fn warn_active_elsewhere() {
    // Best effort, as a statement and never a gate (item 11): a write that
    // fails is not reported and the restore goes on, so from a pipe or a
    // file a stderr that cannot be written loses the warning silently, and
    // a stderr pointed at nowhere (closed, which the runtime reopens on
    // /dev/null, or /dev/null itself) loses it on every input path.
    let _ = std::io::stderr().write_all(ACTIVE_ELSEWHERE.as_bytes());
}

/// The drill: no lock, no key read, no write, no profile, no network
/// (IDENTITY-RECOVERY.md §Verify-only) -- only the comparison.
fn verify(recovery: &phrase::Recovery) -> Result<String, Failure> {
    ProfileIdentity::verify_phrase(&recovery.phrase, &recovery.expected).map_err(mismatch)?;
    Ok(format!(
        "verified: the phrase restores {}\n",
        recovery.expected.as_str()
    ))
}

/// A refusal from the identity crate. A mismatch names both `PeerId`s,
/// which are public; nothing it names is the phrase.
fn mismatch(e: IdentityError) -> Failure {
    match e {
        IdentityError::PeerIdMismatch { got, expected } => Failure::Refused(format!(
            "the phrase restores {got}, not {expected}: a different identity"
        )),
        other => Failure::Refused(other.to_string()),
    }
}
