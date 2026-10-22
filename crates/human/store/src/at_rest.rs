// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The store at rest: its files by name, the plaintext-to-encrypted
//! rewrite, the health a client reports, and Discard and recreate
//! (ADR-0060 decision 6, with A 2026-10-21).
//!
//! # The files, and what each combination means
//!
//! In the profile's human directory: [`STORE_FILE`] with its `-wal`,
//! `-shm` and `-journal`, and the rewrite's two names, [`ENCRYPTING`] and
//! [`ENCRYPTED`]. The open tells every crash state apart by those names
//! alone ([`survey`]):
//!
//! | found | means | the open |
//! |---|---|---|
//! | `human.sqlite` plaintext, `.encrypting` | (a) did not finish | removes `.encrypting`, rewrites again |
//! | `human.sqlite` plaintext, `.encrypted` | (b) was under way | checks the copy opens, scrubs, then (c) |
//! | `.encrypted` alone | (b) finished | (c) |
//! | `human.sqlite` plaintext | never rewritten | (a), (b), (c) |
//! | `human.sqlite` not plaintext | the encrypted store | opens it under the key |
//! | nothing | a new store | a key, then the store |
//! | anything else | not one of these | recovery mode |
//!
//! # What the next open does with a file that is not plaintext
//!
//! In this order: the key opens it -- the store; the platform reports the
//! key invalidated -- [`StoreError::KeyInvalidated`], discard offered
//! directly; otherwise -- no key file, a key that does not open the file,
//! a damaged one, a combination of names the table does not list --
//! recovery mode ([`StoreError::needs_recovery`]), whose screen offers
//! Discard and recreate as a third, confirmed action. Nothing is deleted
//! without the person's word: only [`discard`] deletes, and it takes a
//! [`ConfirmedDiscard`], which only a [`DiscardPlan`] makes.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use interweave_profile_config as profile_config;
use interweave_profile_config::paths::ProfilePaths;
use interweave_profile_config::profile_format;
use rusqlite::{Connection, OpenFlags};

use crate::StoreError;
use crate::key_custody::{Custody, KeyCipher};
use crate::store::{
    HumanStore, STORE_FILE, StoreOptions, header_user_version, private_dir, private_parent_of,
};
use crate::store_key::{self, StoreKey, StoreKeyError, StoreKeyFileError};

/// The rewrite's temporary copy, while it is being written.
pub const ENCRYPTING: &str = "human.sqlite.encrypting";
/// The rewrite's complete copy: its name is the durable mark that one
/// exists (ADR-0060 decision 6 (a)).
pub const ENCRYPTED: &str = "human.sqlite.encrypted";

const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";
/// A write-ahead log's header, then one frame's header, then that frame's
/// page.
const WAL_HEADER: u64 = 32;
const WAL_FRAME_HEADER: u64 = 24;

/// Why the store needs recovery mode, by ADR-0060 decision 6's names. A
/// client shows recovery mode for each: Quit, Retry, and the confirmed
/// Discard and recreate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtRestRecovery {
    /// No key file beside a store that is not plaintext. On the desktop a
    /// lost key and a damaged never-rewritten plaintext leave the same
    /// files, and nothing tells them apart.
    KeyMissing,
    /// The key file holds no key this build can use: damaged, another
    /// version or custody, or the platform refused it other than by
    /// invalidation.
    KeyUnusable(StoreKeyError),
    /// The rewrite's complete copy does not open under the key, so the
    /// plaintext beside it is left as it is rather than scrubbed.
    CopyUnopenable,
    /// A combination of names decision 6 does not list.
    UnlistedFiles(&'static str),
}

impl core::fmt::Display for AtRestRecovery {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::KeyMissing => f.write_str("the store's key file is missing"),
            Self::KeyUnusable(e) => e.fmt(f),
            Self::CopyUnopenable => f.write_str(
                "the store's encrypted copy does not open under its key, \
                 so the plaintext beside it was left unchanged",
            ),
            Self::UnlistedFiles(what) => write!(f, "the store's files are not as expected: {what}"),
        }
    }
}

