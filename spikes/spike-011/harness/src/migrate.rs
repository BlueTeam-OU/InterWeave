// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! ADR-0060 decisions 3 and 6, prototyped to be measured: the desktop key
//! file and the plaintext-to-encrypted rewrite, its crash states told apart
//! by file names. A spike's prototype, not the production code (plan
//! section 29 step 3 writes that): what it settles is whether the sequence
//! the record describes survives a crash at each step and leaves nothing
//! in clear.
//!
//! Layout, as decision 3 places it: `<state>/human-store.key`, outside
//! `<state>/human/`, which holds `human.sqlite`.
//!
//! The key file: `IWSK`, a version byte, the 32-byte key -- exactly 37
//! bytes. A version this build does not know, a malformed file, and a
//! missing one beside an encrypted store are each refused by name, and the
//! store's files are left as they were: a new key would lose the store.

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::time::Instant;

use interweave_human_store::{HumanStore, StoreOptions};

const KEY_FILE: &str = "human-store.key";
/// The encrypted build's profile format: one above the shipping build's
/// (`interweave_profile_config::profile_format::PROFILE_FORMAT`), raised
/// as step (a)'s first act (ADR-0060 A 2026-10-21), so every build since
/// the marker refuses the profile by it before it reads the store.
pub const ENCRYPTED_FORMAT: u32 = interweave_profile_config::profile_format::PROFILE_FORMAT + 1;
const MAGIC: &[u8; 4] = b"IWSK";
const KEY_VERSION: u8 = 1;
const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";
/// Every seeded payload holds it; in clear, it is content left on disk.
pub const CONTENT: &[u8] = b"the quick brown fox jumps";

// The fields are read through `Debug`, in the report.
#[allow(dead_code)]
#[derive(Debug)]
pub enum StartError {
    /// An encrypted (or unreadable) store with no key file beside it.
    KeyMissing,
    /// A key file of a version this build does not read.
    KeyVersion(u8),
    /// A key file that is not one: wrong length or magic.
    KeyMalformed,
    /// Both a plaintext store and something that is not one, with no
    /// file name to say which is authoritative.
    Ambiguous(&'static str),
    Io(String),
    Store(String),
}

fn io(what: &str, e: &std::io::Error) -> StartError {
    StartError::Io(format!("{what}: {e}"))
}

pub struct Layout {
    pub state: PathBuf,
    pub human: PathBuf,
}

impl Layout {
    pub fn at(dir: &Path) -> Self {
        let state = dir.join("state");
        Self { human: state.join("human"), state }
    }
    pub fn key(&self) -> PathBuf {
        self.state.join(KEY_FILE)
    }
    pub fn marker(&self) -> PathBuf {
        self.state
            .join(interweave_profile_config::profile_format::PROFILE_FORMAT_FILE)
    }
    pub fn db(&self) -> PathBuf {
        self.human.join("human.sqlite")
    }
    fn named(&self, suffix: &str) -> PathBuf {
        self.human.join(format!("human.sqlite{suffix}"))
    }
    pub fn create(&self) -> Result<(), StartError> {
        use std::os::unix::fs::DirBuilderExt as _;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.human)
            .map_err(|e| io("layout", &e))
    }
}

/// The step after which a crash is forced, from `SPIKE011_CRASH_AT`.
fn crash_point(step: &str) {
    if std::env::var("SPIKE011_CRASH_AT").is_ok_and(|at| at == step) {
        crate::crash();
    }
}

fn sync_file(path: &Path) -> Result<(), StartError> {
    std::fs::File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|e| io("sync", &e))
}

fn sync_dir(dir: &Path) -> Result<(), StartError> {
    sync_file(dir)
}

fn is_plaintext(path: &Path) -> Result<bool, StartError> {
    let mut head = [0_u8; 16];
    let mut file = std::fs::File::open(path).map_err(|e| io("read header", &e))?;
    match file.read_exact(&mut head) {
        Ok(()) => Ok(&head == SQLITE_HEADER),
        // Shorter than a header: not a plaintext database.
        Err(_) => Ok(false),
    }
}

fn hex(key: &[u8; 32]) -> String {
    key.iter().map(|b| format!("{b:02x}")).collect()
}

