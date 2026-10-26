// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The test display's clipboard, owned as another program owns it when
//! the person copies something there: this module's own X client. It
//! answers a request for the list of what it holds and for the one type
//! it holds, and nothing else -- or, as a hostile program would, never
//! answers, or answers an image in incremental chunks and keeps sending
//! them. So a case needs no clipboard tool on the host, and the
//! client reads what a desktop program would offer it.
//!
//! The display has one clipboard: a case that owns it holds
//! [`crate::display::exclusive`] for its run.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use x11rb::connection::Connection as _;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ChangeWindowAttributesAux, ConnectionExt as _, CreateWindowAux, EventMask,
    PropMode, Property, SELECTION_NOTIFY_EVENT, SelectionNotifyEvent, SelectionRequestEvent,
    WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

/// How the owner answers a request for what it holds.
pub(crate) enum Holding {
    /// `bytes` of the type, in one property.
    Whole(Vec<u8>),
    /// No request answered, not even for the list of what it holds: the
    /// requester cannot learn whether there is an image.
    Silent,
    /// An image announced as incremental, then one byte after another,
    /// never ending.
    Endless,
    /// An image of `total` bytes sent incrementally as the protocol asks,
    /// a chunk each time the requester takes the last one, announced as
    /// far smaller; [`Owner::taken_when_gone`] counts what the requester
    /// took, once its window is destroyed.
    Chunked { total: usize },
}

/// The incremental chunk [`Holding::Chunked`] sends, well inside one
/// request without the big-requests extension.
const CHUNK: usize = 128 * 1024;

/// The clipboard, owned until this is dropped.
pub(crate) struct Owner {
    stop: Arc<AtomicBool>,
    taken: Arc<AtomicUsize>,
    gone: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Owner {
    /// Own the clipboard on `$DISPLAY`, offering `kind` (a MIME type such
    /// as `image/png`) and answering as `holding` says; the ownership is
    /// the server's before this returns.
    pub(crate) fn hold(kind: &str, holding: Holding) -> Self {
        let (conn, screen) = RustConnection::connect(None).expect("the test display answers");
        let atom = |name: &[u8]| {
            conn.intern_atom(false, name)
                .expect("an atom request")
                .reply()
                .expect("an atom")
                .atom
        };
        let clipboard = atom(b"CLIPBOARD");
        let atoms = Atoms {
            targets: atom(b"TARGETS"),
            incr: atom(b"INCR"),
            held: atom(kind.as_bytes()),
        };
        let root = conn.setup().roots[screen].root;
        let window = conn.generate_id().expect("an id");
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new(),
        )
        .expect("a window");
        conn.set_selection_owner(window, clipboard, x11rb::CURRENT_TIME)
            .expect("the clipboard");
        let owner = conn
            .get_selection_owner(clipboard)
            .expect("a request")
            .reply()
            .expect("the clipboard's owner")
            .owner;
        assert_eq!(owner, window, "the clipboard is this case's");
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let taken = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&taken);
        let gone = Arc::new(AtomicBool::new(false));
        let seen_gone = Arc::clone(&gone);
        let thread = std::thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                let Ok(event) = conn.poll_for_event() else {
                    return;
                };
                let Some(Event::SelectionRequest(r)) = event else {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                };
                if matches!(holding, Holding::Silent) {
                    continue;
                }
                if r.target == atoms.targets {
                    let ok = conn
                        .change_property32(
                            PropMode::REPLACE,
                            r.requestor,
                            r.property,
                            AtomEnum::ATOM,
                            &[atoms.targets, atoms.held],
                        )
                        .is_ok();
                    notify(&conn, &r, ok);
                    continue;
                }
                if r.target != atoms.held {
                    notify(&conn, &r, false);
                    continue;
                }
                match &holding {
                    Holding::Whole(bytes) => {
                        let ok = conn
                            .change_property8(
                                PropMode::REPLACE,
                                r.requestor,
                                r.property,
                                atoms.held,
                                bytes,
                            )
                            .is_ok();
                        notify(&conn, &r, ok);
                    }
                    Holding::Silent => unreachable!("answered nothing above"),
                    Holding::Endless => endless(&conn, &atoms, &r, &stopped),
                    Holding::Chunked { total } => {
                        chunked(&conn, &atoms, &r, *total, &stopped, &counted, &seen_gone);
                    }
                }
            }
        });
        Self {
            stop,
            taken,
            gone,
            thread: Some(thread),
        }
    }
}

struct Atoms {
    targets: Atom,
    incr: Atom,
    held: Atom,
}

/// Tell the requester its request is answered, or refused.
fn notify(conn: &RustConnection, r: &SelectionRequestEvent, answered: bool) {
    let event = SelectionNotifyEvent {
        response_type: SELECTION_NOTIFY_EVENT,
        sequence: 0,
        time: r.time,
        requestor: r.requestor,
        selection: r.selection,
        target: r.target,
        property: if answered { r.property } else { x11rb::NONE },
    };
    let _ = conn.send_event(false, r.requestor, EventMask::NO_EVENT, event);
    let _ = conn.flush();
}

