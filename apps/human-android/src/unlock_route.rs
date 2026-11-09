// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What answers a start refused until the person authenticates, in a
//! process that keeps running as in a fresh one
//! (design/2026-11-05-unlock-policy-pending.md §11, L1-L4; ADR-0042
//! A 2026-10-20): the Unlock prompt, the locked notice with focus, or
//! nothing. A pure decision, on every target, so the host tests it;
//! onboard.rs carries it out on Android.

/// What a refusal, or the Activity's own check, is answered with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    /// The Unlock prompt, raised by the Activity (F10).
    Prompt,
    /// No prompt: the locked notice and its Unlock, focused (F12).
    Notice,
    /// Nothing is shown: no window shows, and the Service never prompts
    /// (L4, F13).
    Nothing,
    /// An Unlock prompt asked before and not yet answered answers this one
    /// too: its grant runs the start held for it, its cancel focuses the
    /// notice. A second would be raised over the first's answer (#578
    /// review 5, risk 1).
    Asked,
}

/// The process's state the decision reads.
#[derive(Debug, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "four facts that change apart, every combination one the process can hold"
)]
pub(crate) struct UnlockRoute {
    /// An Activity's window shows: between its start and its stop.
    shown: bool,
    /// An Unlock prompt was cancelled or denied, or left while it showed,
    /// and no start has run since and no launch came: a return raises no
    /// prompt by itself (L2).
    declined: bool,
    /// The person's own action on a screen in front asked for the start
    /// now answering: R's Try again, a discard's restart, an identity
    /// screen's continue or S2's Done, a network grant (L1).
    person_asked: bool,
    /// An Unlock prompt was asked of the Activity and is not answered yet:
    /// raised, or waiting in the asks to be.
    prompting: bool,
}

impl UnlockRoute {
    pub(crate) const fn new() -> Self {
        Self {
            shown: false,
            declined: false,
            person_asked: false,
            prompting: false,
        }
    }

    /// The Activity's window started (true) or stopped (false) showing.
    pub(crate) const fn shown(&mut self, shown: bool) {
        self.shown = shown;
    }

    /// The Activity was created anew -- a launch, whatever the process:
    /// it prompts as a cold launch does (L3), a decline before it
    /// notwithstanding.
    pub(crate) const fn launched(&mut self) {
        self.declined = false;
    }

    /// The person's action asked for the next start (L1).
    pub(crate) const fn person_asked(&mut self) {
        self.person_asked = true;
    }

    /// An Unlock prompt was asked other than by this decision: the
    /// person's Unlock, or the presence gate's before a key operation.
    pub(crate) const fn asked(&mut self) {
        self.prompting = true;
    }

    /// The Unlock prompt asked was dropped before the Activity raised it:
    /// nothing will answer it.
    pub(crate) const fn unasked(&mut self) {
        self.prompting = false;
    }

    /// An Unlock prompt was answered: a grant ends a decline, a cancel or
    /// a denial is one.
    pub(crate) const fn prompted(&mut self, granted: bool) {
        self.declined = !granted;
        self.prompting = false;
    }

    /// A start answered anything but a refusal for the person: the
    /// person's ask is answered, and a start that `ran` ends a decline
    /// ("with no start run since", L2). A refused one is not a start that
    /// ran (L4).
    pub(crate) const fn answered(&mut self, ran: bool) {
        self.person_asked = false;
        if ran {
            self.declined = false;
        }
    }

    /// A start refused for the person: L1 prompts whatever came before, a
    /// window in front prompts unless a decline holds (L3, L2), and none
    /// shown shows nothing (L4). A prompt already asked answers it, the
    /// person's ask included.
    pub(crate) const fn refused(&mut self) -> Route {
        let asked = self.person_asked;
        self.person_asked = false;
        if !self.shown {
            Route::Nothing
        } else if self.prompting {
            Route::Asked
        } else if asked || !self.declined {
            self.prompt()
        } else {
            Route::Notice
        }
    }

    /// The Activity's own check at its start found the key needs the
    /// person, before any start: a launch, or a return with no decline,
    /// prompts (L3); a return after one shows the notice (L2). A prompt
    /// already asked -- a refused start's in this showing -- answers it.
    pub(crate) const fn checked(&mut self) -> Route {
        if self.prompting {
            Route::Asked
        } else if self.declined {
            Route::Notice
        } else {
            self.prompt()
        }
    }

