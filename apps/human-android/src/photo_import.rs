// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The own card's photo, read from the file the picker chose (contact-
//! cards design 5.2; j68 P3-1): one read at a time, bounded in bytes and in
//! time, and given up as soon as it is no longer wanted -- a newer pick,
//! the editor closed, the Activity destroyed. The Activity opens the file
//! under a cancellable open of its own and hands its descriptor here; this
//! waits on it with `poll`, so a provider that answers a prefix and then
//! nothing holds no thread past the deadline. On every target, so the host
//! tests it with a pipe that stalls.

use std::fs::File;
use std::io::Read as _;
use std::os::fd::{AsFd as _, OwnedFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec, poll};

/// How long a pick's file may take to read, from the moment its descriptor
/// arrives. The Activity gives its open the same.
pub(crate) const PHOTO_DEADLINE: Duration = Duration::from_secs(30);

/// The bound on a picked file: the desktop's image bound
/// (`interweave_human_ui_slint::MAX_IMAGE_BYTES`), one byte over telling
/// too large from at the bound.
pub(crate) const MAX_PHOTO_BYTES: usize = 32 * 1024 * 1024;

/// How often a read waiting on its descriptor looks at its flag: the
/// longest a read no longer wanted goes on.
const CANCEL_SLICE: Duration = Duration::from_millis(100);

/// What a read came to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Read {
    /// The file's bytes, within the bound.
    Bytes(Vec<u8>),
    /// Past the bound, past the deadline, or the descriptor failed: handed
    /// on empty, which the editor says cannot be used.
    Unusable,
    /// The pick is no longer wanted: nothing is handed on.
    NotWanted,
}

/// The pick a read runs for.
struct Import {
    request: u64,
    unwanted: Arc<AtomicBool>,
}

/// The editor's picks, the latest the only one wanted.
pub(crate) struct Imports {
    current: Mutex<Option<Import>>,
}

impl Imports {
    pub(crate) const fn new() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }

    /// The picker opened for `request`: a read an earlier pick runs is no
    /// longer wanted.
    pub(crate) fn begin(&self, request: u64) {
        let mut current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(earlier) = current.take() {
            earlier.unwanted.store(true, Ordering::SeqCst);
        }
        *current = Some(Import {
            request,
            unwanted: Arc::new(AtomicBool::new(false)),
        });
    }

    /// The editor closed, or the Activity went: no pick is wanted.
    pub(crate) fn abandon(&self) {
        if let Some(earlier) = self
            .current
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            earlier.unwanted.store(true, Ordering::SeqCst);
        }
    }

    /// Read `fd`, the file chosen for `request`, by `deadline`. The
    /// descriptor is closed on every path.
    pub(crate) fn read(&self, request: u64, fd: OwnedFd, deadline: Instant) -> Read {
        let unwanted = {
            let current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
            match current.as_ref() {
                Some(import) if import.request == request => Arc::clone(&import.unwanted),
                _ => return Read::NotWanted,
            }
        };
        let read = read_bounded(fd, deadline, &unwanted, MAX_PHOTO_BYTES);
        // Given up after the read began: what it found is not handed on.
        if unwanted.load(Ordering::SeqCst) {
            return Read::NotWanted;
        }
        let mut current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        if current.as_ref().is_some_and(|i| i.request == request) {
            *current = None;
        }
        read
    }
}

