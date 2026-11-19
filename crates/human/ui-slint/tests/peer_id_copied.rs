// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Copy `PeerId`'s confirmation below Android 13 on the identity screens
//! (design/2026-10-19-android-identity.md section 5, The copy's
//! confirmation; acceptance 18's host part): the `PeerIdCopied` snackbar
//! on C4, R3 and R4 -- its time, its one announcement, what closes it,
//! where it sits -- and nothing at all where the platform confirms the
//! copy itself. C2's and S2's `PeerId` pages are not built: those screens
//! show their words beside the `PeerId`, so they carry no Copy. And, through
//! a test clipboard that records every ask, the app's own copy taken off
//! the clipboard under `IdAlreadyCreated` (section 6; acceptance 18's
//! clipboard runs).

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use common::Edges;
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use interweave_human_ui_model::{
    IdentityFlow, RuntimeHost, ScreenId, Start, StepFailed, UiModel, UiText, placeholder_en,
};
use interweave_human_ui_slint::{
    ClipMark, CopyFeedback, IdentityInput, PeerIdClipboard, TouchSettings, View, apply_identity,
    identity_page, snackbar_ms,
};
use interweave_transport_api::TransportIdentity;
use slint::ComponentHandle as _;

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

/// One word per letter but x, as the trace tests' list.
const WORDS: &[&str] = &[
    "act", "action", "actor", "bean", "cabin", "dawn", "eagle", "fabric", "galaxy", "habit",
    "icon", "jacket", "kale", "label", "machine", "napkin", "oak", "paddle", "quarter", "rabbit",
    "saddle", "table", "ugly", "vacant", "wagon", "yard", "zebra", "zone",
];

fn mine() -> TransportIdentity {
    TransportIdentity::parse("12D3KooW9tHTtS3inCZiYykw4u5G4frbjVFqhkmJX12gSNCVeH3e")
        .expect("a PeerId")
}

fn take_word(flow: &mut IdentityFlow, word: &str) {
    for letter in word.chars() {
        if flow.picker().and_then(|p| p.chip(word)).is_some() {
            break;
        }
        apply_identity(flow, IdentityInput::Key(letter));
    }
    apply_identity(flow, IdentityInput::Chip(word.to_owned()));
}

fn at_c4() -> IdentityFlow {
    let mut flow = IdentityFlow::new(WORDS, Start::FirstRun { void_paper: false });
    apply_identity(&mut flow, IdentityInput::Primary);
    let words: [&'static str; 24] = WORDS[..24].try_into().expect("24 words");
    flow.created(words, mine(), [3, 9]);
    apply_identity(&mut flow, IdentityInput::Primary);
    apply_identity(&mut flow, IdentityInput::Primary);
    for _ in 0..2 {
        let (position, _) = flow.check().expect("C3 checks a word");
        take_word(&mut flow, WORDS[position]);
    }
    assert_eq!(flow.screen(), ScreenId::C4, "control: at C4");
    flow
}

fn at_r3() -> IdentityFlow {
    let mut flow = IdentityFlow::new(WORDS, Start::FirstRun { void_paper: false });
    apply_identity(&mut flow, IdentityInput::Secondary);
    apply_identity(&mut flow, IdentityInput::Primary);
    for (at, word) in WORDS[..24].iter().enumerate() {
        apply_identity(&mut flow, IdentityInput::Slot(at));
        take_word(&mut flow, word);
    }
    apply_identity(&mut flow, IdentityInput::Primary);
    assert!(flow.waiting(), "control: the check runs");
    flow.checked(Some(mine()));
    assert_eq!(flow.screen(), ScreenId::R3, "control: at R3");
    flow
}

fn at_r4() -> IdentityFlow {
    let mut flow = at_r3();
    apply_identity(&mut flow, IdentityInput::Primary);
    flow.restored(Ok(()));
    assert_eq!(flow.screen(), ScreenId::R4, "control: at R4");
    flow
}

/// The identity page as the root runs it: each input applied to the flow
/// and the page shown again.
/// Let `ms` pass on the testing platform's clock and run what fell due.
fn elapse(ms: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(ms));
    slint::platform::update_timers_and_animations();
}

