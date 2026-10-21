// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! SPIKE-011's harness (`architecture/roadmap/SPIKES.md`): the shipping
//! human store, opened by its own code, timed and searched under the engine
//! this build carries (see `Cargo.toml` for the three).
//!
//! THE KEY REACHES THE STORE WITHOUT A CHANGE TO IT. SQLCipher reads a key
//! given before the first statement on a connection; the store opens its
//! own and runs its pragmas at once. An auto-extension runs inside every
//! `sqlite3_open`, before the open returns, so the key is in place when the
//! store's first pragma runs -- the store's code is measured as it ships,
//! and the production seam stays plan section 29 step 3's.
//!
//! Commands, each printing one JSON object per line:
//!   versions                      the engine: SQLite, and SQLCipher's
//!                                 version and crypto provider
//!   bench <dir> <kept> <reps>     seed a store of <kept> kept messages
//!                                 and a small unread backlog, then time
//!                                 a cold open (a
//!                                 fresh process each, the files' pages
//!                                 evicted first), commit-unread, and the
//!                                 read walk
//!   open-once <db>                one timed open, for bench's cold rows
//!   deletion <dir> clean|unclean|none  RETENTION.md's property: a released and
//!                                 a kept marker, the release, the close
//!                                 (clean) or an abort and the next open
//!                                 (unclean), or no release (none, the
//!                                 control); then the raw search
//!   abort-after-release <db> <row> the unclean close: release, abort
//!   migrate-seed <dir> <kept>     CIPHER builds: a plaintext store, as a
//!                                 build before the change left it
//!   start <dir>                   CIPHER builds: one start of ADR-0060's
//!                                 sequence (`migrate.rs`), a crash forced
//!                                 after the step `SPIKE011_CRASH_AT` names
//!   rollback-check <dir>          the shipping loader's profile-format
//!                                 check on `<dir>/state`'s marker
//!   pages <db>                    INSPECTOR builds only: every page as the
//!                                 engine decrypts it (`sqlite_dbpage`),
//!                                 searched for both markers
//!
//! `SPIKE011_KEY`, 64 hex digits, is the store key in a cipher build: a
//! raw key, so SQLCipher derives nothing from it (ADR-0060 decision 2's key
//! is 32 random bytes, not a passphrase).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use interweave_human_store::{
    AppMessageId, Cursor, HumanStore, InboundOrigin, NewInbound, Page, PageLimits, RowId,
    StoreError, StoreOptions,
};
use interweave_transport_api::TransportIdentity;

#[cfg(feature = "cipher")]
mod migrate;

const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
/// TEST-ONLY markers, as `crates/human/store/tests/released_content.rs`.
const RELEASED: &[u8] = b"SPIKE011-RELEASED-CONTENT-7b3d9f1a5c2e4b6d8f0a1c3e5b7d";
const KEPT: &[u8] = b"SPIKE011-KEPT-CONTROL-CONTENT-2e4c6a8b0d1f3a5c7e9b0d";
/// The client's own page (`crates/human/app-core/src/listing.rs`).
const PAGE_RECORDS: usize = 64;
const PAGE_BYTES: usize = 1024 * 1024;
/// A message's payload: a chat line of a few sentences.
const PAYLOAD_BYTES: usize = 512;
/// The store is what the person kept: a read message not kept is released,
/// and unread and unsent rows are marginal (the owner, 2026-10-21: 500 kept
/// is the realistic size, 2000 already large). A small unread backlog
/// beside them.
const UNREAD_BACKLOG: u64 = 20;

/// End the process as a crash would, writing no core: a core of this
/// process holds the store key and the test content, and the host's
/// collector keeps it where this account cannot remove it (measured: 15
/// cores left by an earlier run). Non-dumpable, the kernel dumps nothing.
fn crash() -> ! {
    // SAFETY: a plain prctl on this process; nothing else is touched.
    unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) };
    std::process::abort()
}

fn engine() -> &'static str {
    if cfg!(feature = "cipher-vendored") {
        "sqlcipher-vendored-openssl"
    } else if cfg!(feature = "cipher") {
        "sqlcipher-system-libcrypto"
    } else {
        "sqlite-plaintext"
    }
}

#[cfg(feature = "cipher")]
mod key {
    use std::ffi::{c_char, c_int};
    use std::sync::Mutex;

    use rusqlite::auto_extension::{init_auto_extension, register_auto_extension};
    use rusqlite::{Connection, ffi};

