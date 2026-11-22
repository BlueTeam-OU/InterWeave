// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An identity flow outlives the window that shows it (onboard.rs,
//! `window_ended` and `start`): each Activity instance runs its own
//! `android_main` thread, and a recreation the manifest does not absorb
//! -- a change of the system or app language -- ends that thread. A flow
//! that died with its thread showed Welcome again on C4 over an identity
//! already sealed, and lost a held start (identity note section 6, A
//! return to a screen showing `IdStartBusy`; section 7, The app leaves
//! the screen). What happens to a flow when its window ends, what the
//! next `Native.identityState` answers while one is parked, and how the
//! next window takes it up are decided here; the answers a window could
//! not take are held here for the next ([`Owed`]). On every target, so
//! the host tests it.

use std::collections::VecDeque;

use crate::ready_check::Over;

/// What a window's end does with the flow it showed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AtWindowEnd {
    /// Kept for the next window of this process, which shows it as it
    /// was: the words, the profile's lock, a held start and its clock.
    Park,
    /// Ended with the window: nothing is kept for the next.
    End,
}

/// Whether a flow finished the Activity itself (Back on a first screen,
/// `Effect::Finish`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Finish {
    /// It did not: the Activity ended for another reason -- a recreation,
    /// or a finish that was not the flow's ([`Destroy`] says which).
    Open,
    /// It did, and its window's end ends it as the finish asked.
    Asked,
}

/// How the Activity whose window ended was destroyed, as its `onDestroy`
/// said before the window's thread ended (`Native.windowEnding`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Destroy {
    /// Not a finish: a recreation for a configuration change, or the
    /// system's destroy of an instance it may make again. The task stays.
    Kept,
    /// A finish (`isFinishing`, and not changing configurations): the
    /// task removed from Recents, `finish()` by the system or the app.
    /// Identity note section 7: a finish drops the words, and the next
    /// launch is a launch.
    Finishing,
}

/// The end of a window showing a flow opened `over` a launch, Settings
/// or a running window, `finish` saying whether the flow itself finished
/// the Activity and `destroy` how the Activity went.
///
/// A finishing destroy ends the flow, whoever finished it, as
/// `Finish::Asked` does: kept, a launch later in the same process would
/// show the words a finish dropped (identity note section 7, Finish,
/// cancellation, completion or process death). Show my phrase ends: S2's
/// words show again without a new prompt only in the same Activity
/// instance, and a new instance is a new display, behind the prompt
/// (section 7) -- MainActivity.onDestroy brings the service back for it.
/// A flow that finished the Activity ends as the finish asked. Any other
/// is parked. The decision is
/// `a_window_end_parks_only_an_open_flow_past_a_destroy_that_is_not_a_finish`'s;
/// that S2 then shows behind a new prompt is BLOCKED on a device run:
/// case D4 of `device-runs/identity-recreation-cases.md`.
pub(crate) const fn at_window_end(over: Over, finish: Finish, destroy: Destroy) -> AtWindowEnd {
    if matches!(destroy, Destroy::Finishing)
        || matches!(finish, Finish::Asked)
        || matches!(over, Over::Settings)
    {
        AtWindowEnd::End
    } else {
        AtWindowEnd::Park
    }
}

/// What `Native.identityState` answers while a flow lives -- shown by a
/// window or parked -- read before the record: the record's state is the
/// flow's to decide, and a read here would answer ready over a flow that
/// still shows (R4 before its Continue, a restore committed), and the
/// Activity would bind the service under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlowCheck {
    /// No start shows: `FLAG_SECURE` on, and nothing bound or started,
    /// as for a first run.
    BeforeStart,
    /// The flow's screen shows the start it asked (a held C4, R4, R3 or
    /// R2, or `IdStartBusy` on one): `FLAG_SECURE` on, and the binding's
    /// own decision of a start -- a busy or locked start retried, one
    /// running left alone -- as when the identity is ready.
    Started,
}