/// `fd`'s bytes up to `max`, waiting on it at most until `deadline` and
/// no longer than [`CANCEL_SLICE`] past `unwanted` being set.
fn read_bounded(fd: OwnedFd, deadline: Instant, unwanted: &AtomicBool, max: usize) -> Read {
    let mut file = File::from(fd);
    let mut out = Vec::new();
    let mut chunk = vec![0_u8; 64 * 1024];
    loop {
        if unwanted.load(Ordering::SeqCst) {
            return Read::NotWanted;
        }
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            return Read::Unusable;
        };
        let wait = left.min(CANCEL_SLICE);
        let timeout = Timespec {
            tv_sec: 0,
            tv_nsec: i64::from(wait.subsec_nanos()),
        };
        let mut fds = [PollFd::from_borrowed_fd(
            file.as_fd(),
            PollFlags::IN | PollFlags::PRI,
        )];
        match poll(&mut fds, Some(&timeout)) {
            Ok(0) | Err(rustix::io::Errno::INTR) => continue,
            Ok(_) => {}
            Err(_) => return Read::Unusable,
        }
        if fds[0].revents().contains(PollFlags::NVAL) {
            return Read::Unusable;
        }
        // Readable, at its end or failed: read says which, without
        // blocking now that poll answered.
        match file.read(&mut chunk) {
            Ok(0) => return Read::Bytes(out),
            Ok(n) => {
                out.extend_from_slice(&chunk[..n]);
                if out.len() > max {
                    return Read::Unusable;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Read::Unusable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    /// A provider's pipe: `prefix` written and the write end kept open,
    /// as a provider that answers part and then stalls.
    fn stalled(prefix: &[u8]) -> (OwnedFd, std::io::PipeWriter) {
        let (reader, mut writer) = std::io::pipe().expect("a pipe");
        writer.write_all(prefix).expect("the prefix");
        (OwnedFd::from(reader), writer)
    }

    /// A file's whole content, as a provider that answers.
    fn whole(content: &[u8]) -> OwnedFd {
        let (reader, mut writer) = std::io::pipe().expect("a pipe");
        writer.write_all(content).expect("the content");
        drop(writer);
        OwnedFd::from(reader)
    }

    /// `read` on a thread of its own: its answer and how long it took, so
    /// a read that does not end fails the test rather than hanging it.
    fn within(
        imports: &Arc<Imports>,
        request: u64,
        fd: OwnedFd,
        deadline: Duration,
    ) -> std::sync::mpsc::Receiver<(Read, Duration)> {
        let (tx, rx) = std::sync::mpsc::channel();
        let imports = Arc::clone(imports);
        std::thread::spawn(move || {
            let started = Instant::now();
            let read = imports.read(request, fd, started + deadline);
            let _ = tx.send((read, started.elapsed()));
        });
        rx
    }

    /// j68 P3-1's test: a provider that answers a prefix and then blocks is
    /// given up at the deadline, and a later valid pick is read. Mutation:
    /// the wait without a deadline.
    #[test]
    fn a_stalled_provider_is_given_up_at_the_deadline_and_a_later_pick_reads() {
        let imports = Arc::new(Imports::new());
        imports.begin(1);
        let (fd, _held) = stalled(b"\x89PNG");
        let (read, took) = within(&imports, 1, fd, Duration::from_millis(300))
            .recv_timeout(Duration::from_secs(5))
            .expect("given up at the deadline");
        assert_eq!(read, Read::Unusable);
        assert!(
            took >= Duration::from_millis(300),
            "not before it: {took:?}"
        );

        imports.begin(2);
        assert_eq!(
            imports.read(2, whole(b"an image"), Instant::now() + PHOTO_DEADLINE),
            Read::Bytes(b"an image".to_vec())
        );
    }

    /// A read a newer pick or a closed editor no longer wants ends within
    /// a slice, whatever its deadline, and hands nothing on. Mutation: the
    /// flag not looked at while waiting.
    #[test]
    fn a_read_no_longer_wanted_ends_at_once_and_hands_nothing_on() {
        for give_up in [|i: &Imports| i.begin(2), |i: &Imports| i.abandon()] {
            let imports = Arc::new(Imports::new());
            imports.begin(1);
            let (fd, _held) = stalled(b"part");
            let reading = within(&imports, 1, fd, Duration::from_secs(60));
            std::thread::sleep(Duration::from_millis(200));
            give_up(&imports);
            let (read, _) = reading
                .recv_timeout(Duration::from_secs(5))
                .expect("ended within a slice");
            assert_eq!(read, Read::NotWanted);
        }
    }

    /// A descriptor for a pick that is not the latest is not read at all.
    #[test]
    fn a_descriptor_for_an_earlier_pick_is_not_read() {
        let imports = Imports::new();
        imports.begin(1);
        imports.begin(2);
        assert_eq!(
            imports.read(1, whole(b"old"), Instant::now() + PHOTO_DEADLINE),
            Read::NotWanted
        );
        imports.abandon();
        assert_eq!(
            imports.read(2, whole(b"new"), Instant::now() + PHOTO_DEADLINE),
            Read::NotWanted,
            "nor one the editor gave up"
        );
    }

    /// Past the bound is unusable; at it, read. Mutation: `>=` for `>`.
    #[test]
    fn the_bound_is_one_byte_past_the_largest_read() {
        let flag = AtomicBool::new(false);
        let deadline = Instant::now() + PHOTO_DEADLINE;
        assert_eq!(
            read_bounded(whole(&[7; 10]), deadline, &flag, 10),
            Read::Bytes(vec![7; 10])
        );
        assert_eq!(
            read_bounded(whole(&[7; 11]), deadline, &flag, 10),
            Read::Unusable
        );
    }
}
