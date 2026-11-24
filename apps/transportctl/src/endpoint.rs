// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The offline endpoint-key commands (ADR-0067 decision 4, ENDPOINTS.md
//! section Configured endpoint set): `endpoint init` mints an endpoint's
//! key pair, `endpoint adopt` brings over a key a release before Stage
//! 22's sealed under a label. Never a socket, never the network, and
//! never a write to `config.yaml`: each prints the entry, and the person,
//! the operator or the deployment's renderer writes it.
//!
//! A release before Stage 22 keyed `endpoint-overlay.json` by the label,
//! and the loader refuses such a member once an entry carries that alias
//! (`EndpointOverlayError::LabelMember`). So each command given an alias
//! also moves that label's overlay members to the id
//! (`EndpointOverlay::carry_label_within`), and does it BEFORE the step
//! that tells a rerun it has nothing left to do, so that a run cut short
//! anywhere is finished by running the same command again.
//!
//! Neither takes the profile lock, unlike `identity init`. The node key
//! that command writes is the one the running daemon holds; an endpoint
//! key is read by the daemon only at its load, through the entry naming
//! it, so a key minted or adopted beside a running daemon changes nothing
//! until an entry names it and the daemon restarts. Two runs racing are
//! decided by the exclusive create, which never replaces a file.
//!
//! Neither reads or prints the secret, except that `adopt` decodes the
//! old record whole -- its body is the secret at filesystem custody --
//! to copy it into the new file and to derive the public key from it,
//! held in `Zeroizing` throughout.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use ed25519_dalek::SigningKey;
use interweave_profile_config::endpoint_key::{
    self, Custody, KeyHeader, LEGACY_IDENTITY_KEYS_DIR, MAX_RECORD_BYTES, RawRecord,
};
use interweave_profile_config::endpoint_overlay::{Carried, EndpointOverlay};
use interweave_profile_config::{
    EndpointAlias, EndpointConfig, LoadError, PersistError, ProfileConfig, ProfilePaths, XdgRoots,
    read_private_within,
};
use interweave_transport_api::EndpointId;
use zeroize::Zeroizing;

use crate::Failure;
use crate::cli::Endpoint;

/// Run an endpoint-key command; what to print on success.
///
/// # Errors
/// [`Failure::Refused`], naming why -- and never a byte of a secret.
pub(crate) fn run(command: Endpoint) -> Result<String, Failure> {
    match command {
        Endpoint::Init {
            profile,
            alias,
            json,
        } => init(&profile, alias.as_ref(), json),
        Endpoint::Adopt {
            profile,
            alias,
            json,
        } => adopt(&profile, &alias, json),
    }
}

fn refused(what: &str, e: &dyn std::fmt::Display) -> Failure {
    Failure::Refused(format!("{what}: {e}"))
}

/// The profile's paths, offline: no runtime directory is needed.
fn paths(name: &str) -> Result<ProfilePaths, Failure> {
    let roots = XdgRoots::from_env().map_err(|e| refused("the XDG directories", &e))?;
    ProfilePaths::resolve_offline(name, &roots).map_err(crate::profile_refusal)
}

/// The entry to write, as printed: the `EndpointId` alone on the first
/// line, then the entry's `key` and `alias` members as `config.yaml`
/// writes them -- or, with `--json`, one object.
fn entry(id: &EndpointId, alias: Option<&EndpointAlias>, json: bool) -> String {
    let key = endpoint_key::key_file_name(id);
    if json {
        let value = serde_json::json!({
            "id": id.as_str(),
            "key": key,
            "alias": alias.map(EndpointAlias::as_str),
        });
        return format!("{value}\n");
    }
    let mut out = format!("{}\nkey: {key}\n", id.as_str());
    if let Some(alias) = alias {
        let _ = writeln!(out, "alias: {alias}");
    }
    out
}

/// The overlay's members for `alias` moved to `id`, for an entry already
/// in `config.yaml` that something other than this command wrote (by
/// hand, or by a run that stopped after writing it). `Nothing` found to
/// move is the caller's own refusal, `otherwise`.
fn carry_for_entry(
    paths: &ProfilePaths,
    alias: &EndpointAlias,
    id: &EndpointId,
    json: bool,
    otherwise: Failure,
) -> Result<String, Failure> {
    match carry(paths, alias, id)? {
        Carried::Moved => Ok(entry(id, Some(alias), json)),
        Carried::Nothing => Err(otherwise),
    }
}

