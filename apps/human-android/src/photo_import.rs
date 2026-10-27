// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The own card's photo, read from the file the picker chose (contact-
//! cards design 5.2; j68 P3-1): one read at a time, bounded in bytes and in
//! time, and given up as soon as it is no longer wanted -- a newer pick,
//! the editor closed, the Activity destroyed. The Activity opens the file
//! under a cancellable open of its own and hands its descriptor here, with
//! what is left of the pick's one deadline.
//!
//! The read runs on a worker of its own, and the caller waits on the worker
//! rather than on the descriptor: `poll` bounds a pipe or a socket, but a
//! regular file -- a FUSE one a provider serves through
//! `openProxyFileDescriptor` included -- polls readable at once and its
//! `read` then blocks for as long as the provider stalls (#483 F1). So the
//! caller answers at the deadline or the cancel whatever the worker is
//! doing, and a worker stuck in such a read is counted: with
//! [`MAX_READ_WORKERS`] alive a pick is unusable at once rather than holding
//! one more thread. On every target, so the host tests it with a pipe that
//! stalls and a reader that never returns.

use std::fs::File;
use std::io::Read as _;
use std::os::fd::{AsFd as _, OwnedFd};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec, poll};

/// How long a pick may take, its open and its read together: the Activity
/// hands over what its open left of it (`MainActivity.PHOTO_DEADLINE_MS`).
pub(crate) const PHOTO_DEADLINE: Duration = Duration::from_secs(30);

/// The read workers alive at once, those a provider holds past their
/// deadline or cancel included: with this many, a pick is refused without
/// a read (#483 F1).
pub(crate) const MAX_READ_WORKERS: usize = 3;

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
    /// Workers still reading, the one a caller waits on included.
    workers: Arc<AtomicUsize>,
}

/// A worker counted while it lives, whichever way it ends.
struct Counted(Arc<AtomicUsize>);

