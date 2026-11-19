// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identity screens' Copy `PeerId` where the app makes its own copy,
//! and that copy's removal under `IdAlreadyCreated`
//! (design/2026-10-19-android-identity.md section 6; the limits are
//! section 12's).
//!
//! The copy goes through a [`PeerIdClipboard`] the root installs
//! ([`crate::View::set_peer_id_clipboard`]); a view without one copies
//! through the window's field, as the desktop does, and removes nothing.
//! Each copy carries a [`ClipMark`] in its clip's description, so the
//! removal tells the app's own copy apart by the description alone: the
//! seam has no way to read what the clipboard holds, and the host tests
//! hold that no description is asked for until the window has the focus
//! with the device unlocked, the conditions the platform gives a
//! description under (`ClipboardService.clipboardAccessAllowed`).
//!
//! Nothing here is kept past the process: a removal still waiting when
//! the process ends is dropped, and the copy stays (section 12).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// One copy's mark, carried in its clip's description: the clipboard
/// still holds the app's copy exactly while its description carries the
/// mark of the app's last copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipMark(pub u64);

/// The system clipboard as the identity screens' Copy `PeerId` reaches
/// it where the app makes its own copy. It offers no read of the
/// clipboard's content: the platform tells the person when an app reads
/// it, and the removal needs only the description (section 6).
pub trait PeerIdClipboard {
    /// Put `peer_id` on the clipboard as the app's copy, its description
    /// carrying a mark no earlier copy carried, which is returned.
    fn copy(&self, peer_id: &str) -> ClipMark;

    /// Clear the clipboard if its clip's description still carries
    /// `mark`, and leave it otherwise: asks for the description, never
    /// for the content. Asked only while the window has the focus and
    /// the device is unlocked.
    fn remove_if_marked(&self, mark: ClipMark);
}

/// The app's copies of a `PeerId` and the one removal they can owe.
#[derive(Default)]
pub(crate) struct PeerClip {
    board: RefCell<Option<Rc<dyn PeerIdClipboard>>>,
    /// The app's last copy through the board: what it copied, and its mark.
    last: RefCell<Option<(String, ClipMark)>>,
    /// A removal owed and not yet asked: it waits for the window's focus
    /// with the device unlocked.
    owed: Cell<Option<ClipMark>>,
    /// The window lacks the focus. A window starts focused; the root
    /// reports each change.
    unfocused: Cell<bool>,
    /// The device is locked, as the root last reported it.
    locked: Cell<bool>,
}

impl PeerClip {
    pub(crate) fn install(&self, board: Rc<dyn PeerIdClipboard>) {
        *self.board.borrow_mut() = Some(board);
    }

    /// Copy `peer` through the board, if one is installed: false, and
    /// nothing copied here, without one.
    pub(crate) fn copy(&self, peer: &str) -> bool {
        let Some(board) = self.board.borrow().clone() else {
            return false;
        };
        let mark = board.copy(peer);
        *self.last.borrow_mut() = Some((peer.to_owned(), mark));
        true
    }

    /// A page showed `void` as a `PeerId` no phone will hold (C4 under
    /// `IdAlreadyCreated`): if the app's last copy was of it, its removal
    /// is owed, asked now or once the window has the focus with the
    /// device unlocked. Owed once: the same page shown again owes
    /// nothing more.
    pub(crate) fn void_shown(&self, void: &str) {
        let copied = self
            .last
            .borrow()
            .as_ref()
            .filter(|(peer, _)| peer == void)
            .map(|&(_, mark)| mark);
        if let Some(mark) = copied {
            self.last.borrow_mut().take();
            self.owed.set(Some(mark));
            self.ask();
        }
    }

    pub(crate) fn window_focused(&self, focused: bool) {
        self.unfocused.set(!focused);
        self.ask();
    }

    pub(crate) fn device_locked(&self, locked: bool) {
        self.locked.set(locked);
        self.ask();
    }

    /// The platform found the window unfocused or the device locked when
    /// the removal was asked, whatever was last reported: it is owed
    /// again, and asked at the next report that both hold.
    pub(crate) fn removal_deferred(&self, mark: ClipMark) {
        if self.owed.get().is_none() {
            self.owed.set(Some(mark));
        }
    }

    fn ask(&self) {
        if self.unfocused.get() || self.locked.get() {
            return;
        }
        let Some(mark) = self.owed.take() else {
            return;
        };
        if let Some(board) = self.board.borrow().clone() {
            board.remove_if_marked(mark);
        }
    }
}