/// The check for a flow whose service was `asked` at some point and whose
/// current screen `shows` a start (`IdentityFlow::start_shows`). Started
/// only for both
/// (`a_flow_answers_started_only_while_its_screen_shows_the_start`):
/// `asked` alone is a latch, set at the first ask and kept while the
/// screen moves on, and a flow back on K1 after R3's `IdStartBusy` would
/// have the Activity restart the service under it, with nothing held,
/// and the next restore refused busy.
pub(crate) const fn flow_check(asked: bool, shows: bool) -> FlowCheck {
    if asked && shows {
        FlowCheck::Started
    } else {
        FlowCheck::BeforeStart
    }
}

/// Whether the chats are attached under a flow's window: a fact the flow
/// records of its window, not inferred from where it was opened -- a
/// K1 opened at a window's start over a running process (`Over::Running`)
/// has none attached until it hands over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Attached {
    /// The flow's own hand-over attaches them.
    No,
    /// Attached under it: the flow was raised over attached chats (K1
    /// raised over a running window, Show my phrase from Settings), or a
    /// held C4 or R4 handed them over.
    Yes,
}

impl Attached {
    /// A flow raised over a window whose chats are attached, or not.
    pub(crate) const fn opened(over_attached_chats: bool) -> Self {
        if over_attached_chats {
            Self::Yes
        } else {
            Self::No
        }
    }

    /// The window that takes the parked flow up attaches its chats at
    /// once, the flow's page on top, exactly where the window that parked
    /// it had them; otherwise the flow's own hand-over attaches them, and
    /// attached twice the window would make two roots
    /// (`the_next_window_attaches_its_chats_where_the_last_had_them`).
    pub(crate) const fn at_rebind(self) -> bool {
        matches!(self, Self::Yes)
    }
}

/// The most answers [`Owed`] holds: far above the few a window's end can
/// leave (a prompt's answer, a dropped Unlock ask, a refused start, a
/// showing's start held), so
/// reaching it is a fault, logged by the caller.
pub(crate) const OWED_MAX: usize = 32;

/// Answers for the window that holds the flow, from threads that cannot
/// tell whether one does: `invoke_on_window` sent between two windows
/// may reach the ended window's loop and be lost while it says it
/// succeeded -- the risk this guards against, which no device case yet
/// drives (today's hooks time the service's start, not the gap between
/// a window's end and the next window's start). Each answer is held here and the window told to take them; a window
/// that does not take them leaves them to the next, which takes them
/// once it holds the flow (`owed_answers_are_taken_once_in_order`). At
/// most [`OWED_MAX`]: past it the oldest is dropped and handed back
/// (`owed_answers_are_bounded_dropping_the_oldest`).
pub(crate) struct Owed<T> {
    queue: VecDeque<T>,
}

impl<T> Owed<T> {
    pub(crate) const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Hold `answer`; the oldest held, dropped to make room, if the queue
    /// was full.
    pub(crate) fn push(&mut self, answer: T) -> Option<T> {
        let dropped = if self.queue.len() >= OWED_MAX {
            self.queue.pop_front()
        } else {
            None
        };
        self.queue.push_back(answer);
        dropped
    }

