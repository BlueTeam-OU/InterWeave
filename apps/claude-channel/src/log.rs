// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The bridge's own log: every line it writes goes to stderr, as the host
//! captures it, and to a bounded file under the profile's state
//! directory, `<state>/claude-channel/<endpoint>.log`, beside the human
//! client's `human/` (`plugin/TOOL-SURFACE.md` §Status visibility, `log`).
//!
//! **A line is a [`Line`], never a string from the caller.** The set is
//! closed, and no variant holds anything a message brought or the bridge
//! minted for one -- no content, no reply token, no message id, no
//! `PeerId`, no endpoint or channel a peer named (observability.md
//! §Principles; TM-32). The two that carry text carry the start's own
//! account of its arguments and environment, and a local I/O error: the
//! wire reaches neither (`tests/threat_rows.rs`,
//! `tm_32_nothing_a_message_brought_reaches_the_bridges_stderr_or_its_log`).
//!
//! **Bounded.** A line that would take the file past [`MAX_LOG_FILE_BYTES`]
//! first moves it to `<endpoint>.log.1`, replacing the one there, so the
//! two hold at most twice the bound; a line is cut at [`MAX_LINE_BYTES`]
//! (`a_log_written_past_its_bound_rotates_and_never_exceeds_it`). The
//! bound is one process's: two bridges on one profile and endpoint --
//! the second refused its lease -- each check the size before their own
//! append, so the file can pass it by a line per writer.
//!
//! **A log that cannot be written is not a refusal.** The bridge serves
//! without its file and `status.log.file_error` says why; stderr stays.

use std::fmt;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use interweave_claude_channel_core::{Delivery, rfc3339_utc};
use interweave_profile_config::PersistError;
use interweave_transport_api::{EndpointId, TransportError};
use serde_json::{Value, json};

/// The largest the log file grows before it is rotated.
pub const MAX_LOG_FILE_BYTES: u64 = 1024 * 1024;

/// The longest line written, its newline included; a longer one is cut at
/// a character boundary.
pub const MAX_LINE_BYTES: usize = 1024;

/// The directory under the profile's state directory.
pub const LOG_DIR: &str = "claude-channel";

/// How much is written: a line at its level or a more severe one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// The bridge cannot do what it was started for.
    Error,
    /// Something was refused or dropped.
    Warn,
    /// The session's life: started, opened, lost, ended. The default.
    #[default]
    Info,
    /// Every reconnect attempt and its delay.
    Debug,
}

impl Level {
    /// `error`, `warn`, `info` or `debug`; anything else is `None`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "error" => Some(Self::Error),
            "warn" => Some(Self::Warn),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }

    /// The name [`Level::parse`] reads.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}

/// Every line the bridge writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// The bridge began serving a session that can work.
    Started {
        /// The mode it serves.
        delivery: Delivery,
    },
    /// A start that cannot work, served rather than exited: the start's
    /// account of its arguments or environment (`refused::Refusal`).
    Refused(String),
    /// The process could not go on: a local runtime or stdio error.
    Failed(String),
    /// The host closed stdin, or a signal came: the bridge ends.
    Ended(&'static str),
    /// A host line over the ceiling, dropped unread.
    HostLineTooLong(usize),
    /// A session opened; whether it holds the endpoint's lease.
    Opened {
        /// The lease is held.
        leased: bool,
    },
    /// A session could not be opened, by its error's class.
    OpenFailed(TransportError),
    /// The session ended; a reconnect is scheduled.
    SessionLost,
    /// The daemon revoked the lease in session.
    LeaseRevoked,
    /// The next open is due after this long.
    Reconnect {
        /// The delay, in milliseconds.
        delay_ms: u64,
    },
    /// A re-join at a reconnect the daemon refused, by its error's class;
    /// the channel is `status.rejoin_refused`'s, never the log's.
    RejoinRefused(TransportError),
    /// An inbound message dropped, as its fixed text (`serve::dropped`).
    Dropped(&'static str),
    /// A notification that did not serialize, by the category.
    Unserializable(serde_json::error::Category),
    /// The pull queue refused an event taken past its room.
    PullQueueRefused,
}

impl Line {
    /// The level the line is written at.
    #[must_use]
    pub const fn level(&self) -> Level {
        match self {
            Self::Refused(_) | Self::Failed(_) | Self::PullQueueRefused => Level::Error,
            Self::HostLineTooLong(_)
            | Self::OpenFailed(_)
            | Self::RejoinRefused(_)
            | Self::Dropped(_)
            | Self::Unserializable(_) => Level::Warn,
            Self::Started { .. }
            | Self::Ended(_)
            | Self::Opened { .. }
            | Self::SessionLost
            | Self::LeaseRevoked => Level::Info,
            Self::Reconnect { .. } => Level::Debug,
        }
    }
}

impl fmt::Display for Line {
    /// The text after `claude-channel: `, as stderr has always carried it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Started { delivery } => write!(
                f,
                "started, version {}, delivery {}",
                env!("CARGO_PKG_VERSION"),
                match delivery {
                    Delivery::Push => "push",
                    Delivery::Pull => "pull",
                }
            ),
            Self::Refused(reason) => write!(f, "{reason}; serving the refusal"),
            Self::Failed(what) => f.write_str(what),
            Self::Ended(why) => write!(f, "ended: {why}"),
            Self::HostLineTooLong(max) => write!(f, "a host line over {max} bytes was dropped"),
            Self::Opened { leased: true } => f.write_str("a session opened, the lease held"),
            Self::Opened { leased: false } => f.write_str("a session opened without a lease"),
            Self::OpenFailed(error) => write!(f, "a session did not open: {error:?}"),
            Self::SessionLost => f.write_str("the session ended; reconnecting"),
            Self::LeaseRevoked => f.write_str("the daemon revoked the endpoint's lease"),
            Self::Reconnect { delay_ms } => write!(f, "the next open in {delay_ms} ms"),
            Self::RejoinRefused(error) => write!(f, "a re-join was refused: {error:?}"),
            Self::Dropped(text) => f.write_str(text),
            Self::Unserializable(category) => {
                write!(f, "a notification did not serialize: {category:?}")
            }
            Self::PullQueueRefused => {
                f.write_str("the pull queue refused an event taken past its room")
            }
        }
    }
}