/// What the human directory holds, by the names alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreFiles {
    /// No store yet.
    None,
    /// A plaintext store, not yet rewritten.
    Plaintext,
    /// A store that is not plaintext: the encrypted store, or a damaged
    /// file, which only the key tells apart.
    Encrypted,
    /// The rewrite's first step did not finish: the plaintext is intact.
    ExportUnfinished,
    /// The rewrite's complete copy beside a plaintext still being scrubbed.
    ScrubUnfinished,
    /// The rewrite's complete copy alone, not yet renamed into place.
    RenameUnfinished,
    /// A combination ADR-0060 decision 6 does not list.
    Unlisted(&'static str),
}

/// The store key as the platform's custody holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyPresence {
    /// No key file.
    Absent,
    /// A key file whose key opens under the platform's custody.
    Present,
    /// The platform reports the key invalidated.
    Invalidated,
    /// A key file holding no key this build can use.
    Unusable(StoreKeyError),
}

/// The store at rest, as a client's health report shows it: the custody
/// that protects the key, whether the key is there, and what the files
/// are. Never a byte of the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtRestReport {
    /// The custody the platform seals the key under.
    pub protection: Custody,
    /// The key.
    pub key: KeyPresence,
    /// The files.
    pub files: StoreFiles,
}

/// The paths of one profile's store files.
struct Names {
    dir: PathBuf,
}

impl Names {
    fn of(paths: &ProfilePaths) -> Self {
        Self {
            dir: paths.human_dir(),
        }
    }

    fn db(&self) -> PathBuf {
        self.dir.join(STORE_FILE)
    }

    fn with(&self, suffix: &str) -> PathBuf {
        self.dir.join(format!("{STORE_FILE}{suffix}"))
    }

    fn encrypting(&self) -> PathBuf {
        self.dir.join(ENCRYPTING)
    }

    fn encrypted(&self) -> PathBuf {
        self.dir.join(ENCRYPTED)
    }

    /// The database and every companion SQLite may leave beside it.
    fn database_files(&self) -> [PathBuf; 4] {
        [
            self.db(),
            self.with("-wal"),
            self.with("-shm"),
            self.with("-journal"),
        ]
    }
}

fn present(path: &Path) -> Result<bool, StoreError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(StoreError::Io(e)),
    }
}

/// Read up to `buf.len()` bytes at `offset`; how many were read.
fn read_at(path: &Path, offset: u64, buf: &mut [u8]) -> Result<usize, StoreError> {
    use std::io::Seek as _;
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(StoreError::Io(e)),
    };
    file.seek(std::io::SeekFrom::Start(offset))
        .map_err(StoreError::Io)?;
    let mut read = 0;
    while read < buf.len() {
        match file.read(&mut buf[read..]).map_err(StoreError::Io)? {
            0 => break,
            n => read += n,
        }
    }
    Ok(read)
}

/// What the database file holds, from its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Content {
    /// No content anywhere: no file, or an empty one with no log.
    Nothing,
    /// SQLite's plaintext header, in the file or in the log's first page.
    Plaintext,
    /// Bytes that are not plaintext: ciphertext, or damage.
    Other,
}

/// The database's content by its header. A store whose every session ended
/// before a checkpoint -- an Android process killed is the normal end --
/// holds its first page only in the write-ahead log, with an empty file,
/// so an empty file is judged by the log's first frame. `SQLCipher` encrypts
/// that page as it encrypts the file's.
fn content(names: &Names) -> Result<Content, StoreError> {
    let mut head = [0_u8; 16];
    match read_at(&names.db(), 0, &mut head)? {
        16 if &head == SQLITE_HEADER => return Ok(Content::Plaintext),
        0 => {}
        _ => return Ok(Content::Other),
    }
    match read_at(
        &names.with("-wal"),
        WAL_HEADER + WAL_FRAME_HEADER,
        &mut head,
    )? {
        0 => Ok(Content::Nothing),
        16 if &head == SQLITE_HEADER => Ok(Content::Plaintext),
        _ => Ok(Content::Other),
    }
}

