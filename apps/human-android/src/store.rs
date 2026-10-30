// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The store's screens on the Activity (ADR-0060 decision 6;
//! design/2026-10-21-contact-cards.md section 7): when the network service
//! did not start because of the message store, the window shows R --
//! recovery mode -- or K -- the key the Keystore invalidated -- in place
//! of the chats, on the identity screens' frame. The flow is ui-model's;
//! this carries out what it asks: the plan and the confirmed discard
//! through the Service host, with the Keystore bridge, and Try again,
//! the restart after a discard and Close through the Activity's Kotlin
//! side. Nothing is deleted before the person confirmed the plan.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use interweave_human_android_platform::{
    ServiceHost, StoreAnswer, StoreDiscarded, StoreGeneration, StoreRefusal,
};
use interweave_human_store::at_rest::{DiscardPlan, DiscardScope};
use interweave_human_ui_model::{DiscardKind, StoreEffect, StoreRecovery, StoreScreen};
use interweave_human_ui_slint::{
    IdentityInput, StoreOutcome, View, apply_store, defer, store_page,
};

use crate::jni::{PROFILE, log};
use crate::keystore::StoreKeystore;
use crate::onboard::{ask, ask_activity, platform};
use crate::store_follow::{Answer, Follow, ShownState, follow as decide};

/// The flow while a store screen shows, and the plan it confirmed.
struct Shown {
    flow: StoreRecovery,
    plan: Option<DiscardPlan>,
    view: Weak<RefCell<View>>,
    /// The hub's count of start answers when the discard finished, read
    /// under the start lock: an answer counted past it is the restart's.
    done_at: Option<StoreGeneration>,
}

thread_local! {
    // On the window's thread only.
    static SHOWN: RefCell<Option<Shown>> = const { RefCell::new(None) };
}

const fn screen(refused: StoreRefusal) -> StoreScreen {
    match refused {
        StoreRefusal::Recovery => StoreScreen::Recovery,
        StoreRefusal::KeyLost => StoreScreen::KeyLost,
    }
}

const fn answer(said: StoreAnswer) -> Answer {
    match said {
        StoreAnswer::Opened => Answer::Opened,
        StoreAnswer::Refused(refused) => Answer::Refused(screen(refused)),
        StoreAnswer::OtherRefusal => Answer::OtherRefusal,
    }
}

const fn kind(scope: DiscardScope) -> DiscardKind {
    match scope {
        DiscardScope::Everything => DiscardKind::Full,
        DiscardScope::UnfinishedPlaintext => DiscardKind::Partial,
    }
}

/// Follow the last start's answer as the hub said it, with the hub's
/// count: show the screen it calls for, or, once a start opened the store
/// -- or the restart after a discard was refused elsewhere -- take it
/// away, with the discard's notice in Chats when one ran. Called at every
/// turn.
pub(crate) fn follow(view: &Rc<RefCell<View>>, said: Option<(StoreAnswer, StoreGeneration)>) {
    SHOWN.with(|shown| {
        let mut shown = shown.borrow_mut();
        let state = shown.as_ref().map(|s| ShownState {
            screen: s.flow.screen(),
            done_at: s.done_at.map(StoreGeneration::count),
        });
        let generation = said.map_or(0, |(_, generation)| generation.count());
        match decide(state, said.map(|(said, _)| answer(said)), generation) {
            Follow::Keep => {}
            Follow::Show(screen) => {
                // The desktop and the phone hold one identity until Stage 22.
                *shown = Some(Shown {
                    flow: StoreRecovery::new(screen, 1),
                    plan: None,
                    view: Rc::downgrade(view),
                    done_at: None,
                });
                view.borrow()
                    .on_identity(move |input| defer(move || input_on(&input)));
                render(shown.as_ref());
            }
            Follow::Hide => {
                let s = shown.take();
                let view = view.borrow();
                view.show_identity(None);
                if let Some(notice) = s.and_then(|s| s.flow.notice()) {
                    view.show_once(notice);
                }
            }
        }
    });
}

fn render(shown: Option<&Shown>) {
    if let Some(s) = shown
        && let Some(view) = s.view.upgrade()
    {
        view.borrow().show_identity(Some(store_page(&s.flow)));
    }
}

fn input_on(input: &IdentityInput) {
    let outcome = SHOWN.with(|shown| {
        let mut shown = shown.borrow_mut();
        let s = shown.as_mut()?;
        let outcome = apply_store(&mut s.flow, input);
        if matches!(input, IdentityInput::Dialog(false) | IdentityInput::Back) {
            s.plan = None;
        }
        render(Some(s));
        Some(outcome)
    });
    match outcome {
        Some(StoreOutcome::Effect(StoreEffect::Plan)) => defer(plan),
        Some(StoreOutcome::Effect(StoreEffect::Discard)) => defer(discard),
        Some(StoreOutcome::TryAgain) => ask_activity(ask::RESTART),
        Some(StoreOutcome::Close) => ask_activity(ask::FINISH),
        Some(StoreOutcome::Shown) | None => {}
    }
}

/// Build the plan: the complete copy opened read-only under the key.
fn plan() {
    let built = platform()
        .ok_or_else(|| "no platform yet".to_owned())
        .and_then(|(dir, cipher)| {
            ServiceHost::global()
                .plan_store_discard(&dir, PROFILE, &StoreKeystore(cipher))
                .map_err(|e| format!("{e:?}"))
        });
    SHOWN.with(|shown| {
        let mut shown = shown.borrow_mut();
        let Some(s) = shown.as_mut() else { return };
        match built {
            Ok(plan) => {
                s.flow.planned(kind(plan.scope()));
                s.plan = Some(plan);
            }
            Err(why) => {
                log(&format!("the discard could not be planned: {why}"));
                s.flow.failed();
            }
        }
        render(Some(s));
    });
}

/// Run the plan the person confirmed, then start the service again,
/// which recreates the store.
fn discard() {
    let confirmed = SHOWN.with(|shown| {
        shown
            .borrow_mut()
            .as_mut()
            .and_then(|s| s.plan.take())
            .map(DiscardPlan::confirm)
    });
    let done = match (confirmed, platform()) {
        (Some(confirmed), Some((dir, cipher))) => ServiceHost::global()
            .discard_store(&dir, PROFILE, &StoreKeystore(cipher), confirmed)
            .map_err(|e| format!("{e:?}")),
        _ => Err("no confirmed plan".to_owned()),
    };
    let ok = SHOWN.with(|shown| {
        let mut shown = shown.borrow_mut();
        let Some(s) = shown.as_mut() else {
            return false;
        };
        let ok = match done {
            Ok(StoreDiscarded {
                discarded,
                answered,
            }) => {
                s.flow.discarded(kind(discarded.scope));
                s.done_at = Some(answered);
                true
            }
            Err(why) => {
                log(&format!("the discard failed: {why}"));
                s.flow.failed();
                false
            }
        };
        render(Some(s));
        ok
    });
    if ok {
        ask_activity(ask::RESTART);
    }
}