/// The testing platform, once per test thread: a test that opens a
/// second page on its thread must not set it again.
fn init_platform() {
    thread_local!(static SET: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
    SET.with(|set| {
        if !set.replace(true) {
            i_slint_backend_testing::init_no_event_loop();
        }
    });
}

struct Page {
    view: View,
    flow: IdentityFlow,
    queued: Rc<RefCell<Vec<IdentityInput>>>,
    applied: Vec<IdentityInput>,
}

impl Page {
    fn new(flow: IdentityFlow) -> Self {
        Self::sized(flow, 900.0, 900.0)
    }

    fn sized(flow: IdentityFlow, width: f32, height: f32) -> Self {
        init_platform();
        let mut view = View::new(RuntimeHost::Embedded).expect("a window");
        view.window()
            .window()
            .set_size(slint::LogicalSize::new(width, height));
        view.window().show().expect("shown");
        view.render(&UiModel::new());
        let queued = Rc::new(RefCell::new(Vec::new()));
        let into = Rc::clone(&queued);
        view.on_identity(move |input| into.borrow_mut().push(input));
        let mut page = Self {
            view,
            flow,
            queued,
            applied: Vec::new(),
        };
        page.settle();
        page
    }

    /// Apply what the page queued, show the flow's page again, and give
    /// the timers two short turns.
    fn settle(&mut self) {
        let queued: Vec<_> = self.queued.borrow_mut().drain(..).collect();
        for input in queued {
            self.applied.push(input.clone());
            apply_identity(&mut self.flow, input);
        }
        self.show();
        for _ in 0..2 {
            elapse(10);
        }
    }

    fn show(&self) {
        if self.flow.screen() == ScreenId::Done {
            self.view.show_identity(None);
        } else {
            self.view.show_identity(Some(identity_page(&self.flow)));
        }
        // A headless window makes an element inside an `if` only when the
        // tree is walked.
        let _ = ElementHandle::find_by_accessible_label(self.view.window(), "q").count();
    }

    fn key(&mut self, key: impl Into<slint::SharedString>) {
        use slint::platform::WindowEvent;
        let text = key.into();
        let window = self.view.window().window();
        window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
        window.dispatch_event(WindowEvent::KeyReleased { text });
        self.settle();
    }

    fn tab(&mut self) {
        self.key(slint::platform::Key::Tab);
    }

    fn backtab(&mut self) {
        self.key(slint::platform::Key::Backtab);
    }

    fn enter(&mut self) {
        self.key(slint::platform::Key::Return);
    }

    fn focus(&self) -> String {
        self.view.window().get_focus_id().to_string()
    }

    /// TAB until `key` has focus.
    fn tab_to(&mut self, key: &str) {
        for _ in 0..12 {
            if self.focus() == key {
                return;
            }
            self.tab();
        }
        panic!("TAB never reached {key:?}; focus on {:?}", self.focus());
    }

    /// Enter on Copy `PeerId`, reached by TAB.
    fn copy(&mut self) {
        self.tab_to("identity-copy");
        self.enter();
    }

    /// The focus keys TAB visits from where focus stands, `n` of them.
    fn tabs(&mut self, n: usize) -> Vec<String> {
        (0..n)
            .map(|_| {
                self.tab();
                self.focus()
            })
            .collect()
    }

    fn take(&mut self) -> Vec<IdentityInput> {
        std::mem::take(&mut self.applied)
    }

    /// The snackbar's text where it is drawn: the element labelled with
    /// it that is not one of the window's announcement slots.
    fn snack(&self) -> Option<ElementHandle> {
        let said = text(UiText::PeerIdCopied);
        let mut found =
            ElementHandle::find_by_accessible_label(self.view.window(), said).filter(|e| {
                !e.id()
                    .is_some_and(|id| id.ends_with("announce-a") || id.ends_with("announce-b"))
            });
        let first = found.next();
        assert!(found.next().is_none(), "one snackbar");
        if let Some(e) = &first {
            assert_eq!(e.accessible_role(), Some(AccessibleRole::Text), "text only");
        }
        first
    }

    fn shown(&self) -> bool {
        let drawn = self.snack().is_some();
        assert_eq!(
            drawn,
            !self.view.window().get_identity_copied().is_empty(),
            "drawn exactly while the view holds it"
        );
        drawn
    }

    /// The window's two announcement slots.
    fn slots(&self) -> [String; 2] {
        let w = self.view.window();
        [
            w.get_announcement_a().to_string(),
            w.get_announcement_b().to_string(),
        ]
    }

    fn button(&self, label: &str) -> ElementHandle {
        ElementHandle::find_by_accessible_label(self.view.window(), label)
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .unwrap_or_else(|| panic!("no button labelled {label:?}"))
    }

    fn element(&self, id: &str) -> ElementHandle {
        ElementHandle::find_by_element_id(self.view.window(), id)
            .next()
            .unwrap_or_else(|| panic!("no element {id}"))
    }
}

fn clipboard(view: &View) -> Option<String> {
    i_slint_core::window::WindowInner::from_pub(view.window().window())
        .context()
        .platform()
        .clipboard_text(i_slint_core::platform::Clipboard::DefaultClipboard)
}

/// The three screens with Copy, and the stop TAB reaches after Copy.
fn screens() -> [(&'static str, IdentityFlow); 3] {
    [("C4", at_c4()), ("R3", at_r3()), ("R4", at_r4())]
}

/// Acceptance 18, on C4, R3 and R4: Enter on Copy `PeerId` copies the
/// canonical string and shows `PeerIdCopied` with no action; focus stays
/// on Copy; TAB and Shift+TAB visit what they visit with no snackbar and
/// never the snackbar; and its text is said once, politely, not again as
/// the page is shown again. Mutation: the copy handler in app.slint not
/// calling `identity-peer-copied` shows nothing, and `copied.rs` setting
/// no text draws nothing; either fails here.
#[test]
fn copy_shows_the_confirmation_keeps_focus_and_says_it_once() {
    let said = text(UiText::PeerIdCopied);
    for (name, flow) in screens() {
        let mut page = Page::new(flow);
        page.tab_to("identity-copy");
        let without = page.tabs(6);
        page.tab_to("identity-copy");
        let back_without: Vec<String> = (0..6)
            .map(|_| {
                page.backtab();
                page.focus()
            })
            .collect();
        page.tab_to("identity-copy");
        assert!(!page.shown(), "{name}: control, none yet");
        assert_eq!(page.slots(), ["", ""], "{name}: control, nothing said");

        page.enter();
        assert_eq!(page.take(), [], "{name}: nothing for the flow");
        assert_eq!(
            clipboard(&page.view).as_deref(),
            Some(mine().as_str()),
            "{name}: the canonical string"
        );
        assert!(page.shown(), "{name}: shown");
        assert_eq!(page.focus(), "identity-copy", "{name}: focus stays");
        assert_eq!(
            page.slots().iter().filter(|s| *s == said).count(),
            1,
            "{name}: said"
        );
        let slots = page.slots();
        page.show();
        page.settle();
        assert_eq!(page.slots(), slots, "{name}: said once");

        assert_eq!(page.tabs(6), without, "{name}: TAB as with none");
        page.tab_to("identity-copy");
        let back: Vec<String> = (0..6)
            .map(|_| {
                page.backtab();
                page.focus()
            })
            .collect();
        assert_eq!(back, back_without, "{name}: Shift+TAB as with none");
        assert!(page.shown(), "{name}: still shown after the walk");
    }
}

/// A second copy while it shows starts its time again from zero and says
/// its text again (section 5): copied at 0 and again at 3 s, it is still
/// up at 8.9 s and gone at 9.1 s. Mutation: `Copied::copied` leaving a
/// running timer alone closes it at 6 s; `announce_in` skipped on a
/// second copy leaves the slots as they were; each fails here.
#[test]
fn a_second_copy_restarts_its_time_and_says_it_again() {
    let said = text(UiText::PeerIdCopied);
    let mut page = Page::new(at_r3());
    page.copy();
    let first = page.slots();
    elapse(3_000 - 20);
    page.enter();
    assert_eq!(page.focus(), "identity-copy", "control");
    let second = page.slots();
    assert_ne!(second, first, "said again");
    assert_eq!(second.iter().filter(|s| *s == said).count(), 1);
    elapse(5_900 - 20);
    assert!(page.shown(), "8.9 s after the first copy, 5.9 s after this");
    elapse(200);
    assert!(!page.shown(), "6 s after the second copy");
}

/// Its time is 6 s, the platform's recommended time when longer, and 2
/// minutes at most (section 5; the message-swipe spec's section 6
/// limits): with no input it closes then. Mutation: a fixed 6 s timer
/// fails the 30 s and the 2-minute runs.
#[test]
fn it_closes_at_its_time_held_between_6_s_and_2_minutes() {
    for (recommended, up) in [(0, 6_000), (30_000, 30_000), (300_000, 120_000)] {
        let mut page = Page::new(at_c4());
        page.view.set_touch_settings(TouchSettings {
            snackbar_ms: snackbar_ms(recommended),
            ..TouchSettings::default()
        });
        page.copy();
        assert!(page.shown(), "{recommended}: control");
        // The copy's settle turned 20 ms of mock time.
        elapse(up - 120);
        assert!(page.shown(), "{recommended}: up just before {up} ms");
        elapse(200);
        assert!(!page.shown(), "{recommended}: closed at {up} ms");
        assert_eq!(page.focus(), "identity-copy", "focus where it was");
    }
}

/// On C4 Continue does not close it: the seal starts and C4 stays, and
/// the snackbar's time goes on, closing at 6 s with the seal still
/// unanswered (section 5). Mutation: `ShownOn::still` comparing the
/// primary's being enabled closes it at Continue and fails here.
#[test]
fn c4s_continue_leaves_it_and_its_time_goes_on() {
    let mut page = Page::new(at_c4());
    page.copy();
    page.tab_to("identity-primary");
    page.take();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Primary], "control: Continue");
    assert!(page.flow.start_held(), "control: the seal runs, unanswered");
    assert_eq!(page.flow.screen(), ScreenId::C4);
    assert!(page.shown(), "Continue leaves it");
    elapse(5_800);
    assert!(page.shown(), "its time goes on under the seal");
    elapse(300);
    assert!(!page.shown(), "closed at its time");
}