/// What the human directory holds, by the names decision 6 lists.
///
/// # Errors
/// [`StoreError::Io`] when a name cannot be looked up.
fn survey(names: &Names) -> Result<StoreFiles, StoreError> {
    let encrypting = present(&names.encrypting())?;
    let encrypted = present(&names.encrypted())?;
    let db = content(names)?;
    Ok(match (db, encrypting, encrypted) {
        (Content::Plaintext, true, false) => StoreFiles::ExportUnfinished,
        (_, true, _) => {
            StoreFiles::Unlisted("an unfinished copy without the plaintext it came from")
        }
        (Content::Plaintext, false, true) => StoreFiles::ScrubUnfinished,
        (Content::Nothing, false, true) => StoreFiles::RenameUnfinished,
        (Content::Other, false, true) => {
            StoreFiles::Unlisted("a complete encrypted copy beside a store that is not plaintext")
        }
        (Content::Plaintext, false, false) => StoreFiles::Plaintext,
        (Content::Other, false, false) => StoreFiles::Encrypted,
        (Content::Nothing, false, false) => StoreFiles::None,
    })
}

/// Give `conn` the key, as `SQLCipher`'s raw-key literal: a raw key, so the
/// engine derives nothing from it (the key is random, not a passphrase).
pub(crate) fn apply_key(conn: &Connection, key: &StoreKey) -> Result<(), StoreError> {
    let literal = key.sqlcipher_literal();
    let statement = zeroize::Zeroizing::new(format!("PRAGMA key = \"{}\";", literal.as_str()));
    conn.execute_batch(&statement)?;
    Ok(())
}

/// The key from its file: `Ok(None)` when there is no file.
fn read_key(paths: &ProfilePaths, cipher: &dyn KeyCipher) -> Result<Option<StoreKey>, StoreError> {
    match store_key::open(paths, cipher) {
        Ok(key) => Ok(key),
        Err(StoreKeyFileError::Key(e)) if e.is_invalidated() => Err(StoreError::KeyInvalidated),
        Err(StoreKeyFileError::Key(e)) => Err(StoreError::AtRest(AtRestRecovery::KeyUnusable(e))),
        Err(e) => Err(StoreError::KeyFile(e.to_string())),
    }
}

/// A key for a store that holds nothing under any key yet: the one in the
/// file when it opens, else a new one in its place. A key with no
/// ciphertext under it protects nothing, so replacing one that does not
/// open loses nothing.
fn key_for_new_ciphertext(
    paths: &ProfilePaths,
    cipher: &dyn KeyCipher,
) -> Result<StoreKey, StoreError> {
    if let Ok(Some(key)) = store_key::open(paths, cipher) {
        return Ok(key);
    }
    store_key::remove(paths, cipher).map_err(|e| StoreError::KeyFile(e.to_string()))?;
    store_key::create(paths, cipher).map_err(|e| StoreError::KeyFile(e.to_string()))
}