    /// Every answer held, oldest first, the queue left empty.
    pub(crate) fn take(&mut self) -> Vec<T> {
        self.queue.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A launch's flow and a K1 raised over a running window are parked
    /// at a destroy that is not a finish, so a recreation on C4 keeps C4
    /// and not Welcome; a finishing destroy ends every flow, whoever
    /// finished, so a later launch in the process does not show the words
    /// (section 7); Show my phrase ends with its instance, and a flow
    /// that finished the Activity ends. Mutations: ignoring `destroy`
    /// fails the loop's `Finishing` assertion; ignoring `finish`, its
    /// `Asked` one; parking Settings, the first assertion.
    #[test]
    fn a_window_end_parks_only_an_open_flow_past_a_destroy_that_is_not_a_finish() {
        use Destroy::{Finishing, Kept};
        use Finish::{Asked, Open};
        assert_eq!(at_window_end(Over::Settings, Open, Kept), AtWindowEnd::End);
        for (finish, destroy) in [(Asked, Kept), (Open, Finishing), (Asked, Finishing)] {
            assert_eq!(
                at_window_end(Over::Settings, finish, destroy),
                AtWindowEnd::End,
                "Settings, {finish:?}, {destroy:?}"
            );
        }
        for over in [Over::Launch, Over::Running] {
            assert_eq!(
                at_window_end(over, Open, Kept),
                AtWindowEnd::Park,
                "{over:?}"
            );
            assert_eq!(
                at_window_end(over, Open, Finishing),
                AtWindowEnd::End,
                "{over:?}: a finish not the flow's"
            );
            assert_eq!(
                at_window_end(over, Asked, Kept),
                AtWindowEnd::End,
                "{over:?}"
            );
            assert_eq!(
                at_window_end(over, Asked, Finishing),
                AtWindowEnd::End,
                "{over:?}"
            );
        }
    }

    /// A flow answers started exactly while it asked its start and its
    /// screen shows that start: a held C4, or `IdStartBusy` on C4, has
    /// the Activity decide a retry; R3's Back off `IdStartBusy` -- the
    /// start asked, its screen gone -- binds nothing, nor does a flow
    /// that never asked, R4 before its Continue. Mutations: answering
    /// `Started` on `asked` alone fails the third assertion; on `shows`
    /// alone, the fourth.
    #[test]
    fn a_flow_answers_started_only_while_its_screen_shows_the_start() {
        // A held C4, or `IdStartBusy` there: asked, and shown.
        assert_eq!(flow_check(true, true), FlowCheck::Started);
        // R4 before its Continue: nothing asked, nothing shown.
        assert_eq!(flow_check(false, false), FlowCheck::BeforeStart);
        // R3 under `IdStartBusy`, then Back: asked, shown no more.
        assert_eq!(flow_check(true, false), FlowCheck::BeforeStart);
        // A screen held before its ask is carried out (C4's seal running).
        assert_eq!(flow_check(false, true), FlowCheck::BeforeStart);
    }

    /// The chats are attached at the next window exactly where the last
    /// had them: under a flow raised over attached chats, and under one
    /// whose held screen handed them over; never under a flow opened at
    /// a window's start that has not -- a K1 kept in a running process
    /// included, whose window had none attached -- since its hand-over
    /// attaches them. Mutations: `opened` ignoring its argument either
    /// way fails one of the first two assertions; `at_rebind` answering
    /// true for every flow fails the first.
    #[test]
    fn the_next_window_attaches_its_chats_where_the_last_had_them() {
        // A launch's flow, or a kept K1 at a window's start.
        assert!(!Attached::opened(false).at_rebind());
        // K1 raised over a running window, Show my phrase.
        assert!(Attached::opened(true).at_rebind());
        // A held C4 or R4 that handed them over.
        assert!(Attached::Yes.at_rebind());
    }

    /// Answers come back in the order they were held, once: a prompt's
    /// answer and a later dropped ask apply in that order, and a second
    /// take finds nothing to apply again. Mutation: `take` draining only
    /// the oldest fails the first `take`'s assertion.
    #[test]
    fn owed_answers_are_taken_once_in_order() {
        let mut owed = Owed::new();
        assert_eq!(owed.push(1), None);
        assert_eq!(owed.push(2), None);
        assert_eq!(owed.take(), vec![1, 2]);
        assert!(owed.take().is_empty());
    }

    /// The queue never holds more than `OWED_MAX`: the answer past it
    /// drops the oldest, handed back for the caller to log, and every
    /// later one is kept. Mutation: removing the length check makes the
    /// first push past the bound answer `None` and the take hold one
    /// more.
    #[test]
    fn owed_answers_are_bounded_dropping_the_oldest() {
        let mut owed = Owed::new();
        for answer in 0..OWED_MAX {
            assert_eq!(owed.push(answer), None);
        }
        assert_eq!(owed.push(OWED_MAX), Some(0));
        let held = owed.take();
        assert_eq!(held.len(), OWED_MAX);
        assert_eq!(held.first(), Some(&1));
        assert_eq!(held.last(), Some(&OWED_MAX));
    }
}
