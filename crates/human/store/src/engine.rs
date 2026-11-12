// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Every connection the store opens: after the engine has finished
//! starting, once per process (j263).
//!
//! `SQLCipher`'s `sqlite3_initialize` marks SQLite started and lets go of
//! its init mutex before it runs `SQLITE_EXTRA_INIT`, `SQLCipher`'s own
//! start-up. A second thread whose first SQLite call lands in that gap
//! is told SQLite is started and keys its connection against a codec
//! that is not: `PRAGMA key` fails with `SQLCipher`'s generic "An error
//! occurred with PRAGMA key or rekey" (`SQLITE_ERROR`, extended 1). Once
//! per process, so a loop in one process never sees it. Measured over
//! 1500 fresh processes of eight threads each: bare rusqlite opens
//! failed in 7, and in none with one open made first; the store's own
//! `open_profile` without this start failed in 5, and with it in none
//! (`tests/first_open_race.rs`).
//!
//! The first open here runs on one thread behind a [`Once`], which
//! returns only after the whole of `sqlite3_initialize`, the `SQLCipher`
//! half included; every other open waits on it. Nothing in the store
//! opens a connection any other way (`no_connection_is_opened_past_the_engine`).

use std::path::Path;
use std::sync::Once;

use rusqlite::{Connection, OpenFlags};

static STARTED: Once = Once::new();

/// The engine started, `SQLCipher` included, before this returns.
fn start() {
    STARTED.call_once(|| {
        // A failure here is the engine's and every later open reports
        // its own; what this call is for is the start it ran to the end.
        drop(Connection::open_in_memory());
    });
}

/// [`Connection::open`], after the engine has started.
pub(crate) fn open(path: impl AsRef<Path>) -> rusqlite::Result<Connection> {
    start();
    Connection::open(path)
}

/// [`Connection::open_with_flags`], after the engine has started.
pub(crate) fn open_with_flags(
    path: impl AsRef<Path>,
    flags: OpenFlags,
) -> rusqlite::Result<Connection> {
    start();
    Connection::open_with_flags(path, flags)
}

/// [`Connection::open_in_memory`], after the engine has started.
pub(crate) fn open_in_memory() -> rusqlite::Result<Connection> {
    start();
    Connection::open_in_memory()
}

#[cfg(test)]
mod tests {
    /// The rule the module doc states: outside this file no source of the
    /// store opens a connection itself. A new `Connection::open*` in
    /// `src/` fails this, whatever thread it would run on.
    #[test]
    fn no_connection_is_opened_past_the_engine() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = Vec::new();
        let mut dirs = vec![src];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).expect("read src") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    dirs.push(path);
                    continue;
                }
                if path.extension().is_none_or(|e| e != "rs") || path.ends_with("engine.rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).expect("read");
                for (n, line) in text.lines().enumerate() {
                    if line.contains("Connection::open") {
                        found.push(format!("{}:{}", path.display(), n + 1));
                    }
                }
            }
        }
        assert!(found.is_empty(), "opened past crate::engine: {found:#?}");
    }
}