/// What the seal's or the restore's result shows closes it (section 5):
/// on C4 `IdAlreadyCreated`, which hides Copy too, and `IdBusy`; on R3 `IdStorageFailed`. A notice raised again with
/// the text it showed closes it as well. Mutation: `ShownOn::still`
/// ignoring the notice fails every case.
#[test]
fn a_notice_the_result_shows_closes_it() {
    for failed in [StepFailed::AlreadyProvisioned, StepFailed::Busy] {
        let mut page = Page::new(at_c4());
        page.copy();
        page.tab_to("identity-primary");
        page.enter();
        assert!(page.shown(), "{failed:?}: control");
        page.flow.sealed(Err(failed.clone()));
        page.settle();
        assert!(!page.shown(), "{failed:?}: the notice closes it");
        let copy = identity_page(&page.flow).copy_label;
        assert_eq!(
            copy.is_empty(),
            failed == StepFailed::AlreadyProvisioned,
            "{failed:?}: Copy goes only with the words not used"
        );
    }

    let mut page = Page::new(at_r3());
    page.copy();
    page.tab_to("identity-primary");
    page.enter();
    assert!(page.flow.waiting(), "control: the restore runs");
    page.tab_to("identity-copy");
    page.enter();
    assert!(page.shown(), "Copy still copies while the restore runs");
    page.flow.restored(Err(StepFailed::Storage));
    page.settle();
    assert!(!page.shown(), "R3's notice closes it");
    // A notice already up, raised again with the same text.
    page.copy();
    assert!(page.shown(), "control: shown over the notice");
    page.tab_to("identity-notice-action");
    page.enter();
    page.flow.restored(Err(StepFailed::Storage));
    page.settle();
    assert!(!page.shown(), "the same notice raised again closes it");
}

/// The system prompt over the screen and the app going to the background
/// close it, which the root says (`View::close_peer_id_copied`), and the
/// screen shown again, as on the return, shows none (sections 5 and 7).
/// Mutation: `Copied::close` leaving `on` set makes no difference to what
/// shows, so the drawn text is what is checked; `close_peer_id_copied`
/// doing nothing fails here.
#[test]
fn the_prompt_and_the_background_close_it_for_good() {
    let mut page = Page::new(at_c4());
    page.copy();
    assert!(page.shown(), "control");
    page.view.close_peer_id_copied();
    page.settle();
    assert!(!page.shown(), "closed");
    page.show();
    elapse(7_000);
    assert!(!page.shown(), "not again on the return");
    assert_eq!(page.focus(), "identity-copy", "focus where it was");
}

/// The progress state shown in place of the screen's action closes it
/// (section 5). C4's own progress state is not built, so the view's rule
/// is held on a page given one. Mutation: `ShownOn::still` ignoring
/// `waiting` fails here.
#[test]
fn the_progress_state_closes_it() {
    let mut page = Page::new(at_c4());
    page.copy();
    assert!(page.shown(), "control");
    let mut busy = identity_page(&page.flow);
    busy.waiting = text(UiText::IdGoingOffline).into();
    page.view.show_identity(Some(busy));
    assert!(!page.shown(), "the progress state closes it");
}

