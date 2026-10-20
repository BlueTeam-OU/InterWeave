// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Reading a private state file: the one reader the trust overlay, the
//! availability overlay and the profile-format marker share, so the rules
//! a file is judged by -- the file regular, not a link, this uid's,
//! readable by nobody else, bounded in size, and for
//! [`read_private_within`] its directory private under the boundary --
//! are written once.

use std::io::Read as _;
use std::path::Path;

use crate::{PersistError, TrustBoundary, persist};

/// Why a private file cannot be read; each caller maps it into its own
/// error, which names the file.
#[derive(Debug)]
pub(crate) enum PrivateReadError {
    /// The file is present and could not be read.
    Read(std::io::Error),
    /// The file, or its directory, is not private.
    NotPrivate {
        /// What is wrong with it.
        detail: String,
    },
    /// The file is larger than the caller's bound.
    TooLarge,
}

/// The text of the private file at `path`, its directory resolved under
/// `boundary`: `None` when the file or its directory is absent.
pub(crate) fn read_private_within(
    path: &Path,
    boundary: &TrustBoundary,
    max_bytes: u64,
) -> Result<Option<String>, PrivateReadError> {
    // Read under the state directory AS RESOLVED (ADR-0028 A
    // 2026-10-08), as the overlays' writes are: `ProfileLock` judged the
    // same directory before load, and this resolves it once more so
    // the read opens where that judgement led. An absent directory
    // holds no file.
    let dir = match persist::resolve_private_dir_within(persist::parent_dir(path), boundary) {
        Ok(dir) => dir,
        Err(PersistError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(PersistError::Io(e)) => return Err(PrivateReadError::Read(e)),
        Err(e) => {
            return Err(PrivateReadError::NotPrivate {
                detail: e.to_string(),
            });
        }
    };
    let resolved = dir.join(persist::file_name(path).map_err(|_| {
        PrivateReadError::Read(std::io::Error::from(std::io::ErrorKind::InvalidInput))
    })?);
    read_bounded(&resolved, max_bytes)
}

/// The text of the private file at `path`, the FILE judged and its
/// directory left to the profile lock.
///
/// The entry is looked up first, without following it, and that answer
/// decides whose failure a failure is. An entry that EXISTS is judged as
/// a file, and every failure from then on -- another account's, open to
/// others, a link, unreadable, too large -- is the file's, never an
/// absence. An entry that does not exist is `None`. A lookup the PATH
/// refuses -- a directory on it this uid cannot search, a file where a
/// directory should be, a loop of links -- is the directory's, and
/// `None` too: the profile lock that follows refuses that directory, in
/// the class each binary has always given it, and a load that takes no
/// lock does not fail on a directory it never writes. Any other lookup
/// failure is a read error.
///
/// For a reader whose answer a writable directory cannot turn into a
/// grant -- the profile-format marker's check at load. Only this uid can
/// own a file the judgement accepts, so another account that can change
/// the directory or a link on its path can make the read find no file
/// (as deleting it would), a file of its own (refused as another
/// account's) or one of this uid's from elsewhere: each a refusal or an
/// absence, never a grant
/// (`a_marker_in_a_state_directory_others_can_read_is_still_judged`,
/// `a_state_directory_the_load_cannot_search_is_left_to_the_lock`).
pub(crate) fn read_private_file(
    path: &Path,
    max_bytes: u64,
) -> Result<Option<String>, PrivateReadError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => read_bounded(path, max_bytes),
        Err(e) if path_refused(&e) => Ok(None),
        Err(e) => Err(PrivateReadError::Read(e)),
    }
}

