// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The test display's clipboard, owned as another program owns it when
//! the person copies something there: this module's own X client, which
//! answers a request for the list of what it holds and for the one type
//! it holds, and nothing else. So a case needs no clipboard tool on the
//! host, and the client reads what a desktop program would offer it.
//!
//! The display has one clipboard: a case that owns it holds
//! [`crate::display::exclusive`] for its run.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use x11rb::connection::Connection as _;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, PropMode, SELECTION_NOTIFY_EVENT,
    SelectionNotifyEvent, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

/// The clipboard, owned until this is dropped.
pub(crate) struct Owner {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Owner {
    /// Own the clipboard on `$DISPLAY`, holding `bytes` as `kind` (a MIME
    /// type such as `image/png`); the ownership is the server's before
    /// this returns. Kept small: answered in one property, not in chunks.
    pub(crate) fn hold(kind: &str, bytes: Vec<u8>) -> Self {
        let (conn, screen) = RustConnection::connect(None).expect("the test display answers");
        let atom = |name: &[u8]| {
            conn.intern_atom(false, name)
                .expect("an atom request")
                .reply()
                .expect("an atom")
                .atom
        };
        let clipboard = atom(b"CLIPBOARD");
        let targets = atom(b"TARGETS");
        let held = atom(kind.as_bytes());
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
        let thread = std::thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                let Ok(event) = conn.poll_for_event() else {
                    return;
                };
                let Some(Event::SelectionRequest(r)) = event else {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                };
                let answered = if r.target == targets {
                    conn.change_property32(
                        PropMode::REPLACE,
                        r.requestor,
                        r.property,
                        AtomEnum::ATOM,
                        &[targets, held],
                    )
                    .is_ok()
                } else if r.target == held {
                    conn.change_property8(PropMode::REPLACE, r.requestor, r.property, held, &bytes)
                        .is_ok()
                } else {
                    false
                };
                let notify = SelectionNotifyEvent {
                    response_type: SELECTION_NOTIFY_EVENT,
                    sequence: 0,
                    time: r.time,
                    requestor: r.requestor,
                    selection: r.selection,
                    target: r.target,
                    property: if answered { r.property } else { x11rb::NONE },
                };
                let _ = conn.send_event(false, r.requestor, EventMask::NO_EVENT, notify);
                let _ = conn.flush();
            }
        });
        Self {
            stop,
            thread: Some(thread),
        }
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
