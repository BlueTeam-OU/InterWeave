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
    /// The hub's count of start answers when its discard finished, if one
    /// did: the screen then waits for the restart's answer.
    pub(crate) done_at: Option<u64>,
}

/// A start's answer, as the hub said it (android-platform's `StoreAnswer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Answer {
    /// The store opened.
    Opened,
    /// The store refused: this screen.
    Refused(StoreScreen),
    /// The start was refused for a reason no store screen shows.
    OtherRefusal,
}

/// What follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Follow {
    /// Nothing changes.
    Keep,
    /// A fresh flow on this screen.
    Show(StoreScreen),
    /// The screen goes, its discard's notice shown if one ran.
    Hide,
}

/// The decision for one turn: the last start's `answer` and the hub's
/// count of answers with it (`generation`), `None` before any start
/// answered.
///
/// A finished discard waits for the restart: a turn before the restart
/// answers keeps the finished flow -- and with it the notice Chats shows
/// next -- and only an answer counted past the discard's ends it (#377
/// review F3). Then, as on any start (contact-cards spec section 7.3,
/// Done), an opened store and a start refused elsewhere both leave for
/// Chats with the notice, and a store refusal starts its screen over.
/// Before a discard, a start refused elsewhere keeps the screen: nothing
/// says the store opened.
pub(crate) const fn follow(
    shown: Option<ShownState>,
    answer: Option<Answer>,
    generation: u64,
) -> Follow {
    let Some(answer) = answer else {
        return Follow::Keep;
    };
    let Some(s) = shown else {
        return match answer {
            Answer::Refused(screen) => Follow::Show(screen),
            Answer::Opened | Answer::OtherRefusal => Follow::Keep,
        };
    };
    match (s.done_at, answer) {
        (_, Answer::Opened) => Follow::Hide,
        (Some(at), _) if generation <= at => Follow::Keep,
        (Some(_), Answer::OtherRefusal) => Follow::Hide,
        (None, Answer::OtherRefusal) => Follow::Keep,
        (None, Answer::Refused(screen)) if same(s.screen, screen) => Follow::Keep,
        (_, Answer::Refused(screen)) => Follow::Show(screen),
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

    const REFUSED_R: Option<Answer> = Some(Answer::Refused(R));
    const REFUSED_K: Option<Answer> = Some(Answer::Refused(K));
    const OPENED: Option<Answer> = Some(Answer::Opened);
    const OTHER: Option<Answer> = Some(Answer::OtherRefusal);

    #[test]
    fn a_refusal_shows_its_screen_and_an_opened_store_hides_it() {
        assert_eq!(follow(None, REFUSED_K, 1), Follow::Show(K));
        assert_eq!(follow(Some(shown(K, None)), REFUSED_K, 1), Follow::Keep);
        assert_eq!(follow(Some(shown(K, None)), REFUSED_R, 2), Follow::Show(R));
        assert_eq!(follow(Some(shown(R, None)), OPENED, 2), Follow::Hide);
        assert_eq!(follow(None, OPENED, 2), Follow::Keep);
        assert_eq!(follow(None, None, 0), Follow::Keep, "no start answered");
        assert_eq!(follow(Some(shown(R, None)), None, 0), Follow::Keep);
    }

    /// A start refused elsewhere is not an opened store: with no discard
    /// it shows nothing and keeps a shown screen (#377 review r3).
    #[test]
    fn a_start_refused_elsewhere_keeps_the_screen_before_a_discard() {
        assert_eq!(follow(None, OTHER, 1), Follow::Keep);
        assert_eq!(follow(Some(shown(R, None)), OTHER, 2), Follow::Keep);
        assert_eq!(follow(Some(shown(K, None)), OTHER, 2), Follow::Keep);
    }

    /// The restart not yet answered: a turn keeps the finished flow, so
    /// its notice survives to Chats, whatever the answer then counted --
    /// one said before the discard ended included (#377 review r3, risk 2).
    #[test]
    fn a_turn_before_the_restart_answers_keeps_the_finished_flow() {
        assert_eq!(follow(Some(shown(R, Some(3))), REFUSED_R, 3), Follow::Keep);
        assert_eq!(follow(Some(shown(K, Some(3))), REFUSED_R, 2), Follow::Keep);
        assert_eq!(follow(Some(shown(R, Some(3))), OTHER, 3), Follow::Keep);
        assert_eq!(follow(Some(shown(R, Some(3))), OPENED, 4), Follow::Hide);
    }

    /// The restart refused again: the screen starts over, live.
    #[test]
    fn a_restart_refused_again_starts_the_screen_over() {
        assert_eq!(
            follow(Some(shown(K, Some(3))), REFUSED_K, 4),
            Follow::Show(K)
        );
        assert_eq!(
            follow(Some(shown(K, Some(3))), REFUSED_R, 4),
            Follow::Show(R)
        );
    }

    /// The restart refused elsewhere: Done opens the app as on any start,
    /// so the screen goes, its notice shown (#377 review r3).
    #[test]
    fn a_restart_refused_elsewhere_leaves_for_chats() {
        assert_eq!(follow(Some(shown(R, Some(3))), OTHER, 4), Follow::Hide);
    }
}
