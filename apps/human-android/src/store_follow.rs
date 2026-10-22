// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the Activity does with the hub's store answer at a turn (ADR-0060
//! decision 6; src/store.rs carries it out): a pure decision, on every
//! target, so the host tests it.

use interweave_human_ui_model::StoreScreen;

/// The store screen as shown now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShownState {
    /// Which screen.
    pub(crate) screen: StoreScreen,
    /// The hub's answer count when its discard finished, if one did: the
    /// screen then waits for the restart's answer.
    pub(crate) done_at: Option<u64>,
}

/// What follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Follow {
    /// Nothing changes.
    Keep,
    /// A fresh flow on this screen.
    Show(StoreScreen),
    /// The store opened: the screen goes, its discard's notice shown.
    Hide,
}

/// The decision for one turn: `refused` and `answers` as the hub last said
/// them (`answers` counts every store answer the view has had).
///
/// A finished discard waits for the restart: a turn before the restart
/// answers keeps the finished flow -- and with it the notice Chats shows
/// once the store opens -- and only a refusal that arrived AFTER the
/// discard starts the screen over (#377 review F3).
pub(crate) const fn follow(
    shown: Option<ShownState>,
    refused: Option<StoreScreen>,
    answers: u64,
) -> Follow {
    match (refused, shown) {
        (None, Some(_)) => Follow::Hide,
        (None, None) => Follow::Keep,
        (Some(screen), None) => Follow::Show(screen),
        (Some(screen), Some(s)) => match s.done_at {
            Some(at) if answers > at => Follow::Show(screen),
            Some(_) => Follow::Keep,
            None if same(s.screen, screen) => Follow::Keep,
            None => Follow::Show(screen),
        },
    }
}

const fn same(a: StoreScreen, b: StoreScreen) -> bool {
    matches!(
        (a, b),
        (StoreScreen::Recovery, StoreScreen::Recovery)
            | (StoreScreen::KeyLost, StoreScreen::KeyLost)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: StoreScreen = StoreScreen::Recovery;
    const K: StoreScreen = StoreScreen::KeyLost;

    const fn shown(screen: StoreScreen, done_at: Option<u64>) -> ShownState {
        ShownState { screen, done_at }
    }

    #[test]
    fn a_refusal_shows_its_screen_and_an_opened_store_hides_it() {
        assert_eq!(follow(None, Some(K), 1), Follow::Show(K));
        assert_eq!(follow(Some(shown(K, None)), Some(K), 1), Follow::Keep);
        assert_eq!(follow(Some(shown(K, None)), Some(R), 2), Follow::Show(R));
        assert_eq!(follow(Some(shown(R, None)), None, 2), Follow::Hide);
        assert_eq!(follow(None, None, 2), Follow::Keep);
    }

    /// The restart not yet answered: a turn keeps the finished flow, so
    /// its notice survives to the open.
    #[test]
    fn a_turn_before_the_restart_answers_keeps_the_finished_flow() {
        assert_eq!(follow(Some(shown(R, Some(3))), Some(R), 3), Follow::Keep);
        assert_eq!(follow(Some(shown(R, Some(3))), None, 4), Follow::Hide);
    }

    /// The restart refused again: the screen starts over, live.
    #[test]
    fn a_restart_refused_again_starts_the_screen_over() {
        assert_eq!(follow(Some(shown(K, Some(3))), Some(K), 4), Follow::Show(K));
        assert_eq!(follow(Some(shown(K, Some(3))), Some(R), 4), Follow::Show(R));
    }
}