/// Open the profile's store under its key, rewriting a plaintext store
/// first and resuming a rewrite a crash stopped. [`HumanStore::open_profile`]
/// is this.
pub(crate) fn open(
    paths: &ProfilePaths,
    cipher: &dyn KeyCipher,
    options: StoreOptions,
) -> Result<HumanStore, StoreError> {
    let names = Names::of(paths);
    // The directory first, judged as the store judges it: every name below
    // is read and written under it as resolved.
    let dir = private_dir(private_parent_of(&names.db()), paths.boundary())?;
    let names = Names { dir };
    // A NEWER BUILD'S PLAINTEXT IS REFUSED BEFORE ANYTHING IS WRITTEN, the
    // marker included, by its header alone (STATE.md Migrations): the
    // refusal leaves every file as it was. An encrypted header names no
    // version; the keyed open's migration refuses that one.
    refuse_newer_plaintext(&names)?;
    let found = survey(&names)?;
    // A PLAINTEXT ABOUT TO BE REWRITTEN IS FIRST MIGRATED, as a plaintext
    // open migrates it: the copy is then this build's schema, and a
    // newer build's store -- its version in the header or only in the
    // log -- is refused here, before the marker or any copy is written
    // (#377 review F1 and its risk).
    if matches!(found, StoreFiles::Plaintext | StoreFiles::ExportUnfinished) {
        drop(HumanStore::open_within(
            &names.db(),
            paths.boundary(),
            options,
            None,
        )?);
    }
    // EVERY KEYED OPEN RAISES THE MARKER, a new store's too: a build from
    // before the encrypted store then refuses the profile by name, never
    // meeting a file it can only read as corruption (A 2026-10-21).
    profile_format::raise(paths).map_err(|e| StoreError::ProfileFormat(e.to_string()))?;
    match found {
        StoreFiles::ExportUnfinished => {
            remove(&names.encrypting())?;
            sync_dir(&names.dir)?;
            rewrite(paths, cipher, &names)?;
        }
        StoreFiles::Plaintext => rewrite(paths, cipher, &names)?,
        StoreFiles::ScrubUnfinished => {
            let key =
                read_key(paths, cipher)?.ok_or(StoreError::AtRest(AtRestRecovery::KeyMissing))?;
            if !copy_opens(&names.encrypted(), &key) {
                return Err(StoreError::AtRest(AtRestRecovery::CopyUnopenable));
            }
            scrub(&names)?;
            take_copy(&names)?;
        }
        StoreFiles::RenameUnfinished => take_copy(&names)?,
        StoreFiles::Unlisted(what) => {
            return Err(StoreError::AtRest(AtRestRecovery::UnlistedFiles(what)));
        }
        StoreFiles::Encrypted | StoreFiles::None => {}
    }
    let key = if content(&names)? == Content::Nothing {
        key_for_new_ciphertext(paths, cipher)?
    } else {
        read_key(paths, cipher)?.ok_or(StoreError::AtRest(AtRestRecovery::KeyMissing))?
    };
    HumanStore::open_within(&names.db(), paths.boundary(), options, Some(&key))
}

/// The profile's store at rest, without opening it or writing anything.
///
/// # Errors
/// [`StoreError`] when the directory or a name cannot be read.
pub fn inspect(paths: &ProfilePaths, cipher: &dyn KeyCipher) -> Result<AtRestReport, StoreError> {
    let names = Names::of(paths);
    let files = if present(&names.dir)? {
        let dir = profile_config::resolve_owned_private_dir_within(&names.dir, paths.boundary())
            .map_err(|e| StoreError::from_persist(&names.dir, e))?;
        survey(&Names { dir })?
    } else {
        StoreFiles::None
    };
    let key = match store_key::open(paths, cipher) {
        Ok(None) => KeyPresence::Absent,
        Ok(Some(_)) => KeyPresence::Present,
        Err(StoreKeyFileError::Key(e)) if e.is_invalidated() => KeyPresence::Invalidated,
        Err(StoreKeyFileError::Key(e)) => KeyPresence::Unusable(e),
        Err(e) => return Err(StoreError::KeyFile(e.to_string())),
    };
    Ok(AtRestReport {
        protection: cipher.custody(),
        key,
        files,
    })
}

/// A read-only connection to the profile's encrypted store under its key,
/// for diagnostics and for tests that count what a running client holds:
/// no migration, no rewrite, no write, and no marker raised. A store that
/// is not yet encrypted, or whose key does not open it, is refused as the
/// open would refuse it.
///
/// # Errors
/// [`StoreError`]: no store, no key, an invalidated or unusable key, or
/// the engine's refusal.
pub fn open_read_only(
    paths: &ProfilePaths,
    cipher: &dyn KeyCipher,
) -> Result<Connection, StoreError> {
    let names = Names::of(paths);
    let key = read_key(paths, cipher)?.ok_or(StoreError::AtRest(AtRestRecovery::KeyMissing))?;
    let conn = Connection::open_with_flags(
        names.db(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    apply_key(&conn, &key)?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))?;
    Ok(conn)
}

/// A WRITABLE connection to the encrypted store under its key, bypassing
/// every rule of [`HumanStore`]: for tests that must put a store into a
/// state no client writes (a newer header). Compiled only with the
/// `test-support` feature, which no client enables.
///
/// # Errors
/// As [`open_read_only`].
#[cfg(feature = "test-support")]
pub fn open_writable_for_tests(
    paths: &ProfilePaths,
    cipher: &dyn KeyCipher,
) -> Result<Connection, StoreError> {
    let names = Names::of(paths);
    let key = read_key(paths, cipher)?.ok_or(StoreError::AtRest(AtRestRecovery::KeyMissing))?;
    let conn = Connection::open_with_flags(names.db(), OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    apply_key(&conn, &key)?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))?;
    Ok(conn)
}