/// Leaving the screen closes it by any way out -- R3's Back, R3's Yes
/// opening R4, R4's Continue once the next screen shows -- and the next
/// screen shows none (section 5). Mutation: `ShownOn::still` ignoring
/// the screen fails the first two; `show_identity(None)` not telling
/// `Copied` fails the third.
#[test]
fn leaving_closes_it_and_the_next_screen_shows_none() {
    let mut page = Page::new(at_r3());
    page.copy();
    assert!(page.shown(), "control");
    page.key(slint::platform::Key::Escape);
    assert_eq!(page.flow.screen(), ScreenId::R2, "control: Back to R2");
    assert!(!page.shown(), "Back closes it");

    let mut page = Page::new(at_r3());
    page.copy();
    page.tab_to("identity-primary");
    page.enter();
    page.flow.restored(Ok(()));
    page.settle();
    assert_eq!(page.flow.screen(), ScreenId::R4, "control: R4");
    assert!(!page.shown(), "R4 shows none");

    let mut page = Page::new(at_r4());
    page.copy();
    page.tab_to("identity-primary");
    page.enter();
    assert!(page.flow.start_held(), "control: Continue holds R4");
    assert!(page.shown(), "Continue leaves it");
    page.flow.decided();
    page.settle();
    assert_eq!(page.flow.screen(), ScreenId::Done, "control: handed over");
    assert!(!page.shown(), "the next screen shows none");
    assert!(page.view.window().get_identity_copied().is_empty());
}

/// From Android 13 the platform confirms the copy and the app shows and
/// says nothing (section 5): the same Enter copies, and nothing shows.
/// Mutation: the feedback check in `wire_peer_id_copied` removed fails
/// here.
#[test]
fn with_the_platforms_confirmation_nothing_shows_or_is_said() {
    for (name, flow) in screens() {
        let mut page = Page::new(flow);
        page.view.set_copy_feedback(CopyFeedback::Platform);
        page.copy();
        assert_eq!(
            clipboard(&page.view).as_deref(),
            Some(mine().as_str()),
            "{name}: copied"
        );
        assert_eq!(page.focus(), "identity-copy", "{name}: focus stays");
        assert!(!page.shown(), "{name}: nothing shows");
        assert_eq!(page.slots(), ["", ""], "{name}: nothing said");
    }
}

/// Where it sits (section 5): 12 px from the window's sides, 12 px above
/// the action, and the scrolling area ends 12 px above it, so it covers
/// nothing; Copy, at the area's end before, is scrolled just far enough
/// to stay whole in view; closed, the area ends where it did. On a short
/// window, R3's Copy just above the area's end. Mutation: `keep-in-view`
/// doing nothing leaves Copy under the area's end and fails here.
#[test]
fn it_sits_above_the_actions_and_keeps_copy_in_view() {
    let mut page = Page::sized(at_r3(), 360.0, 440.0);
    let flick = || "IdentityScreen::page-flick";
    let area_before = Edges::of(&page.element(flick()));
    let copy_before = Edges::of(&page.button(text(UiText::CopyPeerId)));
    assert!(
        copy_before.bottom <= area_before.bottom && copy_before.bottom > area_before.bottom - 60.0,
        "control: Copy at the area's end, {} of {}",
        copy_before.bottom,
        area_before.bottom
    );
    page.copy();
    assert!(page.shown(), "control");
    let snack = Edges::of(&page.element("IdentityScreen::copied-snack"));
    let area = Edges::of(&page.element(flick()));
    let yes = Edges::of(&page.button(text(UiText::IdR3Yes)));
    let copy = Edges::of(&page.button(text(UiText::CopyPeerId)));
    assert!(
        (snack.left - 12.0).abs() < 0.5,
        "12 from the left: {}",
        snack.left
    );
    assert!(
        (snack.right - 348.0).abs() < 0.5,
        "12 from the right: {}",
        snack.right
    );
    assert!(snack.bottom - snack.top >= 48.0, "at least 48 tall");
    assert!(
        (yes.top - snack.bottom - 12.0).abs() < 0.5,
        "12 above the action"
    );
    assert!(
        (snack.top - area.bottom - 12.0).abs() < 0.5,
        "the area ends 12 above it"
    );
    assert!(
        copy.top >= area.top && copy.bottom <= area.bottom + 0.5,
        "Copy whole in view: {}..{} in {}..{}",
        copy.top,
        copy.bottom,
        area.top,
        area.bottom
    );
    assert!(
        (copy.bottom - area.bottom).abs() < 0.5,
        "scrolled just far enough: {} to {}",
        copy.bottom,
        area.bottom
    );
    page.view.close_peer_id_copied();
    page.settle();
    let area_after = Edges::of(&page.element(flick()));
    assert!(
        (area_after.bottom - area_before.bottom).abs() < 0.5,
        "closed, the area ends where it did"
    );
}

/// C4's stops (section 8): its title, which has focus when C4 opens and
/// on which Enter presses nothing, then Copy, then the primary, and with
/// a notice raised the notice after the title; under `IdAlreadyCreated`
/// the title, the notice, then Continue, with no Copy. The order changed
/// with the title's stop (it was Copy, then Continue). Mutation: `page`
/// setting no Copy on C4, or no `heading_key`, fails here.
#[test]
fn c4s_focus_order_carries_copy_before_its_primary() {
    let mut page = Page::new(at_c4());
    assert_eq!(page.focus(), "c4-heading", "C4 opens on its title");
    page.enter();
    assert_eq!(page.take(), [], "Enter on the title presses nothing");
    assert_eq!(
        page.tabs(3),
        ["identity-copy", "identity-primary", "c4-heading"]
    );

    let mut flow = at_c4();
    apply_identity(&mut flow, IdentityInput::Primary);
    flow.sealed(Err(StepFailed::Storage));
    let mut page = Page::new(flow);
    page.tab_to("c4-heading");
    assert_eq!(
        page.tabs(3),
        ["identity-banner", "identity-copy", "identity-primary"]
    );

    let mut flow = at_c4();
    apply_identity(&mut flow, IdentityInput::Primary);
    flow.sealed(Err(StepFailed::AlreadyProvisioned));
    let mut page = Page::new(flow);
    page.tab_to("c4-heading");
    assert_eq!(
        page.tabs(3),
        ["identity-banner", "identity-primary", "c4-heading"]
    );
}

