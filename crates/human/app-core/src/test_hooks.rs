// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! TEST BUILDS ONLY (the `test-hooks` feature): faults a test lays on the
//! facade's windowed reads -- a page or the contacts failing or held, a
//! photo slowed or unreadable -- so the desktop's headless tests drive
//! design/2026-10-27-start-window.md section 10's failure paths through
//! the real facade thread and a real store. Nothing that ships builds this
//! module (architect-cto's A1 for j68).

use std::collections::HashSet;
use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Duration;

/// What a hooked read does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fault {
    /// It reads as it would.
    #[default]
    Pass,
    /// It fails as a store that cannot be read does.
    Fail,
    /// It waits until the test sets another fault, or [`HOLD_LIMIT`]
    /// passes, then does what that says.
    Hold,
}

/// The longest a held read waits: a test that never releases one fails on
/// its own assertions rather than hanging the suite.
pub const HOLD_LIMIT: Duration = Duration::from_secs(20);

#[derive(Debug, Default)]
struct Faults {
    pages: Fault,
    contacts: Fault,
    photo_delay: Duration,
    unreadable_photos: HashSet<String>,
}

/// The faults one facade's reads consult, shared with the test that sets
/// them.
#[derive(Debug, Default)]
pub struct StoreFaults {
    faults: Mutex<Faults>,
    changed: Condvar,
}

/// Which read a fault is for.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Read {
    Page,
    Contacts,
}

impl StoreFaults {
    /// What page reads do from now.
    pub fn set_pages(&self, fault: Fault) {
        self.set(|f| f.pages = fault);
    }

    /// What reads of every contact do from now, the start's included.
    pub fn set_contacts(&self, fault: Fault) {
        self.set(|f| f.contacts = fault);
    }

    /// How long each photo read takes from now.
    pub fn set_photo_delay(&self, delay: Duration) {
        self.set(|f| f.photo_delay = delay);
    }

    /// `contact_id`'s photo reads as one that cannot be decoded.
    pub fn set_unreadable_photo(&self, contact_id: &str) {
        self.set(|f| {
            f.unreadable_photos.insert(contact_id.to_owned());
        });
    }

    fn set(&self, change: impl FnOnce(&mut Faults)) {
        change(&mut self.faults.lock().unwrap_or_else(PoisonError::into_inner));
        self.changed.notify_all();
    }

    /// Whether `read` may go on: false when it is to fail. A held read
    /// waits here for the test, at most [`HOLD_LIMIT`].
    pub(crate) fn admits(&self, read: Read) -> bool {
        let fault = |f: &Faults| match read {
            Read::Page => f.pages,
            Read::Contacts => f.contacts,
        };
        let guard = self.faults.lock().unwrap_or_else(PoisonError::into_inner);
        let (guard, _) = self
            .changed
            .wait_timeout_while(guard, HOLD_LIMIT, |f| fault(f) == Fault::Hold)
            .unwrap_or_else(PoisonError::into_inner);
        fault(&guard) != Fault::Fail
    }

    /// How long a photo read takes, and whether `contact_id`'s reads as
    /// unreadable.
    pub(crate) fn photo(&self, contact_id: &str) -> (Duration, bool) {
        let f = self.faults.lock().unwrap_or_else(PoisonError::into_inner);
        (f.photo_delay, f.unreadable_photos.contains(contact_id))
    }
}