/// Whether a lookup failed on the PATH to the entry, or found no entry:
/// absent, a component this uid cannot search, a component that is not
/// a directory, a loop of links.
fn path_refused(e: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    matches!(
        e.kind(),
        ErrorKind::NotFound | ErrorKind::PermissionDenied | ErrorKind::NotADirectory
    ) || is_link_loop(e)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn is_link_loop(e: &std::io::Error) -> bool {
    e.raw_os_error() == Some(libc::ELOOP)
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn is_link_loop(_e: &std::io::Error) -> bool {
    false
}

/// `path` opened by [`open_private`] and read to at most `max_bytes`:
/// `None` when it, or a directory on its way, is absent.
fn read_bounded(path: &Path, max_bytes: u64) -> Result<Option<String>, PrivateReadError> {
    let file = match open_private(path) {
        Ok(file) => file,
        Err(PrivateReadError::Read(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    let mut text = String::new();
    file.take(max_bytes + 1)
        .read_to_string(&mut text)
        .map_err(PrivateReadError::Read)?;
    if text.len() as u64 > max_bytes {
        return Err(PrivateReadError::TooLarge);
    }
    Ok(Some(text))
}

/// `text` parsed as `T` from a JSON OBJECT and nothing else. serde's
/// derived struct visitor also accepts a sequence -- the fields in
/// order -- so `from_str` alone would read `[[],[]]` as a trust overlay
/// and `["stay-reachable"]` as the availability choice: a second shape
/// neither file is written in. Pinned by the array cases of
/// `a_present_overlay_that_cannot_be_trusted_stops_the_load` and
/// `a_file_naming_anything_but_the_choice_is_refused`.
///
/// The struct is read from the TEXT, not from the parsed value: a
/// `Value` map keeps the last of a key named twice, which would drop the
/// first list silently where the derived visitor refuses a duplicate
/// field (the "named twice" cases of the same two tests).
pub(crate) fn object_only<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if !value.is_object() {
        return Err("not a JSON object".to_owned());
    }
    serde_json::from_str(text).map_err(|e| e.to_string())
}

/// The uid a private file must be owned by, or -- this process's uid
/// unreadable -- a refusal on READ: the owner cannot be checked, so the
/// file cannot be trusted; never a write failure (#215 review P3).
/// `an_unreadable_uid_refuses_the_file_on_read`.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn owner_uid(read: Result<u32, PersistError>) -> Result<u32, PrivateReadError> {
    read.map_err(|_| PrivateReadError::NotPrivate {
        detail: "its owner cannot be checked: this process's uid is unreadable".to_owned(),
    })
}

/// Open `path` for reading only if it is a regular file, not a link,
/// owned by this process's uid and readable or writable by nobody else
/// -- the identity key's rule. Judged on the OPENED file, so the file
/// checked is the file read.
///
/// Its DIRECTORY is judged and resolved by the caller
/// ([`read_private_within`]), which hands this the path under the
/// directory as resolved -- or left to the profile lock
/// ([`read_private_file`]).
fn open_private(path: &Path) -> Result<std::fs::File, PrivateReadError> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        open_private_as(path, owner_uid(persist::effective_uid())?)
    }
    // Elsewhere ownership and mode cannot be checked here, so a present
    // overlay is refused -- but an absent one is still the empty overlay,
    // not a reason not to start.
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        match std::fs::symlink_metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(PrivateReadError::Read(e)),
            _ => Err(PrivateReadError::NotPrivate {
                detail: "owner-only permissions cannot be checked on this platform".to_owned(),
            }),
        }
    }
}

/// [`open_private`] for the account `uid`: apart so a test can judge a
/// file of this process's for another account, which no unprivileged
/// test can create (`a_file_owned_by_another_account_is_refused`).
#[cfg(any(target_os = "linux", target_os = "android"))]
fn open_private_as(path: &Path, uid: u32) -> Result<std::fs::File, PrivateReadError> {
    use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _};
    // O_NONBLOCK so a FIFO in its place opens at once and is refused
    // below as not a regular file, rather than holding start until a
    // writer appears; it changes nothing for a regular file
    // (`an_overlay_that_is_a_fifo_is_refused_without_waiting`).
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| {
            // ELOOP is O_NOFOLLOW meeting a link.
            if e.raw_os_error() == Some(libc::ELOOP) {
                PrivateReadError::NotPrivate {
                    detail: "it is a symbolic link".to_owned(),
                }
            } else {
                PrivateReadError::Read(e)
            }
        })?;
    let meta = file.metadata().map_err(PrivateReadError::Read)?;
    if !meta.file_type().is_file() {
        return Err(PrivateReadError::NotPrivate {
            detail: "it is not a regular file".to_owned(),
        });
    }
    if meta.uid() != uid {
        return Err(PrivateReadError::NotPrivate {
            detail: format!("owned by uid {}, not {uid}", meta.uid()),
        });
    }
    let mode = meta.mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(PrivateReadError::NotPrivate {
            detail: format!("mode is {mode:04o}, wider than 0600"),
        });
    }
    Ok(file)
}

#[cfg(all(test, any(target_os = "linux", target_os = "android")))]
mod tests {
    use super::{PersistError, PrivateReadError, open_private_as, owner_uid};

    #[test]
    fn an_unreadable_uid_refuses_the_file_on_read() {
        assert!(matches!(
            owner_uid(Err(PersistError::UnsupportedPlatform)),
            Err(PrivateReadError::NotPrivate { .. })
        ));
        // The control: a uid read is the owner checked against.
        assert_eq!(owner_uid(Ok(1000)).ok(), Some(1000));
    }

    #[test]
    fn a_file_owned_by_another_account_is_refused() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("private.json");
        std::fs::write(&path, b"{}").expect("write");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
        let ours = crate::persist::effective_uid().expect("this uid");
        // THE CONTROL: judged for its owner, the owner-only file opens.
        assert!(open_private_as(&path, ours).is_ok());
        let other = ours.wrapping_add(1);
        match open_private_as(&path, other) {
            Err(PrivateReadError::NotPrivate { detail }) => {
                assert!(
                    detail.contains(&format!("owned by uid {ours}, not {other}")),
                    "{detail}"
                );
            }
            Err(e) => panic!("refused as another account's: {e:?}"),
            Ok(_) => panic!("refused as another account's"),
        }
    }
}
