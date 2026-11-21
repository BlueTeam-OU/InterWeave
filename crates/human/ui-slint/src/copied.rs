// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identity screens' `PeerIdCopied` snackbar, below Android 13
//! (design/2026-10-19-android-identity.md section 5, The copy's
//! confirmation). The view owns it rather than the flow: the identity
//! screens run before any session exists, so no model's snackbar can
//! carry it, and nothing the flow decides reads it. What closes it is
//! read off the pages the root hands the view ([`Copied::page_shown`]),
//! plus the two things no page says -- the system prompt raised over the
//! screen and the app going to the background -- which the root reports
//! ([`Copied::close`]).

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

use slint::SharedString;

use crate::generated::{AppWindow, IdScreen, IdentityPage};

/// What the page showed when the snackbar opened: a page that differs in
/// any of these shows something else on, over or in place of the screen
/// it confirmed the copy on.
#[derive(Debug, Clone)]
struct ShownOn {
    screen: IdScreen,
    title: SharedString,
    notice: SharedString,
    notices_raised: i32,
    peer_id: SharedString,
    starting: SharedString,
}

impl ShownOn {
    fn of(page: &IdentityPage) -> Self {
        Self {
            screen: page.screen,
            title: page.title.clone(),
            notice: page.notice.clone(),
            notices_raised: page.notices_raised,
            peer_id: page.peer_id.clone(),
            starting: page.starting.clone(),
        }
    }

    /// `page` still shows the screen the copy was confirmed on, with
    /// nothing else on it: no other notice (a notice raised again with
    /// the same text counts), no progress state, no dialog, the same
    /// `PeerId` with its Copy, and no held start's line that was not
    /// there. Continue's hold alone changes none of these, so a C4 whose
    /// seal runs keeps the snackbar until the seal's result shows.
    fn still(&self, page: &IdentityPage) -> bool {
        let now = Self::of(page);
        let starting_came = !now.starting.is_empty() && now.starting != self.starting;
        self.screen == now.screen
            && self.title == now.title
            && self.notice == now.notice
            && self.notices_raised == now.notices_raised
            && self.peer_id == now.peer_id
            && !starting_came
            && page.waiting.is_empty()
            && !page.dialog
            && !page.copy_label.is_empty()
    }
}

/// The confirmation: what it opened on while it shows, and its one timer.
#[derive(Default)]
pub(crate) struct Copied {
    on: RefCell<Option<ShownOn>>,
    timer: slint::Timer,
}

impl Copied {
    /// A copy on the page `window` shows: the snackbar opens, or, already
    /// open, its time starts again from zero. Up for `ms`, the platform's
    /// recommended time held between 6 s and 2 minutes
    /// ([`crate::snackbar_ms`]).
    pub(crate) fn copied(self: &Rc<Self>, window: &AppWindow, text: &str, ms: u64) {
        let page = window.get_identity();
        *self.on.borrow_mut() = Some(ShownOn::of(&page));
        window.set_identity_copied(text.into());
        let this: Weak<Self> = Rc::downgrade(self);
        let weak = slint::ComponentHandle::as_weak(window);
        // `start` replaces a running timer's interval and restarts it.
        self.timer.start(
            slint::TimerMode::SingleShot,
            Duration::from_millis(ms),
            move || {
                if let (Some(this), Some(window)) = (this.upgrade(), weak.upgrade()) {
                    this.close(&window);
                }
            },
        );
    }

    /// The root showed `page`, or hid the screens (`None`): the snackbar
    /// closes when the page leaves the screen it confirmed, or shows
    /// anything else on it ([`ShownOn::still`]).
    pub(crate) fn page_shown(&self, window: &AppWindow, page: Option<&IdentityPage>) {
        let stays = match (self.on.borrow().as_ref(), page) {
            (None, _) => return,
            (Some(on), Some(page)) => on.still(page),
            (Some(_), None) => false,
        };
        if !stays {
            self.close(window);
        }
    }

    /// It closes, and shows again only on another copy.
    pub(crate) fn close(&self, window: &AppWindow) {
        self.timer.stop();
        self.on.borrow_mut().take();
        window.set_identity_copied(SharedString::new());
    }
}