    const fn prompt(&mut self) -> Route {
        self.prompting = true;
        Route::Prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn front() -> UnlockRoute {
        let mut route = UnlockRoute::new();
        route.shown(true);
        route
    }

    /// L1: the person's action prompts, a decline before it
    /// notwithstanding, and its cancel declines (A14, A15).
    #[test]
    fn l1_the_persons_restart_prompts_even_after_a_decline() {
        let mut route = front();
        route.person_asked();
        assert_eq!(route.refused(), Route::Prompt);
        route.prompted(false);
        route.person_asked();
        assert_eq!(route.refused(), Route::Prompt, "Try again pressed again");
        route.prompted(false);
        assert_eq!(
            route.refused(),
            Route::Notice,
            "the ask is spent: a later refusal is not the person's"
        );
    }

    /// L2: a return after a decline, with no start run since, shows the
    /// notice, at the check and at a refusal alike (A17).
    #[test]
    fn l2_a_return_after_a_decline_shows_the_notice() {
        let mut route = front();
        assert_eq!(route.checked(), Route::Prompt, "control: no decline");
        route.prompted(false);
        route.shown(false);
        route.shown(true);
        assert_eq!(route.checked(), Route::Notice);
        assert_eq!(route.refused(), Route::Notice);
    }

    /// L3: a launch prompts whatever was declined before it; a return
    /// prompts when the start that ran since ended the decline, and a
    /// grant ends it too (A17, A18).
    #[test]
    fn l3_a_launch_or_a_start_that_ran_ends_the_decline() {
        let mut route = front();
        route.prompted(false);
        route.launched();
        assert_eq!(route.checked(), Route::Prompt, "a launch");

        route.prompted(false);
        route.answered(true);
        assert_eq!(route.refused(), Route::Prompt, "a start ran since");

        route.prompted(false);
        route.prompted(true);
        assert_eq!(route.checked(), Route::Prompt, "a grant");
    }

    /// L4: with no window, nothing -- the person's ask included -- and a
    /// refused start is not one that ran, so a decline before it holds at
    /// the next return (A19).
    #[test]
    fn l4_no_window_shows_nothing_and_keeps_the_decline() {
        let mut route = front();
        route.shown(false);
        assert_eq!(route.refused(), Route::Nothing);
        route.person_asked();
        assert_eq!(route.refused(), Route::Nothing, "and spends the ask");
        route.shown(true);
        assert_eq!(route.refused(), Route::Prompt, "the next return: L3");

        route.prompted(false);
        route.shown(false);
        assert_eq!(route.refused(), Route::Nothing);
        route.shown(true);
        assert_eq!(route.refused(), Route::Notice, "the next return: L2");
    }

    /// A start answered elsewhere spends the person's ask: a later
    /// refusal is answered by the rule for what started it.
    #[test]
    fn an_answer_spends_the_persons_ask() {
        let mut route = front();
        route.prompted(false);
        route.person_asked();
        route.answered(false);
        assert_eq!(route.refused(), Route::Notice);
    }

    /// The Activity's check and a Service start find the lock in one
    /// showing: one prompt, in either order, the person's ask included
    /// (#578 review 5, risk 1).
    #[test]
    fn one_showing_raises_one_prompt() {
        let mut route = front();
        assert_eq!(route.checked(), Route::Prompt);
        assert_eq!(route.refused(), Route::Asked, "a refusal after the check");
        route.person_asked();
        assert_eq!(route.refused(), Route::Asked, "the person's ask too");

        let mut route = front();
        assert_eq!(route.refused(), Route::Prompt);
        assert_eq!(route.checked(), Route::Asked, "the check after a refusal");
    }

    /// The prompt asked other than by a refusal -- the person's Unlock, the
    /// presence gate -- answers a refusal too.
    #[test]
    fn a_prompt_asked_elsewhere_answers_a_refusal() {
        let mut route = front();
        route.asked();
        assert_eq!(route.refused(), Route::Asked);
        assert_eq!(route.checked(), Route::Asked);
    }

    /// An answer ends the prompt outstanding, a grant or a cancel, and so
    /// does an ask dropped unraised: the next refusal is routed by its
    /// rule again.
    #[test]
    fn an_answer_or_a_dropped_ask_ends_the_outstanding_prompt() {
        let mut route = front();
        assert_eq!(route.checked(), Route::Prompt);
        route.prompted(true);
        assert_eq!(route.refused(), Route::Prompt, "after a grant");
        route.prompted(false);
        assert_eq!(route.refused(), Route::Notice, "after a cancel: L2");

        route.person_asked();
        assert_eq!(route.refused(), Route::Prompt);
        route.unasked();
        route.person_asked();
        assert_eq!(route.refused(), Route::Prompt, "after the ask was dropped");
    }

    /// With no window, nothing, an outstanding prompt or not (L4).
    #[test]
    fn no_window_shows_nothing_with_a_prompt_outstanding() {
        let mut route = front();
        route.asked();
        route.shown(false);
        assert_eq!(route.refused(), Route::Nothing);
    }
}
