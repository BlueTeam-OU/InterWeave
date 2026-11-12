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
//! 1500 fresh processes of eight threads each, the store's own
//! `open_profile` failed in 5 without this start and in none with it
//! (`tests/first_open_race.rs`, the second figure; the first needs the
//! start taken out). A one-off probe of bare rusqlite opens, kept in no
//! test, failed in 7 and in none with one open made first (PR #641).
//!
//! The first open here runs on one thread behind a [`Once`], which
//! returns only after the whole of `sqlite3_initialize`, the `SQLCipher`
//! half included; every other open waits on it. Nothing in the store
//! opens a connection any other way, and every open here goes through
//! `started` (`no_connection_is_opened_past_the_engine`,
//! `every_open_here_waits_for_the_start`).

use std::path::Path;
use std::sync::Once;

use rusqlite::{Connection, OpenFlags};

static STARTED: Once = Once::new();

/// `open`, run once the engine has started, `SQLCipher` included.
fn started<T>(open: impl FnOnce() -> T) -> T {
    // A failure of this first open is the engine's, and every later open
    // reports its own; what it is for is the start it ran to the end.
    STARTED.call_once(|| drop(Connection::open_in_memory()));
    open()
}

/// [`Connection::open`], after the engine has started.
pub(crate) fn open(path: impl AsRef<Path>) -> rusqlite::Result<Connection> {
    started(|| Connection::open(path))
}

/// [`Connection::open_with_flags`], after the engine has started.
pub(crate) fn open_with_flags(
    path: impl AsRef<Path>,
    flags: OpenFlags,
) -> rusqlite::Result<Connection> {
    started(|| Connection::open_with_flags(path, flags))
}

/// [`Connection::open_in_memory`], after the engine has started.
pub(crate) fn open_in_memory() -> rusqlite::Result<Connection> {
    started(Connection::open_in_memory)
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

    /// The other half: in this file, every `Connection::open*` outside
    /// the start's own is what a `started(...)` runs, so no wrapper
    /// opens before the engine has started, and the start is that open.
    /// A wrapper written as a bare `Connection::open*`, or the start's
    /// open taken out of its `Once`, fails this.
    #[test]
    fn every_open_here_waits_for_the_start() {
        let text = include_str!("engine.rs");
        let code = &text[..text.find("#[cfg(test)]").expect("the tests module")];
        let bare: Vec<(usize, &str)> = code
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim_start().starts_with("//"))
            .filter(|(_, line)| line.contains("Connection::open"))
            .filter(|(_, line)| !line.contains("started(") && !line.contains("call_once("))
            .map(|(n, line)| (n + 1, line.trim()))
            .collect();
        assert!(bare.is_empty(), "opened before the start: {bare:#?}");
        let wrappers = code
            .lines()
            .filter(|line| line.contains("started(") && line.contains("Connection::open"))
            .count();
        assert_eq!(wrappers, 3, "open, open_with_flags and open_in_memory");
        // The start itself: one open, the whole of the Once's body. Taken
        // out or emptied, the wrappers above would wait on nothing.
        let start = code
            .lines()
            .filter(|line| {
                line.trim() == "STARTED.call_once(|| drop(Connection::open_in_memory()));"
            })
            .count();
        assert_eq!(start, 1, "the Once opens the engine once");
    }
}