/// The incremental transfer, never ended: announced, then once the
/// requester has taken the announcement, a byte appended after another,
/// so that the requester always has another to take, until the case
/// stops it.
fn endless(conn: &RustConnection, atoms: &Atoms, r: &SelectionRequestEvent, stop: &AtomicBool) {
    let watched = conn.change_window_attributes(
        r.requestor,
        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
    );
    let announced = conn.change_property32(
        PropMode::REPLACE,
        r.requestor,
        r.property,
        atoms.incr,
        &[1_000_000],
    );
    if watched.is_err() || announced.is_err() {
        return;
    }
    notify(conn, r, true);
    // The transfer starts when the requester deletes the announcement.
    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        match conn.poll_for_event() {
            Ok(Some(Event::PropertyNotify(e)))
                if e.window == r.requestor
                    && e.atom == r.property
                    && e.state == Property::DELETE =>
            {
                break;
            }
            Ok(Some(_)) => {}
            Ok(None) => std::thread::sleep(Duration::from_millis(1)),
            Err(_) => return,
        }
    }
    // One append a request, each sent at once. Whether the requester finds
    // a byte at every read is the server's scheduling: measured on Xvfb,
    // it drains the property within a few bytes and ends there, so this
    // proves the transfer ends in time, not the deadline's check against
    // a queue that never runs dry (`next_before`'s unit test holds that).
    while !stop.load(Ordering::Relaxed) {
        if conn
            .change_property8(
                PropMode::APPEND,
                r.requestor,
                r.property,
                atoms.held,
                &[0x89],
            )
            .is_err()
            || conn.flush().is_err()
        {
            return;
        }
        // Drain what the server sends back, so this client's own queue
        // does not grow while the requester's does.
        while let Ok(Some(_)) = conn.poll_for_event() {}
    }
}

impl Owner {
    /// The bytes of a [`Holding::Chunked`] image the requester took, once
    /// its read is over: each chunk counts once the requester deletes it,
    /// and the count is read only after the owner has seen the requester's
    /// window destroyed, which the server reports after every deletion the
    /// requester made of that window's property, so no deletion is still
    /// on its way to the count.
    pub(crate) async fn taken_when_gone(&self) -> usize {
        let deadline = std::time::Instant::now() + crate::common::PATIENCE;
        while !self.gone.load(Ordering::Acquire) {
            assert!(
                std::time::Instant::now() < deadline,
                "the requester's window was never destroyed"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        self.taken.load(Ordering::Acquire)
    }
}

/// Wait for the requester to delete the transfer's property, the
/// protocol's ask for the next chunk; `false` when the case stops first,
/// or when the requester's window is destroyed, which sets `gone`.
fn deleted(
    conn: &RustConnection,
    r: &SelectionRequestEvent,
    stop: &AtomicBool,
    gone: &AtomicBool,
) -> bool {
    loop {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        match conn.poll_for_event() {
            Ok(Some(Event::PropertyNotify(e)))
                if e.window == r.requestor
                    && e.atom == r.property
                    && e.state == Property::DELETE =>
            {
                return true;
            }
            Ok(Some(Event::DestroyNotify(e))) if e.window == r.requestor => {
                gone.store(true, Ordering::Release);
                return false;
            }
            Ok(Some(_)) => {}
            Ok(None) => std::thread::sleep(Duration::from_millis(1)),
            Err(_) => return false,
        }
    }
}

/// The incremental transfer as the protocol has it: announced at 1000
/// bytes, then a chunk each time the requester deletes the last, then an
/// empty chunk to end it, counting each chunk taken into `taken`.
fn chunked(
    conn: &RustConnection,
    atoms: &Atoms,
    r: &SelectionRequestEvent,
    total: usize,
    stop: &AtomicBool,
    taken: &AtomicUsize,
    gone: &AtomicBool,
) {
    let watched = conn.change_window_attributes(
        r.requestor,
        &ChangeWindowAttributesAux::new()
            .event_mask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY),
    );
    let announced = conn.change_property32(
        PropMode::REPLACE,
        r.requestor,
        r.property,
        atoms.incr,
        &[1000],
    );
    if watched.is_err() || announced.is_err() {
        return;
    }
    notify(conn, r, true);
    let chunk = vec![0x89; CHUNK];
    let mut sent = 0;
    while deleted(conn, r, stop, gone) {
        // The deletion that asks for this chunk took the one before it.
        taken.store(sent, Ordering::Release);
        let n = CHUNK.min(total - sent);
        if conn
            .change_property8(
                PropMode::REPLACE,
                r.requestor,
                r.property,
                atoms.held,
                &chunk[..n],
            )
            .is_err()
            || conn.flush().is_err()
        {
            return;
        }
        if n == 0 {
            return;
        }
        sent += n;
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