/// The log: its level, and its file while it can be written. A clone
/// writes the same file.
#[derive(Debug, Clone)]
pub struct Log {
    level: Level,
    file: Option<PathBuf>,
    /// Why there is no file, or that a write to it has failed since the
    /// start; it stays set once one has.
    file_error: Option<&'static str>,
}

impl Log {
    /// stderr alone, `why` being the class `status.log.file_error` says.
    #[must_use]
    pub const fn stderr_only(level: Level, why: &'static str) -> Self {
        Self {
            level,
            file: None,
            file_error: Some(why),
        }
    }

    /// The log for `endpoint` of the profile whose state directory is
    /// `state_dir`: the directory made owner-only, or judged so when it
    /// is there. A directory that cannot be had leaves stderr alone.
    #[must_use]
    pub fn open(level: Level, state_dir: &Path, endpoint: &EndpointId) -> Self {
        let dir = state_dir.join(LOG_DIR);
        let made = interweave_profile_config::create_private_dir(&dir)
            .and_then(|()| interweave_profile_config::require_owned_private_dir(&dir));
        match made {
            Ok(()) => Self {
                level,
                file: Some(dir.join(format!("{}.log", endpoint.as_str()))),
                file_error: None,
            },
            Err(e) => Self::stderr_only(level, persist_class(&e)),
        }
    }

    /// The level lines are written at.
    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    /// The file's path, while there is one.
    #[must_use]
    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// `status.log`: the level, the file's path or `null`, and the class
    /// of why it is not written when it is not.
    #[must_use]
    pub fn status(&self) -> Value {
        let mut status = json!({
            "level": self.level.as_str(),
            "file": self.file.as_ref().map(|p| p.to_string_lossy().into_owned()),
        });
        if let Some(why) = self.file_error {
            status["file_error"] = Value::from(why);
        }
        status
    }

    /// Write `line` to stderr and the file, if its level is in.
    pub fn write(&mut self, line: &Line) {
        if line.level() > self.level {
            return;
        }
        let text = cut(format!("claude-channel: {line}"), MAX_LINE_BYTES - 1);
        // The file first: it is the copy that outlives a host gone.
        if let Some(path) = &self.file {
            let stamped = format!(
                "{} {:<5} {text}",
                rfc3339_utc(wall_ms()).unwrap_or_default(),
                line.level().as_str()
            );
            let stamped = cut(stamped, MAX_LINE_BYTES - 1) + "\n";
            if append(path, stamped.as_bytes()).is_err() {
                self.file_error = Some("write");
            }
        }
        // NOT `eprintln!`, which panics on a stderr the host closed
        // (`a_closed_stderr_neither_ends_the_bridge_nor_costs_the_file_its_lines`).
        let _ = writeln!(std::io::stderr().lock(), "{text}");
    }
}

/// `line` within `max` bytes, cut at a character boundary.
fn cut(mut line: String, max: usize) -> String {
    if line.len() > max {
        let mut at = max;
        while !line.is_char_boundary(at) {
            at -= 1;
        }
        line.truncate(at);
    }
    line
}

