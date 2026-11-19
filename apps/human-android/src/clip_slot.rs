// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identity screens' copy of a `PeerId` on its way to the Activity's
//! Kotlin side (identity note section 6): the window marks each copy and
//! asks for it (`ask::CLIP_COPY`, the mark its argument); Kotlin takes the
//! text by that mark (`Native.peerIdClip`) and puts it on the clipboard
//! with the mark in the clip's label, which the removal later compares
//! without reading the clip's content. On every target, so the host tests
//! the marks and the slot.

use std::sync::{Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ask_word::MAX_ARG;

/// The last copy asked for and not yet taken, and the last mark given.
pub(crate) struct ClipSlot {
    held: Mutex<(u64, Option<String>)>,
}

impl ClipSlot {
    pub(crate) const fn new() -> Self {
        Self {
            held: Mutex::new((0, None)),
        }
    }

    /// Hold `text` for Kotlin under a new mark, which is returned: above
    /// every mark this process gave, and from the clock's microseconds
    /// when those are higher, so a later process's marks start above an
    /// earlier one's while the clock runs forward. Within an ask's
    /// argument ([`MAX_ARG`]), which it travels in. A copy not yet taken
    /// is replaced: only the last copy is the app's.
    pub(crate) fn put(&self, text: &str) -> u64 {
        self.put_at(text, now_micros())
    }

    fn put_at(&self, text: &str, now: u64) -> u64 {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        let mark = (held.0 + 1).max(now & MAX_ARG) & MAX_ARG;
        *held = (mark, Some(text.to_owned()));
        mark
    }

    /// The text held under `mark`, taken; empty when another copy has
    /// replaced it or it was taken already.
    pub(crate) fn take(&self, mark: u64) -> String {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        if held.0 == mark {
            held.1.take().unwrap_or_default()
        } else {
            String::new()
        }
    }
}

fn now_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_micros()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each mark is above the last, the clock's when that is higher,
    /// within an ask's argument. Mutation: `put_at` taking the clock
    /// alone gives the same mark twice at one instant and fails "above";
    /// dropping the `MAX_ARG` mask fails "fits".
    #[test]
    fn each_mark_is_new_and_fits_an_ask() {
        let slot = ClipSlot::new();
        let first = slot.put_at("a", 1_000);
        assert_eq!(first, 1_000, "from the clock");
        assert!(slot.put_at("b", 1_000) > first, "above, at one instant");
        assert!(slot.put_at("c", 5) > first, "above, the clock gone back");
        let big = slot.put_at("d", u64::MAX);
        assert!(big <= MAX_ARG, "fits");
        assert!(slot.put("e") <= MAX_ARG);
    }

    /// Kotlin takes the text by its mark, once; an older mark, replaced by
    /// a later copy, takes nothing. Mutation: `take` ignoring the mark
    /// hands the later copy to the older ask and fails here.
    #[test]
    fn the_text_is_taken_by_its_mark_once() {
        let slot = ClipSlot::new();
        let old = slot.put("first");
        let new = slot.put("second");
        assert_eq!(slot.take(old), "", "replaced");
        assert_eq!(slot.take(new), "second");
        assert_eq!(slot.take(new), "", "once");
    }
}