// ----- the rewrite (ADR-0060 decision 6 (a)-(c)) -----

/// The steps of the rewrite, each the point after which a crash leaves
/// the files the open resumes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewriteStep {
    /// The profile-format marker raised.
    MarkerRaised,
    /// The key file written and synced.
    KeyWritten,
    /// The copy exported and synced under its temporary name.
    Exported,
    /// The copy renamed to its complete name: the durable mark.
    Marked,
    /// The plaintext's rows deleted, before its vacuum.
    ScrubDeleted,
    /// The plaintext vacuumed and its log truncated.
    Scrubbed,
    /// The plaintext and its companions unlinked.
    Unlinked,
    /// The copy renamed into the store's name.
    Renamed,
}

#[cfg(feature = "crash-points")]
mod crash {
    //! A test build's crash points: the process ends where the variable
    //! names, with no destructor run and no further write, as a kill does.
    //! Compiled only with the `crash-points` feature, which no client
    //! enables; inert without the variable.

    /// The variable naming the step to end the process after.
    pub const VARIABLE: &str = "INTERWEAVE_STORE_CRASH_AFTER";
    /// The exit status a crash point ends with.
    pub const STATUS: i32 = 86;

    pub(super) fn reached(step: super::RewriteStep) {
        if std::env::var(VARIABLE).is_ok_and(|at| at == format!("{step:?}")) {
            std::process::exit(STATUS);
        }
    }
}

#[cfg(feature = "crash-points")]
pub use crash::{STATUS as CRASH_STATUS, VARIABLE as CRASH_VARIABLE};

#[cfg(feature = "crash-points")]
fn reached(step: RewriteStep) {
    crash::reached(step);
}

#[cfg(not(feature = "crash-points"))]
const fn reached(_: RewriteStep) {}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e)
}

fn sync(path: &Path) -> Result<(), StoreError> {
    std::fs::File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(io)
}

fn sync_dir(dir: &Path) -> Result<(), StoreError> {
    sync(dir)
}

fn remove(path: &Path) -> Result<(), StoreError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io(e)),
    }
}

/// (a), (b) and (c) over an intact plaintext.
fn rewrite(paths: &ProfilePaths, cipher: &dyn KeyCipher, names: &Names) -> Result<(), StoreError> {
    // BEFORE THE MARK, as `open` raised it: from here a build older than
    // this one refuses the profile by its marker, naming both numbers, and
    // never meets the encrypted file's `NotADatabase` (A 2026-10-21).
    reached(RewriteStep::MarkerRaised);
    let key = key_for_new_ciphertext(paths, cipher)?;
    reached(RewriteStep::KeyWritten);
    export(names, &key)?;
    scrub(names)?;
    take_copy(names)
}

/// A plaintext store whose header says a newer build wrote it.
fn refuse_newer_plaintext(names: &Names) -> Result<(), StoreError> {
    if let Some(version) = header_user_version(&names.db())?
        && version > crate::schema::SCHEMA_VERSION
    {
        return Err(StoreError::Migration(format!(
            "database is at schema version {version}, newer than this build's {}; \
             refusing to downgrade",
            crate::schema::SCHEMA_VERSION
        )));
    }
    Ok(())
}