/// R3 with `IdRestoreOther` (sections 6 and 8): the notice under the app
/// bar and its Continue right after it, before the question, Copy, Yes and
/// No; the record's `PeerId` takes no stop. Continue pressed by Enter holds
/// R3: focus stays on it, a second Enter starts nothing, and Yes, No and
/// that Continue show disabled. Mutation: R3's notice action set to none
/// for `IdRestoreOther`, or the action left enabled while held, fails
/// here.
#[test]
fn r3s_restore_other_continue_follows_the_notice_and_holds_r3() {
    let recorded = TransportIdentity::parse("12D3KooW9xCm2jWjNVrwh51SWCQBMYdMyeU3NpT85QhLVkF6PcNM")
        .expect("a PeerId");
    let mut flow = at_r3();
    apply_identity(&mut flow, IdentityInput::Primary);
    flow.restored(Err(StepFailed::RestoreOther(recorded)));
    let mut page = Page::new(flow);
    page.tab_to("identity-back");
    assert_eq!(
        page.tabs(7),
        [
            "identity-banner",
            "identity-notice-action",
            "r3-question",
            "identity-copy",
            "identity-primary",
            "identity-secondary",
            "identity-back",
        ]
    );
    page.tab_to("identity-notice-action");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::NoticeAction]);
    assert!(page.flow.start_held(), "control: the start is held");
    assert_eq!(page.focus(), "identity-notice-action", "no focus moved");
    page.enter();
    assert_eq!(page.take(), [], "a second Enter presses nothing");
    let held = identity_page(&page.flow);
    assert!(held.notice_action_held && held.secondary_held && !held.primary_enabled);
}

/// What the app asked of the test's clipboard: the seam has no read of
/// the content, so this record is the whole of what the app asked.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Asked {
    Copy(String),
    /// A request for the primary clip's description.
    Describe,
    Clear,
}

/// The test's system clipboard: what it holds, with the mark in its
/// description when the app made the copy, and every ask of the app.
#[derive(Default)]
struct Board {
    asked: RefCell<Vec<Asked>>,
    clip: RefCell<Option<(String, Option<u64>)>>,
    marks: std::cell::Cell<u64>,
}

impl Board {
    /// Another app, or the platform's own copy, puts `text` there: no
    /// mark of the app's.
    fn other_copy(&self, text: &str) {
        *self.clip.borrow_mut() = Some((text.to_owned(), None));
    }

    /// What it holds, read by the test, never by the app.
    fn holds(&self) -> Option<String> {
        self.clip.borrow().as_ref().map(|(text, _)| text.clone())
    }

    fn asked(&self) -> Vec<Asked> {
        self.asked.borrow().clone()
    }

    fn describes(&self) -> usize {
        self.asked
            .borrow()
            .iter()
            .filter(|a| **a == Asked::Describe)
            .count()
    }
}

impl PeerIdClipboard for Board {
    fn copy(&self, peer_id: &str) -> ClipMark {
        self.asked
            .borrow_mut()
            .push(Asked::Copy(peer_id.to_owned()));
        let mark = self.marks.get() + 1;
        self.marks.set(mark);
        *self.clip.borrow_mut() = Some((peer_id.to_owned(), Some(mark)));
        ClipMark(mark)
    }

    fn remove_if_marked(&self, mark: ClipMark) {
        self.asked.borrow_mut().push(Asked::Describe);
        let ours = self
            .clip
            .borrow()
            .as_ref()
            .is_some_and(|(_, m)| *m == Some(mark.0));
        if ours {
            self.asked.borrow_mut().push(Asked::Clear);
            self.clip.borrow_mut().take();
        }
    }
}

/// C4 with the test's clipboard installed, below Android 13 or at 13 and
/// later (who confirms the copy).
fn c4_with_board(feedback: CopyFeedback) -> (Page, Rc<Board>) {
    let board = Rc::new(Board::default());
    let mut page = Page::new(at_c4());
    page.view.set_copy_feedback(feedback);
    page.view.set_peer_id_clipboard(Rc::clone(&board) as Rc<_>);
    (page, board)
}

/// Continue on C4, and the seal answered `AlreadyProvisioned`.
fn already_created(page: &mut Page) {
    page.tab_to("identity-primary");
    page.enter();
    assert!(page.flow.start_held(), "control: the seal runs");
    page.flow.sealed(Err(StepFailed::AlreadyProvisioned));
    page.settle();
    assert_eq!(
        identity_page(&page.flow).notice,
        text(UiText::IdAlreadyCreated),
        "control: IdAlreadyCreated shows"
    );
}

const FEEDBACKS: [CopyFeedback; 2] = [CopyFeedback::Snackbar, CopyFeedback::Platform];

/// Acceptance 18's first clipboard run, below 13 and at 13 or later: a
/// copy on C4 goes through the app's clipboard, and when the seal is
/// answered `AlreadyProvisioned` the clipboard no longer holds the
/// `PeerId`; the app asked for the description, never the content, and
/// the confirmation is as without the app's clipboard. Shown again, the
/// page owes nothing more. Mutation: `show_identity` not telling
/// `PeerClip` of the void `PeerId`, or `void_shown` comparing against
/// another string, leaves the `PeerId` there and fails here; the
/// app.slint branch left calling the window's field puts no copy on the
/// test's clipboard and fails at "copied".
#[test]
fn already_created_takes_the_copy_off_the_clipboard() {
    for feedback in FEEDBACKS {
        let (mut page, board) = c4_with_board(feedback);
        page.copy();
        assert_eq!(
            board.holds().as_deref(),
            Some(mine().as_str()),
            "{feedback:?}: copied"
        );
        assert_eq!(
            page.shown(),
            feedback == CopyFeedback::Snackbar,
            "{feedback:?}: confirmed as before"
        );
        assert_eq!(page.focus(), "identity-copy", "{feedback:?}: focus stays");
        already_created(&mut page);
        assert_eq!(board.holds(), None, "{feedback:?}: taken off");
        assert_eq!(
            board.asked(),
            [
                Asked::Copy(mine().as_str().to_owned()),
                Asked::Describe,
                Asked::Clear
            ],
            "{feedback:?}: the description asked, no content read"
        );
        page.show();
        page.settle();
        assert_eq!(board.describes(), 1, "{feedback:?}: owed once");
    }
}