impl Drop for Counted {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Imports {
    pub(crate) fn new() -> Self {
        Self {
            current: Mutex::new(None),
            workers: Arc::new(AtomicUsize::new(0)),
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
    /// descriptor is closed on every path: here when the read never
    /// starts, by the worker when it ends.
    pub(crate) fn read(&self, request: u64, fd: OwnedFd, deadline: Instant) -> Read {
        self.read_with(request, deadline, move |unwanted| {
            read_bounded(fd, deadline, unwanted, MAX_PHOTO_BYTES)
        })
    }

    /// `work` for `request` on a worker, answered by `deadline`, or within
    /// [`CANCEL_SLICE`] of the pick no longer being wanted, whatever the
    /// worker is doing. `work` is told through its flag when its answer is
    /// no longer waited for.
    fn read_with(
        &self,
        request: u64,
        deadline: Instant,
        work: impl FnOnce(&AtomicBool) -> Read + Send + 'static,
    ) -> Read {
        let unwanted = {
            let current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
            match current.as_ref() {
                Some(import) if import.request == request => Arc::clone(&import.unwanted),
                _ => return Read::NotWanted,
            }
        };
        if Instant::now() >= deadline {
            return self.done(request, Read::Unusable);
        }
        // Counted before it starts, so two picks at once cannot both pass.
        if self.workers.fetch_add(1, Ordering::SeqCst) >= MAX_READ_WORKERS {
            self.workers.fetch_sub(1, Ordering::SeqCst);
            return self.done(request, Read::Unusable);
        }
        let counted = Counted(Arc::clone(&self.workers));
        let (tx, rx) = std::sync::mpsc::channel();
        let flag = Arc::clone(&unwanted);
        let spawned = std::thread::Builder::new()
            .name("photo-read".to_owned())
            .spawn(move || {
                let _counted = counted;
                let _ = tx.send(work(&flag));
            });
        if spawned.is_err() {
            return self.done(request, Read::Unusable);
        }
        loop {
            if unwanted.load(Ordering::SeqCst) {
                return Read::NotWanted;
            }
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                // The worker is told, so a read `poll` bounds ends too.
                unwanted.store(true, Ordering::SeqCst);
                return self.done(request, Read::Unusable);
            };
            match rx.recv_timeout(left.min(CANCEL_SLICE)) {
                Ok(read) => {
                    // Given up while the worker finished: not handed on.
                    if unwanted.load(Ordering::SeqCst) {
                        return Read::NotWanted;
                    }
                    return self.done(request, read);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return self.done(request, Read::Unusable);
                }
            }
        }
    }

    /// `request`'s read is over: no longer the one waited for.
    fn done(&self, request: u64, read: Read) -> Read {
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
        // Readable, at its end or failed: read says which. For a pipe or a
        // socket it does not block now that poll answered; for a regular
        // file it may, which the caller does not wait on (`read_with`).
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

    /// A read that blocks and never looks at its flag, as `read` on a FUSE
    /// file whose provider stalls: it ends only when `release` is sent.
    fn stuck(release: std::sync::mpsc::Receiver<()>) -> impl FnOnce(&AtomicBool) -> Read + Send {
        move |_| {
            let _ = release.recv();
            Read::Bytes(b"late".to_vec())
        }
    }

    /// #483 F1: a read that blocks past `poll` -- a regular file polls
    /// readable at once -- is answered at the deadline, and within a slice
    /// of a newer pick, whatever its worker does. Mutation: the caller
    /// waiting on the worker's answer alone.
    #[test]
    fn a_read_stuck_in_the_kernel_is_answered_at_the_deadline_and_the_cancel() {
        let imports = Arc::new(Imports::new());
        let (release, held) = std::sync::mpsc::channel();
        imports.begin(1);
        // On a thread of its own: a wait that does not end fails the test
        // rather than hanging it.
        let (tx, rx) = std::sync::mpsc::channel();
        {
            let imports = Arc::clone(&imports);
            std::thread::spawn(move || {
                let started = Instant::now();
                let read = imports.read_with(1, started + Duration::from_millis(300), stuck(held));
                let _ = tx.send((read, started.elapsed()));
            });
        }
        let (read, took) = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("answered at the deadline");
        assert_eq!(read, Read::Unusable);
        assert!(
            took >= Duration::from_millis(300),
            "not before it: {took:?}"
        );

        let (release_2, held_2) = std::sync::mpsc::channel();
        imports.begin(2);
        let waiting = {
            let imports = Arc::clone(&imports);
            std::thread::spawn(move || {
                imports.read_with(2, Instant::now() + Duration::from_secs(60), stuck(held_2))
            })
        };
        std::thread::sleep(Duration::from_millis(200));
        let given_up = Instant::now();
        imports.begin(3);
        assert_eq!(waiting.join().expect("the wait"), Read::NotWanted);
        assert!(
            given_up.elapsed() < Duration::from_secs(2),
            "within a slice"
        );
        let _ = (release.send(()), release_2.send(()));
    }

    /// Reads stuck past their answer are bounded: with
    /// [`MAX_READ_WORKERS`] alive a pick is unusable at once, and once they
    /// end a pick reads again. Mutation: the count not taken.
    #[test]
    fn stuck_reads_are_bounded_and_free_their_place_when_they_end() {
        let imports = Imports::new();
        let mut releases = Vec::new();
        for request in 1..=u64::try_from(MAX_READ_WORKERS).expect("small") {
            let (release, held) = std::sync::mpsc::channel();
            releases.push(release);
            imports.begin(request);
            assert_eq!(
                imports.read_with(
                    request,
                    Instant::now() + Duration::from_millis(50),
                    stuck(held)
                ),
                Read::Unusable,
                "given up at its deadline, its worker still stuck"
            );
        }
        imports.begin(10);
        let refused_at = Instant::now();
        assert_eq!(
            imports.read_with(10, Instant::now() + PHOTO_DEADLINE, |_| Read::Bytes(vec![
                1
            ])),
            Read::Unusable,
            "at the bound: refused without a read"
        );
        assert!(refused_at.elapsed() < Duration::from_secs(1), "at once");

        for release in releases {
            let _ = release.send(());
        }
        let freed = Instant::now() + Duration::from_secs(5);
        while imports.workers.load(Ordering::SeqCst) > 0 {
            assert!(Instant::now() < freed, "the stuck workers ended");
            std::thread::sleep(Duration::from_millis(10));
        }
        imports.begin(11);
        assert_eq!(
            imports.read_with(11, Instant::now() + PHOTO_DEADLINE, |_| Read::Bytes(vec![
                2
            ])),
            Read::Bytes(vec![2])
        );
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