    /// The key the next open is given, as 64 hex digits; `None` opens a
    /// connection unkeyed, as a plaintext file needs.
    static KEY: Mutex<Option<String>> = Mutex::new(None);

    fn key_this(conn: Connection) -> rusqlite::Result<()> {
        let key = KEY.lock().map_or(None, |k| k.clone());
        match key {
            Some(hex) => conn.execute_batch(&format!("PRAGMA key = \"x'{hex}'\";")),
            None => Ok(()),
        }
    }

    unsafe extern "C" fn keyed(
        db: *mut ffi::sqlite3,
        err: *mut *mut c_char,
        _: *const ffi::sqlite3_api_routines,
    ) -> c_int {
        // SAFETY: called by SQLite inside an open, with that open's handle;
        // `key_this` neither opens nor closes a database.
        unsafe { init_auto_extension(db, err, key_this) }
    }

    pub fn hex_ok(hex: &str) -> bool {
        hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())
    }

    /// The key for every open from now on, or none.
    pub fn set(hex: Option<String>) {
        if let Ok(mut key) = KEY.lock() {
            *key = hex;
        }
    }

    /// Registers the hook, keyed from `SPIKE011_KEY` when it is set.
    pub fn install() -> Result<(), String> {
        if let Ok(hex) = std::env::var("SPIKE011_KEY") {
            if !hex_ok(&hex) {
                return Err("SPIKE011_KEY is not 64 hex digits".to_owned());
            }
            set(Some(hex));
        }
        // SAFETY: the extension opens no database and never closes the one
        // it is given; registered once, before any open.
        unsafe { register_auto_extension(keyed) }.map_err(|e| format!("auto-extension: {e}"))
    }

    pub fn from_env() -> bool {
        KEY.lock().is_ok_and(|k| k.is_some())
    }
}

#[cfg(not(feature = "cipher"))]
mod key {
    pub fn install() -> Result<(), String> {
        Ok(())
    }
    pub fn from_env() -> bool {
        true
    }
}

fn inbound(id: u64, payload: Vec<u8>) -> NewInbound {
    NewInbound {
        app_message_id: AppMessageId::parse(format!("{id:032x}")).expect("canonical id"),
        origin: InboundOrigin {
            peer: TransportIdentity::parse(PEER).expect("canonical peer"),
            endpoint: None,
            channel: None,
        },
        media_type: None,
        payload,
        received_at: 2_000 + id,
    }
}

fn line(n: u64) -> Vec<u8> {
    let mut text = format!("message {n}: ").into_bytes();
    let words = b"the quick brown fox jumps over the lazy dog, ";
    while text.len() < PAYLOAD_BYTES {
        text.extend_from_slice(words);
    }
    text.truncate(PAYLOAD_BYTES);
    text
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(suffix);
    PathBuf::from(p)
}

fn store_err(what: &str, e: &StoreError) -> String {
    format!("{what}: {e:?}")
}

/// THE OPENER THE PLATFORM'S CLIENT USES. On Android the store is judged
/// up to the app's data directory as the platform reports it, never `/`:
/// `/data` is `system` `0771`, which the desktop opener refuses by design
/// (ADR-0028 A 2026-10-08). `SPIKE011_BOUNDARY` names that directory for
/// a run (device.sh: the run's own directory under `/data/local/tmp`), and
/// the store is opened as the Android client opens it --
/// `ProfilePaths::resolve_embedded` and `HumanStore::open_profile`, so
/// its file sits where the app's does under the boundary. Unset, the
/// desktop opener, as the desktop rows were measured.
fn boundary() -> Option<PathBuf> {
    std::env::var_os("SPIKE011_BOUNDARY").map(PathBuf::from)
}

fn embedded_paths(boundary: &Path) -> Result<interweave_profile_config::ProfilePaths, String> {
    let boundary = interweave_profile_config::TrustBoundary::new(boundary)
        .map_err(|e| format!("boundary {}: {e:?}", boundary.display()))?;
    interweave_profile_config::ProfilePaths::resolve_embedded(EMBEDDED_PROFILE, boundary)
        .map_err(|e| format!("embedded paths: {e:?}"))
}

/// The profile name the embedded runs use; any valid name serves.
const EMBEDDED_PROFILE: &str = "spike-011";

/// Where a run in `dir` keeps its store: `dir/human.sqlite` on the
/// desktop, the embedded layout under the boundary on Android.
fn store_db(dir: &Path) -> Result<PathBuf, String> {
    match boundary() {
        Some(b) => Ok(embedded_paths(&b)?
            .human_dir()
            .join(interweave_human_store::STORE_FILE)),
        None => Ok(dir.join("human.sqlite")),
    }
}