fn read_key(path: &Path) -> Result<[u8; 32], StartError> {
    let bytes = std::fs::read(path).map_err(|e| io("key file", &e))?;
    if bytes.len() < 5 || &bytes[..4] != MAGIC {
        return Err(StartError::KeyMalformed);
    }
    if bytes[4] != KEY_VERSION {
        return Err(StartError::KeyVersion(bytes[4]));
    }
    if bytes.len() != 37 {
        return Err(StartError::KeyMalformed);
    }
    let mut key = [0_u8; 32];
    key.copy_from_slice(&bytes[5..]);
    Ok(key)
}

/// A new key from the system's CSPRNG, written and synced before anything
/// is encrypted under it (decision 6 step (a)).
fn new_key(layout: &Layout) -> Result<[u8; 32], StartError> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut key = [0_u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut key))
        .map_err(|e| io("csprng", &e))?;
    let tmp = layout.state.join(format!("{KEY_FILE}.tmp"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .map_err(|e| io("key file", &e))?;
    file.write_all(MAGIC)
        .and_then(|()| file.write_all(&[KEY_VERSION]))
        .and_then(|()| file.write_all(&key))
        .and_then(|()| file.sync_all())
        .map_err(|e| io("key file", &e))?;
    std::fs::rename(&tmp, layout.key()).map_err(|e| io("key file", &e))?;
    sync_dir(&layout.state)?;
    Ok(key)
}

fn sql(e: &rusqlite::Error) -> StartError {
    StartError::Store(e.to_string())
}

/// Step (a), the copy: the plaintext store exported under the key to
/// `.encrypting`, synced, renamed `.encrypted`, the directory synced.
fn export(layout: &Layout, key: &[u8; 32]) -> Result<(), StartError> {
    let encrypting = layout.named(".encrypting");
    // CREATED OWNER-ONLY FIRST, as the store creates its own file: ATTACH
    // creates a missing database with the process umask (0644 measured), a
    // copy every local account could read and the store then refuses to
    // open (PermissionsTooOpen). An empty 0600 file is opened, not made.
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&encrypting)
            .map_err(|e| io("create the copy", &e))?;
    }
    crate::key::set(None);
    let plain = rusqlite::Connection::open(layout.db()).map_err(|e| sql(&e))?;
    // `sqlcipher_export` copies the schema and the rows, NOT the header's
    // user_version, which is the store's schema version: carried by hand,
    // or the encrypted store reads as version 0 and migrates from scratch.
    let version: i64 = plain
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| sql(&e))?;
    plain
        .execute_batch(&format!(
            "ATTACH DATABASE '{}' AS enc KEY \"x'{}'\";
             SELECT sqlcipher_export('enc');
             PRAGMA enc.user_version = {version};
             DETACH DATABASE enc;",
            encrypting.display(),
            hex(key)
        ))
        .map_err(|e| sql(&e))?;
    drop(plain);
    sync_file(&encrypting)?;
    crash_point("exported");
    std::fs::rename(&encrypting, layout.named(".encrypted")).map_err(|e| io("rename", &e))?;
    sync_dir(&layout.human)?;
    crash_point("renamed-encrypted");
    Ok(())
}

/// Step (b): the plaintext scrubbed in place -- its rows deleted under
/// secure_delete, vacuumed, its log checkpointed and truncated -- then its
/// companions unlinked, then the file. Idempotent: a crash part-way is
/// resumed from the start.
fn scrub_plaintext(layout: &Layout) -> Result<(), StartError> {
    let db = layout.db();
    if db.exists() {
        crate::key::set(None);
        let plain = rusqlite::Connection::open(&db).map_err(|e| sql(&e))?;
        plain
            .execute_batch(
                "PRAGMA secure_delete = ON;
                 DELETE FROM unread_inbound;
                 DELETE FROM kept_inbound;
                 DELETE FROM pending_outbound;
                 VACUUM;
                 PRAGMA wal_checkpoint(TRUNCATE);",
            )
            .map_err(|e| sql(&e))?;
        drop(plain);
        sync_file(&db)?;
    }
    crash_point("scrubbed");
    for suffix in ["-wal", "-shm", ""] {
        let path = layout.named(suffix);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io("unlink", &e)),
        }
    }
    sync_dir(&layout.human)?;
    crash_point("unlinked");
    Ok(())
}

/// Step (c).
fn take_encrypted(layout: &Layout) -> Result<(), StartError> {
    std::fs::rename(layout.named(".encrypted"), layout.db()).map_err(|e| io("rename", &e))?;
    sync_dir(&layout.human)?;
    crash_point("renamed-final");
    Ok(())
}