/// Acceptance 18's second clipboard run, as far as it can be built: C2's
/// `PeerId` page is not built, so its copy is stood in for by the press's
/// callback while C4 shows, with nothing copied by C4's own Copy -- the
/// removal counts a copy of that `PeerId` whichever screen made it. That
/// callback is the one C4's Copy reaches too, so until C2's page exists
/// this run proves nothing beyond the first run's
/// (`already_created_takes_the_copy_off_the_clipboard`); run 2 as written
/// waits on C2's page.
#[test]
fn a_copy_of_that_peer_id_from_another_screen_counts() {
    for feedback in FEEDBACKS {
        let (mut page, board) = c4_with_board(feedback);
        page.view
            .window()
            .invoke_identity_peer_copy(mine().as_str().into());
        assert_eq!(board.holds().as_deref(), Some(mine().as_str()), "control");
        already_created(&mut page);
        assert_eq!(board.holds(), None, "{feedback:?}: taken off");
    }
}

/// Acceptance 18's third clipboard run: another text copied after the
/// app's copy and before the answer stays; the app asked the description
/// and cleared nothing. The fake clears only on its own mark, so the
/// view's half is asking with its last copy's mark (the first run fails if
/// it asks with any other) and never clearing unasked.
#[test]
fn what_was_copied_since_stays() {
    for feedback in FEEDBACKS {
        let (mut page, board) = c4_with_board(feedback);
        page.copy();
        board.other_copy("something else");
        already_created(&mut page);
        assert_eq!(
            board.holds().as_deref(),
            Some("something else"),
            "{feedback:?}: stays"
        );
        assert_eq!(
            board.asked(),
            [Asked::Copy(mine().as_str().to_owned()), Asked::Describe],
            "{feedback:?}: nothing cleared, no content read"
        );
    }
}

/// Acceptance 18's fourth and fifth clipboard runs: with C4's window
/// unfocused in the foreground, the answer asks nothing and the `PeerId`
/// stays until the window has the focus again; and with the device
/// reported locked when the focus comes back, it waits for the unlock.
/// No description is asked while either holds. Mutation: `PeerClip::ask`
/// not checking `unfocused`, or not checking `locked`, asks at once and
/// fails the matching run.
#[test]
fn the_removal_waits_for_the_focus_and_the_unlock() {
    for feedback in FEEDBACKS {
        let (mut page, board) = c4_with_board(feedback);
        page.copy();
        page.view.set_clipboard_focus(false);
        already_created(&mut page);
        assert_eq!(board.holds().as_deref(), Some(mine().as_str()), "waits");
        assert_eq!(board.describes(), 0, "{feedback:?}: nothing asked");
        page.view.set_clipboard_focus(true);
        assert_eq!(board.holds(), None, "{feedback:?}: focused, taken off");

        let (mut page, board) = c4_with_board(feedback);
        page.copy();
        page.view.set_clipboard_focus(false);
        already_created(&mut page);
        page.view.set_device_locked(true);
        page.view.set_clipboard_focus(true);
        assert_eq!(board.holds().as_deref(), Some(mine().as_str()), "locked");
        assert_eq!(board.describes(), 0, "{feedback:?}: nothing asked");
        page.view.set_device_locked(false);
        assert_eq!(board.holds(), None, "{feedback:?}: unlocked, taken off");
        assert_eq!(board.describes(), 1, "{feedback:?}: asked once");
    }
}

/// A removal the platform put off (the window found unfocused or the
/// device locked when it was asked, whatever was last reported) is asked
/// again at the next report that both hold, and not before. Mutation:
/// `removal_deferred` doing nothing leaves the `PeerId` and fails here.
#[test]
fn a_removal_the_platform_put_off_is_asked_again() {
    let (mut page, board) = c4_with_board(CopyFeedback::Snackbar);
    page.copy();
    page.view.set_clipboard_focus(false);
    already_created(&mut page);
    page.view.peer_id_removal_deferred(ClipMark(1));
    assert_eq!(board.describes(), 0, "control: nothing asked yet");
    page.view.set_clipboard_focus(true);
    assert_eq!(board.describes(), 1, "control: asked");
    // The platform answered that it could not: put the clip back as it
    // was, as an unanswered ask leaves it.
    *board.clip.borrow_mut() = Some((mine().as_str().to_owned(), Some(1)));
    page.view.peer_id_removal_deferred(ClipMark(1));
    assert_eq!(board.describes(), 1, "not again before a report");
    page.view.set_clipboard_focus(true);
    assert_eq!(board.describes(), 2, "asked again");
    assert_eq!(board.holds(), None, "taken off");
}

/// Nothing outlives the view (section 12): a removal still waiting when
/// the process ends is dropped, so a new view that shows the same page
/// asks nothing. Mutation: `PeerClip` keeping its last copy or what it
/// owes in a static survives the first view and fails here.
#[test]
fn a_removal_waiting_at_the_end_is_dropped() {
    let (mut page, board) = c4_with_board(CopyFeedback::Snackbar);
    page.copy();
    page.view.set_clipboard_focus(false);
    already_created(&mut page);
    let flow = page.flow;
    drop(page.view);

    let mut next = Page::new(flow);
    next.view.set_peer_id_clipboard(Rc::clone(&board) as Rc<_>);
    next.show();
    next.view.set_clipboard_focus(true);
    next.settle();
    assert_eq!(board.holds().as_deref(), Some(mine().as_str()), "stays");
    assert_eq!(board.describes(), 0, "nothing asked");
}