fn open_store(db: &Path) -> Result<HumanStore, String> {
    let Some(b) = boundary() else {
        return HumanStore::open(db, StoreOptions::default()).map_err(|e| store_err("open", &e));
    };
    let paths = embedded_paths(&b)?;
    let expected = paths.human_dir().join(interweave_human_store::STORE_FILE);
    if expected != db {
        return Err(format!(
            "{} is not the embedded store {}",
            db.display(),
            expected.display()
        ));
    }
    HumanStore::open_profile(&paths, StoreOptions::default()).map_err(|e| store_err("open", &e))
}

fn limits() -> PageLimits {
    PageLimits::new(PAGE_RECORDS, PAGE_BYTES).expect("non-zero")
}

fn walk<T>(
    mut page: impl FnMut(Option<Cursor>) -> Result<Page<T>, StoreError>,
) -> Result<usize, StoreError> {
    let (mut rows, mut after) = (0, None);
    loop {
        let Page { items, next } = page(after)?;
        rows += items.len();
        match next {
            Some(cursor) => after = Some(cursor),
            None => return Ok(rows),
        }
    }
}

/// What the client reads to show its conversations at start
/// (`listing.rs`'s `list_all`): every pending, unread and kept row, page by
/// page. The store has no per-conversation read; this is the read.
fn read_walk(store: &HumanStore) -> Result<usize, StoreError> {
    Ok(walk(|after| store.pending_outbound_page(after, limits()))?
        + walk(|after| store.unread_inbound_page(after, limits()))?
        + walk(|after| store.kept_inbound_page(after, limits()))?)
}

/// Drop the files' pages from the page cache: a cold open reads them from
/// the medium. Unprivileged, on the caller's own files; dirty pages are not
/// dropped, so the files are synced first.
fn evict(db: &Path) {
    for path in [
        db.to_path_buf(),
        with_suffix(db, "-wal"),
        with_suffix(db, "-shm"),
    ] {
        if let Ok(file) = std::fs::File::open(&path) {
            let _ = file.sync_all();
            use std::os::fd::AsRawFd as _;
            // SAFETY: a valid descriptor for the call's duration.
            unsafe { libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) };
        }
    }
}

struct Stats {
    op: &'static str,
    micros: Vec<u128>,
}

impl Stats {
    fn print(mut self, extra: &str) {
        self.micros.sort_unstable();
        let n = self.micros.len();
        let at = |q: usize| self.micros[(n - 1) * q / 100];
        println!(
            "{{\"engine\":\"{}\",\"op\":\"{}\",\"n\":{n},\"min_us\":{},\"median_us\":{},\"p90_us\":{},\"max_us\":{}{extra}}}",
            engine(),
            self.op,
            self.micros[0],
            at(50),
            at(90),
            self.micros[n - 1]
        );
    }
}

