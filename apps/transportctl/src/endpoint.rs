// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The offline endpoint-key commands (ADR-0067 decision 4, ENDPOINTS.md
//! section Configured endpoint set): `endpoint init` mints an endpoint's
//! key pair, `endpoint adopt` brings over a key a release before Stage
//! 22's sealed under a label. Never a socket, never the network, and
//! never a write to `config.yaml`: each prints the entry, and the person,
//! the operator or the deployment's renderer writes it.
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
use interweave_profile_config::endpoint_overlay::EndpointOverlay;
use interweave_profile_config::{
    EndpointAlias, LoadError, PersistError, ProfileConfig, ProfilePaths, XdgRoots,
    read_private_within, resolve_endpoint,
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

/// Mint a new endpoint key at filesystem custody and print its id and
/// the entry naming it (ADR-0067 decision 4).
///
/// An alias an entry of the composed configuration already carries is
/// refused before anything is minted -- when `config.yaml` is there and
/// loads without its keys. One that does not load is the case a
/// deployment migrating from labels is in (its entries still carry `id`),
/// so the mint goes on and stderr says the alias was not checked; the
/// daemon's load refuses a duplicate alias by name.
fn init(name: &str, alias: Option<&EndpointAlias>, json: bool) -> Result<String, Failure> {
    let paths = paths(name)?;
    if let Some(alias) = alias {
        match ProfileConfig::load_document(&paths) {
            Ok(config) => {
                let (overlay, _) = EndpointOverlay::read_within(
                    &EndpointOverlay::path_for(&paths),
                    &config.endpoints,
                    paths.boundary(),
                )
                .map_err(|e| refused("the endpoint overlay", &e))?;
                if let Ok(taken) = resolve_endpoint(&config.endpoints, &overlay, alias.as_str()) {
                    return Err(Failure::Refused(format!(
                        "the alias '{alias}' is taken: the entry with key {} carries it",
                        endpoint_key::key_file_name(&taken)
                    )));
                }
            }
            Err(LoadError::Read(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => eprintln!(
                "transportctl: note: the alias was not checked against {}, which does not load: {e}",
                paths.config_file().display()
            ),
        }
    }
    let id = endpoint_key::mint_filesystem(&paths)
        .map_err(|e| refused("minting the endpoint key", &e))?;
    Ok(entry(&id, alias, json))
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
/// the same key; only then is the old file removed, and its directory
/// synced. Interrupted anywhere, both files or the old one alone are
/// left, and a second run finishes from either: a new file already there
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
        return Err(nothing_to_adopt(&paths, alias, &shown));
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
    match interweave_profile_config::create_private_exclusive_within(
        &new,
        record.as_bytes(),
        paths.boundary(),
    ) {
        // A file already there is a run interrupted after its write:
        // judged below, never replaced.
        Ok(()) | Err(PersistError::AlreadyExists) => {}
        Err(e) => {
            return Err(Failure::Refused(format!(
                "writing {}: {e}; {shown} is kept, and a second run finishes",
                new.display()
            )));
        }
    }
    same_key(&paths, &new, &id, &raw.public_key, &secret)?;
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
/// the alias, the refusal names that entry's key -- an adopt that
/// finished, or a key minted for it -- instead of sending the operator to
/// mint another one, which `endpoint init` would refuse for the alias.
fn nothing_to_adopt(paths: &ProfilePaths, alias: &EndpointAlias, shown: &str) -> Failure {
    let configured = ProfileConfig::load_document(paths).ok().and_then(|config| {
        config
            .endpoints
            .entries
            .iter()
            .find(|e| e.alias.as_ref() == Some(alias))
            .map(interweave_profile_config::EndpointConfig::key_file_name)
    });
    match configured {
        Some(key) => Failure::Refused(format!(
            "nothing to adopt: {shown} is not there, and config.yaml's entry with the alias \
             '{alias}' already names key {key}"
        )),
        None => Failure::Refused(format!(
            "nothing to adopt: {shown} is not there. An endpoint with no sealed key gets a new \
             key, and a new id, from `transportctl endpoint init --profile {} --alias {alias}`",
            paths.profile()
        )),
    }
}