/// While the progress state shows, `CopyPeerId` copies nothing by Enter,
/// Space, a screen reader's activate or a tap, shows no snackbar, and is
/// drawn disabled, keeping its stop (section 5). C4's own progress state
/// is not built, so the view's rule is held on a page given one.
/// Mutation: Copy's `enabled-action` left true copies at the first press
/// and fails here.
#[test]
fn copy_copies_nothing_under_the_progress_state() {
    let board = Rc::new(Board::default());
    let mut page = Page::new(at_c4());
    page.view.set_peer_id_clipboard(Rc::clone(&board) as Rc<_>);
    page.tab_to("identity-copy");
    let mut busy = identity_page(&page.flow);
    busy.waiting = text(UiText::IdGoingOffline).into();
    page.view.show_identity(Some(busy.clone()));
    let _ = ElementHandle::find_by_accessible_label(page.view.window(), "q").count();
    assert_eq!(
        page.focus(),
        "identity-copy",
        "control: Copy keeps its stop"
    );
    let copy = page.button(text(UiText::CopyPeerId));
    assert_eq!(copy.accessible_enabled(), Some(false), "drawn disabled");
    for press in ["enter", "space", "activate", "tap"] {
        match press {
            "enter" => dispatch(&page, slint::platform::Key::Return.into()),
            "space" => dispatch(&page, " ".into()),
            "activate" => copy.invoke_accessible_default_action(),
            _ => copy.mock_single_click(slint::platform::PointerEventButton::Left),
        }
        page.view.show_identity(Some(busy.clone()));
        elapse(20);
        assert_eq!(board.asked(), [], "{press}: nothing copied");
        assert!(!page.shown(), "{press}: no snackbar");
        assert_eq!(page.slots(), ["", ""], "{press}: nothing said");
    }
    page.view.show_identity(Some(identity_page(&page.flow)));
    let _ = ElementHandle::find_by_accessible_label(page.view.window(), "q").count();
    assert_eq!(
        page.button(text(UiText::CopyPeerId)).accessible_enabled(),
        Some(true),
        "control: enabled without it"
    );
}

/// A key pressed and released on the window, with no settle: the page is
/// shown by the test as it wants it.
fn dispatch(page: &Page, text: slint::SharedString) {
    use slint::platform::WindowEvent;
    let window = page.view.window().window();
    window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text });
}

/// The page with `IdRestoreOther` raised on R3, the record naming another
/// `PeerId`.
fn at_r3_restore_other() -> IdentityFlow {
    let recorded = TransportIdentity::parse("12D3KooW9xCm2jWjNVrwh51SWCQBMYdMyeU3NpT85QhLVkF6PcNM")
        .expect("a PeerId");
    let mut flow = at_r3();
    apply_identity(&mut flow, IdentityInput::Primary);
    flow.restored(Err(StepFailed::RestoreOther(recorded)));
    flow
}

/// C4 with `IdStorageFailed` raised by its seal.
fn at_c4_storage_failed() -> IdentityFlow {
    let mut flow = at_c4();
    apply_identity(&mut flow, IdentityInput::Primary);
    flow.sealed(Err(StepFailed::Storage));
    flow
}

/// Let time pass from `*now` to `at`, both counted from the snackbar's
/// opening, and move `*now` there.
fn until(now: &mut u64, at: u64) {
    assert!(at >= *now, "time runs forward: {at} from {}", *now);
    elapse(at - *now);
    *now = at;
}

impl Page {
    /// Enter on the focused control at `*now`, its settle counted.
    fn enter_at(&mut self, now: &mut u64) {
        self.enter();
        *now += 20;
    }

    /// The root's held-start line (add-a-peer section 5.1), which the root
    /// shows once the screen has been held 1 s: the test stands in for its
    /// timer.
    fn starting_shows(&mut self, now: &mut u64) {
        self.flow.set_starting(true);
        self.settle();
        *now += 20;
        assert!(
            !identity_page(&self.flow).starting.is_empty(),
            "control: ConnectStarting shows"
        );
    }
}

/// Acceptance 18 on R3: Enter on `IdR3Yes` 3 s after the snackbar opened,
/// the restore unanswered, leaves it, and its time goes on: shown 2 s
/// after the Enter, gone 4 s after it. Mutation: `ShownOn::still`
/// comparing the primary's being enabled closes it at the Enter and fails
/// here.
#[test]
fn r3s_yes_leaves_it_and_its_time_goes_on() {
    let mut page = Page::new(at_r3());
    page.copy();
    let mut now = 20;
    page.tab_to("identity-primary");
    now += 20;
    until(&mut now, 3_000);
    page.enter_at(&mut now);
    assert!(page.flow.waiting(), "control: the restore runs, unanswered");
    until(&mut now, 5_000);
    assert!(page.shown(), "2 s after the Enter");
    until(&mut now, 7_000);
    assert!(!page.shown(), "4 s after the Enter");
}

/// A screen's name, how to reach it, and the control that holds it.
type HeldScreen = (&'static str, fn() -> IdentityFlow, &'static str);

/// The screens whose primary holds the screen for a start, and the control
/// pressed: C4's Continue (its seal unanswered), R4's Continue (its start
/// unanswered), and R3's `IdRestoreOther` Continue inside the notice.
fn held_screens() -> [HeldScreen; 3] {
    [
        ("C4", at_c4, "identity-primary"),
        ("R4", at_r4, "identity-primary"),
        (
            "R3 IdRestoreOther",
            at_r3_restore_other,
            "identity-notice-action",
        ),
    ]
}