fn versions() -> Result<(), String> {
    let conn = rusqlite::Connection::open_in_memory().map_err(|e| e.to_string())?;
    let sqlite: String = conn
        .query_row("SELECT sqlite_version()", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let pragma = |name: &str| -> String {
        conn.query_row(&format!("PRAGMA {name}"), [], |r| r.get::<_, String>(0))
            .unwrap_or_else(|_| "-".to_owned())
    };
    println!(
        "{{\"engine\":\"{}\",\"sqlite\":\"{sqlite}\",\"cipher_version\":\"{}\",\"cipher_provider\":\"{}\",\"cipher_provider_version\":\"{}\"}}",
        engine(),
        pragma("cipher_version"),
        pragma("cipher_provider"),
        pragma("cipher_provider_version")
    );
    Ok(())
}

fn seed(db: &Path, kept: u64) -> Result<(), String> {
    let mut store = open_store(db)?;
    for n in 0..kept + UNREAD_BACKLOG {
        let row = store
            .commit_unread_inbound(&inbound(n, line(n)))
            .map_err(|e| store_err("seed commit", &e))?;
        if n < kept {
            let read = store
                .mark_read(row, 3_000 + n)
                .map_err(|e| store_err("seed read", &e))?;
            store
                .keep(&read, 3_001 + n)
                .map_err(|e| store_err("seed keep", &e))?;
        }
    }
    Ok(())
}

fn bench(dir: &Path, kept: u64, reps: usize) -> Result<(), String> {
    let db = store_db(dir)?;
    if db.exists() {
        return Err(format!(
            "{} exists: bench seeds its own store",
            db.display()
        ));
    }
    seed(&db, kept)?;
    let bytes = std::fs::metadata(&db).map_or(0, |m| m.len());
    let extra = format!(",\"kept\":{kept},\"unread\":{UNREAD_BACKLOG},\"db_bytes\":{bytes}");

    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut open = Stats {
        op: "open-cold",
        micros: Vec::new(),
    };
    for _ in 0..reps {
        evict(&db);
        let out = Command::new(&me)
            .arg("open-once")
            .arg(&db)
            .output()
            .map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout);
        let micros = text.trim().parse::<u128>().map_err(|_| {
            format!(
                "open-once said {text:?} ({})",
                String::from_utf8_lossy(&out.stderr)
            )
        })?;
        open.micros.push(micros);
    }
    open.print(&extra);

    let mut store = open_store(&db)?;
    let mut commit = Stats {
        op: "commit-unread",
        micros: Vec::new(),
    };
    for k in 0..reps as u64 {
        let n = kept + UNREAD_BACKLOG + k;
        let new = inbound(n, line(n));
        let start = Instant::now();
        store
            .commit_unread_inbound(&new)
            .map_err(|e| store_err("commit", &e))?;
        commit.micros.push(start.elapsed().as_micros());
    }
    commit.print(&extra);

    let mut read = Stats {
        op: "read-walk",
        micros: Vec::new(),
    };
    let mut rows = 0;
    for _ in 0..reps {
        let start = Instant::now();
        rows = read_walk(&store).map_err(|e| store_err("read", &e))?;
        read.micros.push(start.elapsed().as_micros());
    }
    read.print(&format!("{extra},\"rows\":{rows}"));
    Ok(())
}

fn open_once(db: &Path) -> Result<(), String> {
    let start = Instant::now();
    let store = open_store(db)?;
    let micros = start.elapsed().as_micros();
    drop(store);
    println!("{micros}");
    Ok(())
}

/// Whether `needle` is in the file (a missing file holds nothing).
fn holds(path: &Path, needle: &[u8]) -> bool {
    std::fs::read(path).is_ok_and(|bytes| bytes.windows(needle.len()).any(|w| w == needle))
}

fn search(db: &Path, case: &str) {
    for (file, path) in [("db", db.to_path_buf()), ("wal", with_suffix(db, "-wal"))] {
        let size = std::fs::metadata(&path).map_or(-1, |m| m.len() as i64);
        println!(
            "{{\"engine\":\"{}\",\"case\":\"{case}\",\"search\":\"raw\",\"file\":\"{file}\",\"bytes\":{size},\"released\":{},\"kept\":{}}}",
            engine(),
            holds(&path, RELEASED),
            holds(&path, KEPT)
        );
    }
}

fn deletion(dir: &Path, case: &str) -> Result<(), String> {
    let db = store_db(dir)?;
    if db.exists() {
        return Err(format!(
            "{} exists: deletion seeds its own store",
            db.display()
        ));
    }
    let mut store = open_store(&db)?;
    let kept = store
        .commit_unread_inbound(&inbound(2, KEPT.to_vec()))
        .map_err(|e| store_err("kept", &e))?;
    let read = store
        .mark_read(kept, 3_000)
        .map_err(|e| store_err("read", &e))?;
    store
        .keep(&read, 3_001)
        .map_err(|e| store_err("keep", &e))?;
    let released = store
        .commit_unread_inbound(&inbound(1, RELEASED.to_vec()))
        .map_err(|e| store_err("released", &e))?;
    drop(store);
    search(&db, "setup");

    match case {
        "clean" => {
            let mut store = open_store(&db).map_err(|e| format!("reopen: {e}"))?;
            store
                .mark_read(released, 4_000)
                .map_err(|e| store_err("release", &e))?;
            drop(store);
        }
        "unclean" => {
            let me = std::env::current_exe().map_err(|e| e.to_string())?;
            let status = Command::new(&me)
                .arg("abort-after-release")
                .arg(&db)
                .arg(released.get().to_string())
                .status()
                .map_err(|e| e.to_string())?;
            if status.success() {
                return Err("the unclean close did not abort".to_owned());
            }
            search(&db, "after-abort");
            let store = open_store(&db).map_err(|e| format!("the next open: {e}"))?;
            drop(store);
        }
        // The control for `pages`: the released marker, unreleased, is
        // what a decrypted search finds.
        "none" => {}
        other => return Err(format!("no case {other:?}: clean, unclean or none")),
    }
    search(&db, case);
    Ok(())
}