/// Append `bytes` to `path`, first rotating it to `<path>.1` when they
/// would take it past [`MAX_LOG_FILE_BYTES`]. Opened per line: the lines
/// are few, and a handle kept open would write on into a file another
/// bridge of the same endpoint had rotated away.
fn append(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let len = match std::fs::metadata(path) {
        Ok(meta) => meta.len(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(e) => return Err(e),
    };
    let added = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if len > 0 && len.saturating_add(added) > MAX_LOG_FILE_BYTES {
        let mut rotated = path.as_os_str().to_owned();
        rotated.push(".1");
        std::fs::rename(path, rotated)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path)?.write_all(bytes)
}

/// A directory failure's class, for `status.log.file_error`.
const fn persist_class(error: &PersistError) -> &'static str {
    match error {
        PersistError::DirectoryNotPrivate { .. } => "directory_not_private",
        PersistError::UnsupportedPlatform => "unsupported_platform",
        _ => "directory",
    }
}

fn wall_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn private_state() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
                .expect("0700");
        }
        dir
    }

    fn endpoint() -> EndpointId {
        EndpointId::parse("claude".to_owned()).expect("an endpoint")
    }

    /// Written far past the bound, the file never exceeds it, the
    /// rotated one holds what came before, and the newest line is in
    /// the live file.
    #[test]
    fn a_log_written_past_its_bound_rotates_and_never_exceeds_it() {
        let state = private_state();
        let mut log = Log::open(Level::Debug, state.path(), &endpoint());
        let path = log.file().expect("a file").to_owned();
        let mut rotated = path.as_os_str().to_owned();
        rotated.push(".1");
        let rotated = PathBuf::from(rotated);
        let mut rotations = 0;
        let mut last_len = 0;
        // About 3.5 times the bound in lines of ~60 bytes.
        for delay_ms in 0..60_000_u64 {
            log.write(&Line::Reconnect { delay_ms });
            let len = std::fs::metadata(&path).expect("the file").len();
            assert!(
                len <= MAX_LOG_FILE_BYTES,
                "{len} bytes after line {delay_ms}"
            );
            if len < last_len {
                rotations += 1;
            }
            last_len = len;
        }
        assert!(
            rotations >= 2,
            "the bound was passed: {rotations} rotations"
        );
        let old = std::fs::metadata(&rotated).expect("the rotated file").len();
        assert!(old <= MAX_LOG_FILE_BYTES, "the rotated file: {old} bytes");
        let live = std::fs::read_to_string(&path).expect("reads");
        assert!(
            live.trim_end().ends_with("the next open in 59999 ms"),
            "the newest line is in the live file"
        );
        assert_eq!(
            log.status()["file_error"],
            Value::Null,
            "every write landed"
        );
    }

    /// A line past [`MAX_LINE_BYTES`] is cut, at a character boundary.
    #[test]
    fn a_line_is_cut_at_its_bound() {
        let state = private_state();
        let mut log = Log::open(Level::Info, state.path(), &endpoint());
        log.write(&Line::Refused("é".repeat(MAX_LINE_BYTES)));
        let written = std::fs::read_to_string(log.file().expect("a file")).expect("reads");
        assert!(written.len() <= MAX_LINE_BYTES, "{} bytes", written.len());
        assert!(written.ends_with('\n'));
    }

    /// Below the level nothing is written; at it, the line is, with its
    /// instant and level; the file is owner-only.
    #[test]
    fn the_level_decides_what_is_written() {
        let state = private_state();
        let mut log = Log::open(Level::Warn, state.path(), &endpoint());
        log.write(&Line::SessionLost);
        log.write(&Line::Reconnect { delay_ms: 250 });
        let path = log.file().expect("a file").to_owned();
        assert!(!path.exists(), "info and debug are below warn");
        log.write(&Line::HostLineTooLong(7));
        let written = std::fs::read_to_string(&path).expect("reads");
        assert!(
            written.contains(" warn  claude-channel: a host line over 7 bytes was dropped\n"),
            "{written}"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path).expect("meta").permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert_eq!(log.status()["level"], "warn");
    }

    /// A state directory another account could write gets no file, and
    /// `status` says why; the bridge is not refused for it.
    #[cfg(unix)]
    #[test]
    fn an_unguarded_state_directory_gets_no_file() {
        use std::os::unix::fs::PermissionsExt as _;
        let state = private_state();
        std::fs::set_permissions(state.path(), std::fs::Permissions::from_mode(0o777))
            .expect("0777");
        let log = Log::open(Level::Info, state.path(), &endpoint());
        assert_eq!(log.file(), None);
        assert_eq!(log.status()["file_error"], "directory_not_private");
        assert_eq!(log.status()["file"], Value::Null);
    }

    #[test]
    fn the_levels_parse_by_name_and_nothing_else() {
        for level in [Level::Error, Level::Warn, Level::Info, Level::Debug] {
            assert_eq!(Level::parse(level.as_str()), Some(level));
        }
        for bad in ["", "INFO", "trace", "off", "information"] {
            assert_eq!(Level::parse(bad), None, "{bad}");
        }
        assert_eq!(Level::default(), Level::Info);
    }
}