fn carry(paths: &ProfilePaths, alias: &EndpointAlias, id: &EndpointId) -> Result<Carried, Failure> {
    EndpointOverlay::carry_label_within(
        &EndpointOverlay::path_for(paths),
        paths.boundary(),
        alias,
        id,
    )
    .map_err(|e| refused("carrying the endpoint overlay", &e))
}

/// The entry of the composed configuration carrying `alias`: `config.yaml`
/// with the endpoint overlay applied, as `admin.rs` composes it, so an
/// entry the overlay `added` is seen as well as an authored one. An overlay
/// that does not read leaves `config.yaml`'s entries alone, with a note.
fn entry_with_alias(
    paths: &ProfilePaths,
    config: &ProfileConfig,
    alias: &EndpointAlias,
) -> Option<EndpointConfig> {
    let endpoints = match crate::admin::composed(paths) {
        Ok((composed, overlay)) => overlay.apply(&composed.endpoints),
        Err(why) => {
            eprintln!(
                "transportctl: note: the alias was not checked against the endpoint overlay: {why}"
            );
            config.endpoints.clone()
        }
    };
    endpoints
        .entries
        .into_iter()
        .find(|e| e.alias.as_ref() == Some(alias))
}

/// Mint a new endpoint key at filesystem custody and print its id and
/// the entry naming it (ADR-0067 decision 4).
///
/// An alias an entry of the composed configuration already carries is
/// refused before anything is minted -- when `config.yaml` is there and
/// loads without its keys -- unless the overlay still holds members under
/// that label, in which case the entry was written without the carry and
/// this run carries them to the entry's id. A `config.yaml` that does not
/// load is the case a deployment migrating from labels is in (its entries
/// still carry `id`), so the mint goes on and stderr says the alias was
/// not checked; the daemon's load refuses a duplicate alias by name.
///
/// With an alias, the key is minted into `endpoint-keys/.pending-<alias>.key`
/// and published as `<id>.key` only after the overlay's members for the
/// alias are on the id ([`init_with_alias`]); without one it is minted
/// straight to `<id>.key`.
fn init(name: &str, alias: Option<&EndpointAlias>, json: bool) -> Result<String, Failure> {
    let paths = paths(name)?;
    let Some(alias) = alias else {
        let id = endpoint_key::mint_filesystem(&paths)
            .map_err(|e| refused("minting the endpoint key", &e))?;
        return Ok(entry(&id, None, json));
    };
    match ProfileConfig::load_document(&paths) {
        Ok(config) => {
            if let Some(taken) = entry_with_alias(&paths, &config, alias) {
                let refusal = Failure::Refused(format!(
                    "the alias '{alias}' is taken: the entry with key {} carries it",
                    taken.key_file_name()
                ));
                return carry_for_entry(&paths, alias, &taken.id, json, refusal);
            }
        }
        Err(LoadError::Read(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => eprintln!(
            "transportctl: note: the alias was not checked against {}, which does not load: {e}",
            paths.config_file().display()
        ),
    }
    init_with_alias(&paths, alias, json)
}

/// `init --alias`: the key, then the carry, then the key's name.
///
/// IN THIS ORDER, so that a run cut short anywhere ends, when run again,
/// with the overlay's members on the id of the ONE key file and nothing
/// else minted: the key is read from the pending file when a run left
/// one (and minted into it when none), the members are carried to its
/// id, and only then is the file renamed to `<id>.key` -- by a rename
/// that never replaces. The carry precedes the rename because once the
/// file has its id's name nothing finds it by the alias.
/// `tests/endpoint.rs` `init_rerun_after_an_interruption_mints_nothing`.
fn init_with_alias(
    paths: &ProfilePaths,
    alias: &EndpointAlias,
    json: bool,
) -> Result<String, Failure> {
    let pending = endpoint_key::pending_key_path(paths, alias);
    let shown = pending.display().to_string();
    let id = match read_private_within(&pending, paths.boundary(), MAX_RECORD_BYTES)
        .map_err(|e| refused(&shown, &e))?
    {
        Some(line) => pending_id(&Zeroizing::new(line), &shown)?,
        None => endpoint_key::mint_filesystem_at(paths, |_| pending.clone())
            .map_err(|e| refused("minting the endpoint key", &e))?,
    };
    carry(paths, alias, &id).map_err(|f| match f {
        Failure::Refused(m) => Failure::Refused(format!(
            "{m}; the key is kept in {shown}, and a second run finishes"
        )),
        other @ Failure::Unreachable(_) => other,
    })?;
    let new = endpoint_key::key_path(paths, &id);
    // Said BEFORE the rename: once the file has its id's name a rerun no
    // longer finds it by the alias, and a run cut short after the rename
    // would otherwise leave the overlay on an id nobody was told
    // (`init_reports_the_id_before_it_publishes`).
    eprintln!(
        "transportctl: the key for '{alias}' is {}; publishing it as {}",
        id.as_str(),
        new.display()
    );
    match interweave_profile_config::persist::rename_private_no_replace_within(
        &pending,
        &new,
        paths.boundary(),
    ) {
        Ok(()) => {}
        // Published; only its name's durability is in doubt.
        Err(PersistError::Unsynced(e)) => eprintln!(
            "transportctl: warning: {} is written, but its directory could not be synced: {e}",
            new.display()
        ),
        // The `link` fallback (a filesystem without RENAME_NOREPLACE)
        // cut short between its link and its unlink leaves the key under
        // both names: the same bytes there finish the run, anything else
        // is refused and both are kept
        // (`init_finishes_when_the_key_is_under_both_names`).
        Err(PersistError::AlreadyExists) => {
            let same = |p: &Path| {
                read_private_within(p, paths.boundary(), MAX_RECORD_BYTES)
                    .ok()
                    .flatten()
                    .map(Zeroizing::new)
            };
            match (same(&pending), same(&new)) {
                (Some(a), Some(b)) if a == b => {
                    if let Err(e) = interweave_profile_config::persist::remove_private_within(
                        &pending,
                        paths.boundary(),
                    ) {
                        eprintln!(
                            "transportctl: warning: {shown} is a second name of {}, and could \
                             not be removed: {e}",
                            new.display()
                        );
                    }
                }
                _ => {
                    return Err(Failure::Refused(format!(
                        "{} is there already and holds another key: {shown} is kept, and \
                         nothing was replaced",
                        new.display()
                    )));
                }
            }
        }
        Err(e) => {
            return Err(Failure::Refused(format!(
                "publishing {}: {e}; the key is kept in {shown}, and a second run finishes",
                new.display()
            )));
        }
    }
    Ok(entry(&id, Some(alias), json))
}

/// The id of the key in a pending file a run left: a filesystem-custody
/// record whose header names the id its public key derives and whose body
/// is the secret of that key. Anything else is not a key this command
/// minted, and is left where it is.
fn pending_id(line: &Zeroizing<String>, shown: &str) -> Result<EndpointId, Failure> {
    let header = KeyHeader::from_line(line).map_err(|e| refused(shown, &e))?;
    let bytes = endpoint_key::decode_line(line).map_err(|e| refused(shown, &e))?;
    let raw = RawRecord::parse(&bytes).map_err(|e| refused(shown, &e))?;
    let secret: Option<[u8; 32]> = raw.body.try_into().ok();
    let genuine = secret.is_some_and(|secret| {
        SigningKey::from_bytes(&Zeroizing::new(secret))
            .verifying_key()
            .to_bytes()
            == *header.public_key()
    });
    if header.custody() != Custody::FilesystemOnly || !genuine {
        return Err(Failure::Refused(format!(
            "{shown} is not a filesystem-custody key record of its own public key; it is \
             kept, and nothing was minted"
        )));
    }
    Ok(header.endpoint().clone())
}

/// `<state_dir>/identity-keys/<label>.key`: the file a release before
/// Stage 22's sealed an identity's key in. The alias grammar has no
/// separator and no leading dot, so this stays one entry of that
/// directory.
fn legacy_path(paths: &ProfilePaths, alias: &EndpointAlias) -> PathBuf {
    paths
        .state_dir()
        .join(LEGACY_IDENTITY_KEYS_DIR)
        .join(format!("{alias}{}", endpoint_key::KEY_FILE_SUFFIX))
}

/// Bring over the key a release before Stage 22's sealed under `alias`
/// (ENDPOINTS.md section Configured endpoint set, A 2026-11-19).
///
/// IN THIS ORDER, and the order is the point: the old record is read and
/// judged; the new file is written by the exclusive create, which syncs
/// the file and its directory; the new file is read back and must hold
/// the same key; the overlay's members for the label are carried to the
/// id; only then is the old file removed, and its directory synced.
/// Interrupted anywhere, both files or the old one alone are left, and a second run finishes from either: a new file already there
/// is checked, never replaced, and nothing is minted.
/// `tests/endpoint.rs`
/// `a_failed_write_leaves_the_old_file` fails if the removal comes first.
fn adopt(name: &str, alias: &EndpointAlias, json: bool) -> Result<String, Failure> {
    let paths = paths(name)?;
    let old = legacy_path(&paths, alias);
    let shown = old.display().to_string();
    let Some(line) = read_private_within(&old, paths.boundary(), MAX_RECORD_BYTES)
        .map_err(|e| refused(&shown, &e))?
    else {
        return nothing_to_adopt(&paths, alias, &shown, json);
    };
    let line = Zeroizing::new(line);
    let bytes = endpoint_key::decode_line(&line).map_err(|e| refused(&shown, &e))?;
    let raw = RawRecord::parse(&bytes).map_err(|e| refused(&shown, &e))?;
    if raw.endpoint != alias.as_str().as_bytes() {
        // What the header names is not repeated: it is whatever was
        // written there.
        return Err(Failure::Refused(format!(
            "{shown}: the record's header names another endpoint than '{alias}'; nothing was changed"
        )));
    }
    if raw.custody == Custody::Keystore {
        return Err(Failure::Refused(format!(
            "{shown} is sealed under the Keystore's custody: endpoint adopt takes a \
             filesystem-custody record alone (on Android the embedded host re-seals it in \
             process); nothing was changed"
        )));
    }
    // At filesystem custody the body is the secret itself.
    let secret: Zeroizing<[u8; 32]> = Zeroizing::new(raw.body.try_into().map_err(|_| {
        Failure::Refused(format!(
            "{shown}: the body of a filesystem-custody record is not a 32-byte secret; nothing \
             was changed"
        ))
    })?);
    if SigningKey::from_bytes(&secret).verifying_key().to_bytes() != raw.public_key {
        return Err(Failure::Refused(format!(
            "{shown}: the header's public key is not the secret's: the record was altered; \
             nothing was changed"
        )));
    }
    let id = EndpointId::from_ed25519_public(&raw.public_key);
    let header = endpoint_key::header_bytes(
        Custody::FilesystemOnly,
        id.as_str().as_bytes(),
        &raw.public_key,
    )
    .map_err(|e| refused("the new record's header", &e))?;
    let record = endpoint_key::encode_record(&header, secret.as_slice());
    let new = endpoint_key::key_path(&paths, &id);
    let mut resumed = false;
    match interweave_profile_config::create_private_exclusive_within(
        &new,
        record.as_bytes(),
        paths.boundary(),
    ) {
        // A file already there is a run interrupted after its write:
        // judged below, never replaced.
        Ok(()) => {}
        Err(PersistError::AlreadyExists) => resumed = true,
        Err(e) => {
            return Err(Failure::Refused(format!(
                "writing {}: {e}; {shown} is kept, and a second run finishes",
                new.display()
            )));
        }
    }
    same_key(&paths, &new, &id, &raw.public_key, &secret)?;
    // BEFORE the old file goes: the old file is what tells a rerun it has
    // work left, so a carry that fails leaves it, and a second run
    // finishes.
    carry(&paths, alias, &id).map_err(|f| match f {
        Failure::Refused(m) => Failure::Refused(format!(
            "{m}; {shown} is kept and {} is written, and a second run finishes",
            new.display()
        )),
        other @ Failure::Unreachable(_) => other,
    })?;
    if resumed {
        // The exclusive create answers AlreadyExists BEFORE it syncs the
        // directory, so a file an earlier run published may never have
        // had its name made durable (that run was refused Unsynced, or
        // was killed between the publish and the sync). Nothing else
        // syncs it, and the old file goes next: sync here, or keep the
        // old file. Tested for the refusal (a_resumed_adopt_that_cannot_sync_...);
        // that the sync reaches the disk is not something a test can see.
        if let Err(e) = sync_dir(new.parent().unwrap_or(Path::new("."))) {
            return Err(Failure::Refused(format!(
                "syncing {}: {e}; {shown} is kept, and a second run finishes",
                new.display()
            )));
        }
    }
    match interweave_profile_config::persist::remove_private_within(&old, paths.boundary()) {
        Ok(_) => {}
        // The old file is gone and its absence may not survive a crash;
        // if it comes back, a second run finds the new file and removes
        // it again.
        Err(PersistError::Unsynced(e)) => eprintln!(
            "transportctl: warning: {shown} is removed, but its directory could not be synced: {e}"
        ),
        Err(e) => {
            return Err(Failure::Refused(format!(
                "removing {shown}: {e}; {} is written, and a second run removes it",
                new.display()
            )));
        }
    }
    Ok(entry(&id, Some(alias), json))
}

/// fsync the directory `dir`, the one `profile-config` does not export a
/// way to sync on its own. `dir` is the parent of a file `same_key` has
/// just read through the boundary.
fn sync_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::File::open(dir)?.sync_all()
}

/// The file at `new` holds exactly the adopted key: its header derives
/// `id` and names `public_key`, at filesystem custody, and its body is
/// `secret`. Asked of a file just written as of one a previous run left,
/// so the old file is removed only beside a copy that holds its key.
fn same_key(
    paths: &ProfilePaths,
    new: &Path,
    id: &EndpointId,
    public_key: &[u8; 32],
    secret: &[u8; 32],
) -> Result<(), Failure> {
    let shown = new.display().to_string();
    let differs = || {
        Failure::Refused(format!(
            "{shown} is there and holds another key than the one adopted; both files are kept"
        ))
    };
    let line = read_private_within(new, paths.boundary(), MAX_RECORD_BYTES)
        .map_err(|e| refused(&shown, &e))?
        .map(Zeroizing::new)
        .ok_or_else(|| Failure::Refused(format!("{shown} is not there after its write")))?;
    let header = KeyHeader::from_line(&line).map_err(|e| refused(&shown, &e))?;
    if header.endpoint() != id
        || header.endpoint().ed25519_public().as_ref() != Some(public_key)
        || header.custody() != Custody::FilesystemOnly
    {
        return Err(differs());
    }
    let bytes = endpoint_key::decode_line(&line).map_err(|e| refused(&shown, &e))?;
    let raw = RawRecord::parse(&bytes).map_err(|e| refused(&shown, &e))?;
    if raw.body != secret.as_slice() {
        return Err(differs());
    }
    Ok(())
}

/// No `identity-keys/<label>.key`: nothing to adopt, and never a mint in
/// its place, so that a mistyped alias cannot make a new identity where
/// an old one was meant. When `config.yaml` loads and an entry carries
/// the alias, it is an adopt that finished, or a key minted or an entry
/// written by hand for it: the overlay's members for the label are
/// carried to that entry's id when there are any (an entry written
/// without the carry, which the loader refuses until it is done), and
/// otherwise the refusal names that entry's key instead of sending the
/// operator to mint another one, which `endpoint init` would refuse for
/// the alias.
fn nothing_to_adopt(
    paths: &ProfilePaths,
    alias: &EndpointAlias,
    shown: &str,
    json: bool,
) -> Result<String, Failure> {
    let configured = ProfileConfig::load_document(paths).ok().and_then(|config| {
        entry_with_alias(paths, &config, alias).map(|e| (e.id.clone(), e.key_file_name()))
    });
    match configured {
        Some((id, key)) => carry_for_entry(
            paths,
            alias,
            &id,
            json,
            Failure::Refused(format!(
                "nothing to adopt: {shown} is not there, and config.yaml's entry with the alias \
                 '{alias}' already names key {key}"
            )),
        ),
        None => Err(Failure::Refused(format!(
            "nothing to adopt: {shown} is not there. An endpoint with no sealed key gets a new \
             key, and a new id, from `transportctl endpoint init --profile {} --alias {alias}`",
            paths.profile()
        ))),
    }
}