/// (a): the plaintext exported under the key to [`ENCRYPTING`], synced,
/// renamed [`ENCRYPTED`], the directory synced.
fn export(names: &Names, key: &StoreKey) -> Result<(), StoreError> {
    let encrypting = names.encrypting();
    // OWNER-ONLY FROM CREATION: `ATTACH` creates a missing database with
    // the process umask, 0644 on a default system (SPIKE-011), which the
    // store then refuses as `PermissionsTooOpen`.
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&encrypting)
            .map_err(io)?;
    }
    let plain = Connection::open(names.db())?;
    let version: i64 = plain.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let path = encrypting
        .to_str()
        .ok_or(StoreError::NotAFile {
            what: "the store's directory is not a UTF-8 path",
        })?
        .replace('\'', "''");
    let literal = key.sqlcipher_literal();
    let attach = zeroize::Zeroizing::new(format!(
        "ATTACH DATABASE '{path}' AS encrypted KEY \"{}\";",
        literal.as_str()
    ));
    plain.execute_batch(&attach)?;
    // `sqlcipher_export` copies the schema and the rows, not the header's
    // `user_version`, the store's schema version: carried by hand, or the
    // copy reads as version 0 and migrates from nothing (SPIKE-011).
    plain.execute_batch(&format!(
        "SELECT sqlcipher_export('encrypted');
         PRAGMA encrypted.user_version = {version};
         DETACH DATABASE encrypted;"
    ))?;
    drop(plain);
    sync(&encrypting)?;
    reached(RewriteStep::Exported);
    std::fs::rename(&encrypting, names.encrypted()).map_err(io)?;
    sync_dir(&names.dir)?;
    reached(RewriteStep::Marked);
    Ok(())
}

/// (b): the plaintext scrubbed while it still holds its name -- every row
/// deleted under `secure_delete`, the file vacuumed, its log truncated --
/// then `-wal`, `-shm`, `-journal` and the file unlinked, the directory
/// synced.
fn scrub(names: &Names) -> Result<(), StoreError> {
    if content(names)? == Content::Plaintext {
        let plain = Connection::open(names.db())?;
        plain.pragma_update(None, "secure_delete", true)?;
        let tables: Vec<String> = plain
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            )?
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        plain.pragma_update(None, "foreign_keys", false)?;
        let tx = plain.unchecked_transaction()?;
        for table in &tables {
            tx.execute_batch(&format!("DELETE FROM \"{}\";", table.replace('"', "\"\"")))?;
        }
        tx.commit()?;
        reached(RewriteStep::ScrubDeleted);
        plain.execute_batch("VACUUM;")?;
        let busy: i64 = plain.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))?;
        if busy != 0 {
            return Err(StoreError::LogNotTruncated);
        }
        drop(plain);
        sync(&names.db())?;
        reached(RewriteStep::Scrubbed);
    }
    for path in [
        names.with("-wal"),
        names.with("-shm"),
        names.with("-journal"),
        names.db(),
    ] {
        remove(&path)?;
    }
    sync_dir(&names.dir)?;
    reached(RewriteStep::Unlinked);
    Ok(())
}

/// (c): the complete copy renamed into the store's name.
fn take_copy(names: &Names) -> Result<(), StoreError> {
    std::fs::rename(names.encrypted(), names.db()).map_err(io)?;
    sync_dir(&names.dir)?;
    reached(RewriteStep::Renamed);
    Ok(())
}

/// Whether the copy at `path` opens read-only under `key` at a schema
/// version this build can migrate. Found by trying, as decision 6 orders:
/// a damaged copy and a key that does not open it both answer `false`.
/// NOT this build's exact shape: a copy an earlier build left stalled is
/// at that build's version and is migrated by the open that promotes it
/// -- requiring the current shape stranded it in recovery mode and made
/// the discard take it (#377 review F1).
fn copy_opens(path: &Path, key: &StoreKey) -> bool {
    let Ok(conn) = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return false;
    };
    if apply_key(&conn, key).is_err() {
        return false;
    }
    conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
        .is_ok_and(|v| (1..=crate::schema::SCHEMA_VERSION).contains(&v))
        && conn
            .query_row("SELECT count(*) FROM sqlite_master", [], |r| {
                r.get::<_, i64>(0)
            })
            .is_ok()
}

// ----- Discard and recreate -----

/// What a confirmed discard removes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardScope {
    /// The unfinished plaintext alone: a complete encrypted copy exists and
    /// opens under the key, so the key and the copy are kept and the next
    /// open promotes the copy. Nothing the person had is lost.
    UnfinishedPlaintext,
    /// Everything the store held, and its key: every contact and its
    /// routes, pending outbound, unread and kept messages, the conversation
    /// list, the duplicate-suppression pairs, the client's settings and the
    /// person's own cards. No identity, no identity key, no trust and no
    /// transport configuration: none of them is in the store.
    Everything,
}

