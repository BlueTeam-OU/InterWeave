// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Copy `PeerId`'s confirmation below Android 13 on the identity screens
//! (design/2026-10-19-android-identity.md section 5, The copy's
//! confirmation; acceptance 18's host part): the `PeerIdCopied` snackbar
//! on C4, R3 and R4 -- its time, its one announcement, what closes it,
//! where it sits -- and nothing at all where the platform confirms the
//! copy itself. C2's and S2's `PeerId` pages are not built: those screens
//! show their words beside the `PeerId`, so they carry no Copy.

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
    CopyFeedback, IdentityInput, TouchSettings, View, apply_identity, identity_page, snackbar_ms,
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
/// on C4 `IdVoidPaper` (`IdAlreadyCreated` in the design), which hides Copy
/// too, and `IdBusy`; on R3 `IdStorageFailed`. A notice raised again with
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