fn abort_after_release(db: &Path, row: &str) -> Result<(), String> {
    let row: i64 = row.parse().map_err(|_| format!("row {row:?}"))?;
    let mut store = open_store(db)?;
    store
        .mark_read(RowId::from_stored(row), 4_000)
        .map_err(|e| store_err("release", &e))?;
    // No drop: the process ends with the store open, its log as the
    // release left it.
    crash()
}

/// Every page as the engine reads it, decrypted under the key in a cipher
/// build, searched. Needs `SQLITE_ENABLE_DBPAGE_VTAB`, which the measured
/// builds do not carry: run from a separate INSPECTOR build
/// (`LIBSQLITE3_FLAGS=SQLITE_ENABLE_DBPAGE_VTAB`), on a copy, read-only.
fn pages(db: &Path) -> Result<(), String> {
    let conn = rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT data FROM sqlite_dbpage ORDER BY pgno")
        .map_err(|e| format!("sqlite_dbpage (an inspector build?): {e}"))?;
    let mut all = Vec::new();
    let mut count = 0;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let page: Vec<u8> = row.get(0).map_err(|e| e.to_string())?;
        all.extend_from_slice(&page);
        count += 1;
    }
    let found = |needle: &[u8]| all.windows(needle.len()).any(|w| w == needle);
    println!(
        "{{\"engine\":\"{}\",\"search\":\"pages\",\"pages\":{count},\"released\":{},\"kept\":{}}}",
        engine(),
        found(RELEASED),
        found(KEPT)
    );
    Ok(())
}

/// What a shipping build (any engine; the marker check is not the
/// engine's) says of the profile at `<dir>/state`: its own loader's
/// marker check, `check_within` against its own `PROFILE_FORMAT`.
fn rollback_check(dir: &Path) -> Result<(), String> {
    use interweave_profile_config::profile_format::{
        PROFILE_FORMAT, PROFILE_FORMAT_FILE, check_within,
    };
    let path = dir.join("state").join(PROFILE_FORMAT_FILE);
    let answer = match check_within(
        &path,
        &interweave_profile_config::TrustBoundary::root(),
        PROFILE_FORMAT,
    ) {
        Ok(n) => format!("accepted: format {n}"),
        Err(e) => format!("refused: {e}"),
    };
    println!(
        "{{\"engine\":\"{}\",\"own_format\":{PROFILE_FORMAT},\"rollback\":{answer:?}}}",
        engine()
    );
    Ok(())
}

fn run(args: &[String]) -> Result<(), String> {
    key::install()?;
    // A cipher build measured without its key would measure plaintext.
    let keyed_commands = [
        "bench",
        "open-once",
        "deletion",
        "abort-after-release",
        "pages",
    ];
    if args
        .first()
        .is_some_and(|c| keyed_commands.contains(&c.as_str()))
        && !key::from_env()
    {
        return Err("SPIKE011_KEY is not set".to_owned());
    }
    let arg = |i: usize| args.get(i).ok_or_else(|| format!("missing argument {i}"));
    match args.first().map(String::as_str) {
        Some("versions") => versions(),
        Some("bench") => {
            let kept = arg(2)?.parse().map_err(|_| "kept".to_owned())?;
            let reps = arg(3)?.parse().map_err(|_| "reps".to_owned())?;
            bench(Path::new(arg(1)?), kept, reps)
        }
        Some("open-once") => open_once(Path::new(arg(1)?)),
        Some("deletion") => deletion(Path::new(arg(1)?), arg(2)?),
        Some("abort-after-release") => abort_after_release(Path::new(arg(1)?), arg(2)?),
        Some("pages") => pages(Path::new(arg(1)?)),
        Some("rollback-check") => rollback_check(Path::new(arg(1)?)),
        #[cfg(feature = "cipher")]
        Some("migrate-seed") => {
            let kept = arg(2)?.parse().map_err(|_| "kept".to_owned())?;
            migrate::seed_plaintext(Path::new(arg(1)?), kept)
        }
        #[cfg(feature = "cipher")]
        Some("start") => migrate::start_report(Path::new(arg(1)?)),
        _ => Err(
            "usage: versions | bench <dir> <kept> <reps> | open-once <db> | \
                  deletion <dir> clean|unclean|none | abort-after-release <db> <row> | pages <db>"
                .to_owned(),
        ),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("spike-011: {e}");
            ExitCode::FAILURE
        }
    }
}