/// A discard as it would run now, built by looking -- the copy opened
/// read-only under the key -- before the person is asked. Confirm it with
/// [`DiscardPlan::confirm`] once they have said yes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscardPlan {
    scope: DiscardScope,
    files: Vec<PathBuf>,
    removes_key: bool,
}

impl DiscardPlan {
    /// What goes.
    #[must_use]
    pub const fn scope(&self) -> DiscardScope {
        self.scope
    }

    /// The files it removes that are there now.
    #[must_use]
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Whether the key file goes with them.
    #[must_use]
    pub const fn removes_key(&self) -> bool {
        self.removes_key
    }

    /// The person's confirmation of this plan: the only way to make a
    /// [`ConfirmedDiscard`]. Call it after they confirmed, never before
    /// asking.
    #[must_use]
    pub fn confirm(self) -> ConfirmedDiscard {
        ConfirmedDiscard(self)
    }
}

/// A [`DiscardPlan`] the person confirmed.
#[derive(Debug)]
pub struct ConfirmedDiscard(DiscardPlan);

/// Look at the profile's store and say what a discard would remove now.
/// Writes nothing. The caller holds the profile's client lock, as for an
/// open.
///
/// # Errors
/// [`StoreError`] when the directory or a name cannot be read.
pub fn plan_discard(
    paths: &ProfilePaths,
    cipher: &dyn KeyCipher,
) -> Result<DiscardPlan, StoreError> {
    let names = Names::of(paths);
    let names = if present(&names.dir)? {
        Names {
            dir: profile_config::resolve_owned_private_dir_within(&names.dir, paths.boundary())
                .map_err(|e| StoreError::from_persist(&names.dir, e))?,
        }
    } else {
        names
    };
    let copy = present(&names.encrypted())?;
    let copy_kept = copy
        && matches!(store_key::open(paths, cipher), Ok(Some(key)) if copy_opens(&names.encrypted(), &key));
    let (scope, mut candidates) = if copy_kept {
        (DiscardScope::UnfinishedPlaintext, Vec::new())
    } else {
        (DiscardScope::Everything, vec![names.encrypted()])
    };
    candidates.extend(names.database_files());
    candidates.push(names.encrypting());
    let mut files = Vec::new();
    for path in candidates {
        if present(&path)? {
            files.push(path);
        }
    }
    files.sort();
    Ok(DiscardPlan {
        scope,
        files,
        removes_key: !copy_kept,
    })
}

/// What a discard did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discarded {
    /// What went.
    pub scope: DiscardScope,
}

/// Run a confirmed discard. The plan is made again first and must be the
/// one the person confirmed -- the same scope and the same files -- or
/// nothing is removed and [`StoreError::DiscardChanged`] says so, for the
/// client to ask again. The next open recreates the store: a new key and
/// an empty store, or the kept copy promoted.
///
/// # Errors
/// [`StoreError::DiscardChanged`]; [`StoreError::Io`] or
/// [`StoreError::KeyFile`] when a removal fails, the files before it gone.
pub fn discard(
    paths: &ProfilePaths,
    cipher: &dyn KeyCipher,
    confirmed: ConfirmedDiscard,
) -> Result<Discarded, StoreError> {
    let ConfirmedDiscard(plan) = confirmed;
    let now = plan_discard(paths, cipher)?;
    if now != plan {
        return Err(StoreError::DiscardChanged);
    }
    for path in &plan.files {
        remove(path)?;
    }
    if let Some(dir) = plan
        .files
        .first()
        .map(|f| private_parent_of(f).to_path_buf())
    {
        sync_dir(&dir)?;
    }
    // THE KEY LAST: a crash before this leaves a key with nothing under it,
    // which the next open replaces; the other order would leave a store
    // with no key, which is recovery mode again.
    if plan.removes_key {
        store_key::remove(paths, cipher).map_err(|e| StoreError::KeyFile(e.to_string()))?;
    }
    Ok(Discarded { scope: plan.scope })
}