/// What a start finds, by file names, and does.
pub fn start(layout: &Layout) -> Result<(HumanStore, &'static str), StartError> {
    let db = layout.db();
    let encrypting = layout.named(".encrypting");
    let encrypted = layout.named(".encrypted");
    let mut path_taken = "opened";

    // An export that did not finish: the plaintext is still the store.
    if encrypting.exists() {
        std::fs::remove_file(&encrypting).map_err(|e| io("unlink", &e))?;
        sync_dir(&layout.human)?;
        path_taken = "export restarted";
    }
    if encrypted.exists() {
        // The copy is complete (it is renamed only once synced): it is the
        // store, and any plaintext beside it is what (b) had not removed.
        if db.exists() && !is_plaintext(&db)? {
            return Err(StartError::Ambiguous(
                "human.sqlite.encrypted beside a human.sqlite that is not plaintext",
            ));
        }
        scrub_plaintext(layout)?;
        take_encrypted(layout)?;
        path_taken = "rewrite resumed";
    } else if db.exists() && is_plaintext(&db)? {
        // Step (a)'s first act: the marker, so a build that reads only
        // plaintext refuses the profile by it from here on, before
        // anything is encrypted.
        interweave_profile_config::profile_format::raise_within(
            &layout.marker(),
            &interweave_profile_config::TrustBoundary::root(),
            ENCRYPTED_FORMAT,
        )
        .map_err(|e| StartError::Io(format!("marker: {e}")))?;
        crash_point("marker-raised");
        // A crash between the key file and the export leaves the key: it
        // encrypts nothing yet, and is reused.
        let key = if layout.key().exists() {
            read_key(&layout.key())?
        } else {
            new_key(layout)?
        };
        crash_point("key-written");
        export(layout, &key)?;
        scrub_plaintext(layout)?;
        take_encrypted(layout)?;
        if path_taken == "opened" {
            path_taken = "rewritten";
        }
    }

    let key = if db.exists() {
        if !layout.key().exists() {
            return Err(StartError::KeyMissing);
        }
        read_key(&layout.key())?
    } else if layout.key().exists() {
        read_key(&layout.key())?
    } else {
        path_taken = "created";
        interweave_profile_config::profile_format::raise_within(
            &layout.marker(),
            &interweave_profile_config::TrustBoundary::root(),
            ENCRYPTED_FORMAT,
        )
        .map_err(|e| StartError::Io(format!("marker: {e}")))?;
        new_key(layout)?
    };
    crate::key::set(Some(hex(&key)));
    let store = HumanStore::open(&db, StoreOptions::default())
        .map_err(|e| StartError::Store(format!("{e:?}")))?;
    Ok((store, path_taken))
}

/// Seed a PLAINTEXT store, as a build before the change left it.
pub fn seed_plaintext(dir: &Path, kept: u64) -> Result<(), String> {
    let layout = Layout::at(dir);
    layout.create().map_err(|e| format!("{e:?}"))?;
    crate::key::set(None);
    crate::seed(&layout.db(), kept)?;
    if !is_plaintext(&layout.db()).map_err(|e| format!("{e:?}"))? {
        return Err("the seeded store is not plaintext".to_owned());
    }
    Ok(())
}

fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|it| {
            it.filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn holds(path: &Path, needle: &[u8]) -> bool {
    std::fs::read(path).is_ok_and(|b| b.windows(needle.len()).any(|w| w == needle))
}

/// One start, reported: the path it took, the rows the store holds, the
/// files left, and whether any file of the human directory holds content
/// in clear.
pub fn start_report(dir: &Path) -> Result<(), String> {
    let layout = Layout::at(dir);
    let begin = Instant::now();
    let outcome = start(&layout);
    let micros = begin.elapsed().as_micros();
    let (result, rows) = match &outcome {
        Ok((store, path)) => (
            (*path).to_owned(),
            crate::read_walk(store).map_err(|e| format!("{e:?}"))?,
        ),
        Err(e) => (format!("refused: {e:?}"), 0),
    };
    drop(outcome);
    let human = files(&layout.human);
    let clear: Vec<&String> = human
        .iter()
        .filter(|name| holds(&layout.human.join(name), CONTENT))
        .collect();
    println!(
        "{{\"engine\":\"{}\",\"start\":\"{result}\",\"rows\":{rows},\"micros\":{micros},\"human_files\":{human:?},\"state_files\":{:?},\"content_in_clear\":{clear:?}}}",
        crate::engine(),
        files(&layout.state)
    );
    Ok(())
}