/// Acceptance 18's three runs beside `ConnectStarting`, on C4, on R4 and,
/// the second and third, on R3 after `IdRestoreOther`'s Continue (the
/// first applies to C4 and R4 only). First: Continue 5.5 s after the
/// snackbar opened -- shown 0.2 s after, gone 0.7 s after, closed by its
/// time before `ConnectStarting`. Second: Continue at 2 s -- it closes
/// when `ConnectStarting` shows 1 s later, and the one announcement then
/// is `ConnectStarting`'s. Third: nothing copied before Continue, a copy
/// 1.5 s after it, while `ConnectStarting` shows -- the snackbar shows,
/// `ConnectStarting` stays, and the one announcement is the snackbar's.
/// Mutation: `ShownOn::still` ignoring `starting` keeps it past the line
/// and fails the second run; counting any line shown as one that came,
/// rather than one that was not there at the copy, closes a copy made
/// under it and fails the third; `still` comparing the primary's being
/// enabled closes it at the press and fails the first.
#[test]
fn beside_connect_starting() {
    let said = text(UiText::PeerIdCopied);
    let starting = text(UiText::ConnectStarting);
    for (name, flow, pressed) in held_screens() {
        if !name.starts_with("R3") {
            let mut page = Page::new(flow());
            page.copy();
            let mut now = 20;
            page.tab_to(pressed);
            now += 20;
            until(&mut now, 5_500);
            page.enter_at(&mut now);
            assert!(page.flow.start_held(), "{name}: control, held");
            until(&mut now, 5_700);
            assert!(page.shown(), "{name} 1: 0.2 s after Continue");
            until(&mut now, 6_200);
            assert!(!page.shown(), "{name} 1: 0.7 s after Continue");
            until(&mut now, 6_500);
            page.starting_shows(&mut now);
            assert!(!page.shown(), "{name} 1: not back with the line");
        }

        let mut page = Page::new(flow());
        page.copy();
        let mut now = 20;
        page.tab_to(pressed);
        now += 20;
        until(&mut now, 2_000);
        page.enter_at(&mut now);
        assert!(page.flow.start_held(), "{name}: control, held");
        assert!(page.shown(), "{name} 2: the press leaves it");
        until(&mut now, 3_000);
        page.starting_shows(&mut now);
        assert!(!page.shown(), "{name} 2: closed by the line");
        assert_eq!(
            page.slots().iter().filter(|s| *s == starting).count(),
            1,
            "{name} 2: the line said"
        );
        assert!(
            !page.slots().iter().any(|s| s == said),
            "{name} 2: the snackbar not said again"
        );

        let mut page = Page::new(flow());
        page.tab_to(pressed);
        let mut now = 0;
        page.enter_at(&mut now);
        assert!(page.flow.start_held(), "{name}: control, held");
        until(&mut now, 1_000);
        page.starting_shows(&mut now);
        until(&mut now, 1_500);
        page.copy();
        assert!(page.shown(), "{name} 3: shown under the line");
        assert!(
            !identity_page(&page.flow).starting.is_empty(),
            "{name} 3: the line stays"
        );
        assert_eq!(
            page.slots().iter().filter(|s| *s == said).count(),
            1,
            "{name} 3: the snackbar said"
        );
        assert!(
            !page.slots().iter().any(|s| s == starting),
            "{name} 3: the line not said again"
        );
    }
}

/// Acceptance 18 on C4 with `IdStorageFailed`: a copy shows the snackbar
/// and the notice stays; Try again 5.5 s after, the seal unanswered, leaves
/// it -- shown 0.2 s after, gone 0.7 s after. In a second run, Try again 1 s
/// after the copy, answered `IdStorageFailed` again 1 s later: the same
/// notice is the result, so it closes the snackbar when it shows, and focus
/// goes to the notice (sections 5 and 8). Mutation: `ShownOn::still`
/// ignoring `notices_raised` keeps it past the second result and fails
/// here.
#[test]
fn beside_a_notice_and_its_try_again() {
    let mut page = Page::new(at_c4_storage_failed());
    page.copy();
    let mut now = 20;
    assert!(page.shown(), "shown over the notice");
    assert_eq!(
        identity_page(&page.flow).notice,
        text(UiText::IdStorageFailed),
        "the notice stays"
    );
    page.tab_to("identity-primary");
    now += 20;
    until(&mut now, 5_500);
    page.enter_at(&mut now);
    assert!(page.flow.start_held(), "control: the seal runs again");
    until(&mut now, 5_700);
    assert!(page.shown(), "0.2 s after Try again");
    until(&mut now, 6_200);
    assert!(!page.shown(), "0.7 s after Try again");

    let mut page = Page::new(at_c4_storage_failed());
    page.copy();
    let mut now = 20;
    page.tab_to("identity-primary");
    now += 20;
    until(&mut now, 1_000);
    page.enter_at(&mut now);
    until(&mut now, 2_000);
    assert!(page.shown(), "control: up until the result");
    page.flow.sealed(Err(StepFailed::Storage));
    page.settle();
    assert!(!page.shown(), "the same notice, a result, closes it");
    assert_eq!(page.focus(), "identity-banner", "focus to the notice");
}

/// A rotation 3 s after it opened (the window's size swapped) keeps it
/// shown, 12 px above the actions, with no second announcement, and it
/// closes 6 s after it opened (section 5). Mutation: `Copied::page_shown`
/// closing it at every page shown, as the resize's settle shows one, fails
/// here.
#[test]
fn a_rotation_keeps_it_and_its_time() {
    let mut page = Page::sized(at_c4(), 360.0, 640.0);
    page.copy();
    let mut now = 20;
    let said = page.slots();
    until(&mut now, 3_000);
    page.view
        .window()
        .window()
        .set_size(slint::LogicalSize::new(640.0, 360.0));
    page.settle();
    now += 20;
    assert!(page.shown(), "kept");
    assert_eq!(page.slots(), said, "not said again");
    let snack = Edges::of(&page.element("IdentityScreen::copied-snack"));
    let primary = Edges::of(&page.button(text(UiText::IdContinue)));
    assert!(
        (primary.top - snack.bottom - 12.0).abs() < 0.5,
        "12 above the actions: {} and {}",
        snack.bottom,
        primary.top
    );
    until(&mut now, 5_900);
    assert!(page.shown(), "up until its time");
    until(&mut now, 6_100);
    assert!(!page.shown(), "closed 6 s after it opened");
}

/// On C4 and R4 as on R3: before a copy the scrolling area ends where it
/// ends; while the snackbar shows, 12 px above it; once it closes, where
/// it ended before (section 5): the R3 test's rule, on the other two
/// screens.
#[test]
fn the_area_ends_above_it_and_where_it_was_after() {
    for (name, flow) in [("C4", at_c4()), ("R4", at_r4())] {
        let mut page = Page::sized(flow, 360.0, 640.0);
        let flick = "IdentityScreen::page-flick";
        let before = Edges::of(&page.element(flick));
        page.copy();
        assert!(page.shown(), "{name}: control");
        let snack = Edges::of(&page.element("IdentityScreen::copied-snack"));
        let area = Edges::of(&page.element(flick));
        assert!(
            (snack.top - area.bottom - 12.0).abs() < 0.5,
            "{name}: the area ends 12 above it: {} and {}",
            area.bottom,
            snack.top
        );
        page.view.close_peer_id_copied();
        page.settle();
        let after = Edges::of(&page.element(flick));
        assert!(
            (after.bottom - before.bottom).abs() < 0.5,
            "{name}: closed, the area ends where it did"
        );
    }
}
