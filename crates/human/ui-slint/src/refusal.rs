// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The desktop's start-up refusal screen (plan §18 clause (4), decided
//! 2026-10-20): why the app did not start, the file it is about, Close
//! always and Try again where trying again can change the answer. The
//! root decides which refusal it is and what Try again does; this only
//! shows it, and names the reason by its key, never by its English.

use std::cell::Cell;
use std::rc::Rc;

use interweave_human_ui_model::{StoreRecovery, UiText, placeholder_en};
use slint::{ComponentHandle as _, SharedString};

use crate::code_font;
use crate::generated::RefusalWindow;

/// What the screen says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusalPage {
    /// Why, as a person can act on it.
    pub reason: UiText,
    /// The file or folder it is about, whole; none when the refusal
    /// names none.
    pub path: Option<String>,
    /// Whether Try again is offered: only where the root can run the
    /// refused step again and get another answer without a relaunch.
    pub can_retry: bool,
    /// Whether Delete stored data… is offered: the store's recovery
    /// screen alone (ADR-0060 decision 6), and only where the root can
    /// run the discard.
    pub can_discard: bool,
}

/// The refusal screen's window.
pub struct RefusalView {
    window: RefusalWindow,
    next_slot_b: Rc<Cell<bool>>,
}

impl RefusalView {
    /// A refusal screen showing `page`. Shows nothing until
    /// [`run`](Self::run). Close, Escape and the window's own close all
    /// end [`run`](Self::run).
    ///
    /// # Errors
    /// The toolkit's, when no platform can create a window.
    pub fn new(page: &RefusalPage) -> Result<Self, slint::PlatformError> {
        let window = RefusalWindow::new()?;
        let text = |t| SharedString::from(placeholder_en::text(t));
        window.set_window_title(text(UiText::AppTitle));
        window.set_heading(text(UiText::RefusalHeading));
        window.set_path_label(text(UiText::RefusalPath));
        window.set_close_label(text(UiText::Close));
        window.set_retry_label(text(UiText::TryAgain));
        window.set_discard_label(text(UiText::DiscardStore));
        window.set_cancel_label(text(UiText::Cancel));
        window.set_code_font(code_font());
        let weak = window.as_weak();
        window.on_close_screen(move || {
            if let Some(window) = weak.upgrade() {
                // Hiding the last window ends the event loop.
                let _ = window.hide();
            }
        });
        let view = Self {
            window,
            next_slot_b: Rc::new(Cell::new(false)),
        };
        view.show_page(page);
        Ok(view)
    }

    /// Show `page` in place of what is shown, and move focus to its
    /// primary action. The status line is cleared: it spoke of the page
    /// before.
    pub fn show_page(&self, page: &RefusalPage) {
        let window = &self.window;
        window.set_reason(placeholder_en::text(page.reason).into());
        window.set_path(page.path.clone().unwrap_or_default().into());
        window.set_can_retry(page.can_retry);
        window.set_can_discard(page.can_discard);
        window.set_status_a(SharedString::new());
        window.set_status_b(SharedString::new());
        window.invoke_focus_primary();
    }

    /// Say, in the screen's live region, that Try again ran and the
    /// answer is still a refusal -- or a press that changed nothing on
    /// screen would look like a press that did nothing. Said in the slot
    /// the last one was not, so a second time is still heard.
    pub fn tried_again(&self) {
        let said = SharedString::from(placeholder_en::text(UiText::RefusalStillRefused));
        let b = self.next_slot_b.get();
        if b {
            self.window.set_status_a(SharedString::new());
            self.window.set_status_b(said);
        } else {
            self.window.set_status_b(SharedString::new());
            self.window.set_status_a(said);
        }
        self.next_slot_b.set(!b);
    }

    /// Show where the store's Discard and recreate is (ADR-0060 decision
    /// 6): the actions inert while the plan is built or the discard runs,
    /// the line under the heading, and the confirmation -- whose opening
    /// moves focus to its Cancel.
    pub fn show_store_recovery(&self, flow: &StoreRecovery) {
        let window = &self.window;
        let text = |t| SharedString::from(placeholder_en::text(t));
        window.set_busy(!flow.actions_live() && flow.dialog().is_none());
        window.set_progress(flow.status().map(text).unwrap_or_default());
        let was_open = window.get_dialog_open();
        if let Some(dialog) = flow.dialog() {
            window.set_dialog_title(text(dialog.title));
            window.set_dialog_lead(dialog.lead.map(text).unwrap_or_default());
            let items: Vec<SharedString> = dialog.items.iter().map(|t| text(*t)).collect();
            window.set_dialog_items(slint::ModelRc::new(slint::VecModel::from(items)));
            let mut notes = dialog.notes.iter().map(|t| text(*t));
            window.set_dialog_note_a(notes.next().unwrap_or_default());
            window.set_dialog_note_b(notes.next().unwrap_or_default());
            window.set_dialog_confirm_label(text(dialog.confirm));
            window.set_dialog_danger(dialog.destructive);
            window.set_dialog_open(true);
            if !was_open {
                window.invoke_focus_dialog();
            }
        } else {
            window.set_dialog_open(false);
            if was_open {
                // Back where the confirmation was asked from.
                window.invoke_focus_primary();
            }
        }
    }

    /// Call `discard` when the person presses Delete stored data….
    pub fn on_discard(&self, discard: impl Fn() + 'static) {
        self.window.on_discard(discard);
    }

    /// Call `cancel` on the confirmation's Cancel, Escape or the scrim.
    pub fn on_dialog_cancel(&self, cancel: impl Fn() + 'static) {
        self.window.on_dialog_cancel(cancel);
    }

    /// Call `confirm` on the confirmation's primary action.
    pub fn on_dialog_confirm(&self, confirm: impl Fn() + 'static) {
        self.window.on_dialog_confirm(confirm);
    }

    /// Call `retry` when the person presses Try again.
    pub fn on_retry(&self, retry: impl Fn() + 'static) {
        self.window.on_retry(retry);
    }

    /// Close the screen: [`run`](Self::run) returns.
    pub fn close(&self) {
        self.window.invoke_close_screen();
    }

    /// Show the screen and run the event loop until it is closed.
    ///
    /// # Errors
    /// The toolkit's, as text: no window could be shown.
    pub fn run(&self) -> Result<(), String> {
        self.window.show().map_err(|e| e.to_string())?;
        self.window.invoke_focus_primary();
        slint::run_event_loop().map_err(|e| e.to_string())?;
        let _ = self.window.hide();
        Ok(())
    }

    /// The window, for tests that read its tree.
    #[must_use]
    pub const fn window(&self) -> &RefusalWindow {
        &self.window
    }
}
