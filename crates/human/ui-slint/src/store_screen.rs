// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Android's store screens -- R, recovery mode, and K, the key the
//! platform invalidated -- on the identity screens' frame
//! (design/2026-10-21-contact-cards.md §7.1): ui-model's
//! [`StoreRecovery`] becomes one [`IdentityPage`], and a tap on it one
//! flow call. The root carries out what comes back; nothing here plans,
//! discards or starts anything.

use interweave_human_ui_model::{StoreEffect, StoreRecovery, StoreScreen, UiText, placeholder_en};
use slint::{ModelRc, SharedString, VecModel};

use crate::IdentityInput;
use crate::generated::IdentityPage;

/// What a tap on a store screen comes to, for the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreOutcome {
    /// Nothing for the root: the page changed, or the tap was inert.
    Shown,
    /// Build the plan, or run the confirmed discard.
    Effect(StoreEffect),
    /// R's Try again: start the service again.
    TryAgain,
    /// Close: leave the app's task.
    Close,
}

/// The page `flow` shows: R with Try again (filled), Delete stored data…
/// (outlined, in `error`) and Close; K with Delete stored data… as its
/// primary, in `error`, and Close; the line under way in place of the
/// primary; a failure as the page's banner; the confirmation as its
/// dialog, Cancel the initial focus.
#[must_use]
pub fn store_page(flow: &StoreRecovery) -> IdentityPage {
    let text = |t: UiText| SharedString::from(placeholder_en::text(t));
    let failed = flow.status() == Some(UiText::StoreDiscardFailed);
    let waiting = if failed {
        SharedString::new()
    } else {
        flow.status().map(text).unwrap_or_default()
    };
    let mut page = IdentityPage {
        title: text(flow.heading()),
        paragraphs: ModelRc::new(VecModel::from(vec![text(flow.body())])),
        notice: if failed {
            text(UiText::StoreDiscardFailed)
        } else {
            SharedString::new()
        },
        waiting,
        primary_enabled: true,
        ..IdentityPage::default()
    };
    match flow.screen() {
        StoreScreen::Recovery => {
            page.primary = text(UiText::TryAgain);
            page.secondary = text(UiText::DiscardStore);
            page.secondary_danger = true;
            page.tertiary = text(UiText::Close);
        }
        StoreScreen::KeyLost => {
            page.primary = text(UiText::DiscardStore);
            page.primary_danger = true;
            page.secondary = text(UiText::Close);
        }
    }
    if let Some(dialog) = flow.dialog() {
        page.dialog = true;
        page.dialog_title = text(dialog.title);
        page.dialog_lead = dialog.lead.map(text).unwrap_or_default();
        let items: Vec<SharedString> = dialog.items.iter().map(|t| text(*t)).collect();
        page.dialog_items = ModelRc::new(VecModel::from(items));
        let mut notes = dialog.notes.iter().map(|t| text(*t));
        page.dialog_note_a = notes.next().unwrap_or_default();
        page.dialog_note_b = notes.next().unwrap_or_default();
        page.dialog_delete = text(dialog.confirm);
        page.dialog_keep = text(UiText::Cancel);
        page.dialog_danger = dialog.destructive;
    }
    page
}

/// Apply a tap on the page `flow` shows.
pub fn apply_store(flow: &mut StoreRecovery, input: &IdentityInput) -> StoreOutcome {
    let live = flow.actions_live();
    match (flow.screen(), input) {
        (_, IdentityInput::Dialog(true)) => flow
            .confirm()
            .map_or(StoreOutcome::Shown, StoreOutcome::Effect),
        (_, IdentityInput::Dialog(false)) => {
            flow.cancel();
            StoreOutcome::Shown
        }
        (_, IdentityInput::Back) => {
            if flow.back() {
                StoreOutcome::Shown
            } else {
                StoreOutcome::Close
            }
        }
        (StoreScreen::Recovery, IdentityInput::Primary) if live => StoreOutcome::TryAgain,
        (StoreScreen::Recovery, IdentityInput::Secondary)
        | (StoreScreen::KeyLost, IdentityInput::Primary) => flow
            .press_discard()
            .map_or(StoreOutcome::Shown, StoreOutcome::Effect),
        (StoreScreen::Recovery, IdentityInput::Tertiary)
        | (StoreScreen::KeyLost, IdentityInput::Secondary)
            if live =>
        {
            StoreOutcome::Close
        }
        _ => StoreOutcome::Shown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interweave_human_ui_model::{DiscardKind, StorePhase};

    #[test]
    fn r_offers_try_again_then_delete_then_close_and_k_has_no_try_again() {
        let r = store_page(&StoreRecovery::new(StoreScreen::Recovery, 1));
        assert_eq!(r.primary, placeholder_en::text(UiText::TryAgain));
        assert_eq!(r.secondary, placeholder_en::text(UiText::DiscardStore));
        assert!(r.secondary_danger && !r.primary_danger);
        assert_eq!(r.tertiary, placeholder_en::text(UiText::Close));
        let k = store_page(&StoreRecovery::new(StoreScreen::KeyLost, 1));
        assert_eq!(k.title, placeholder_en::text(UiText::StoreKeyLostTitle));
        assert_eq!(k.primary, placeholder_en::text(UiText::DiscardStore));
        assert!(k.primary_danger);
        assert_eq!(k.secondary, placeholder_en::text(UiText::Close));
        assert!(k.tertiary.is_empty(), "no Try again on K");
    }

    #[test]
    fn a_tap_plans_the_dialog_confirms_and_back_cancels() {
        let mut flow = StoreRecovery::new(StoreScreen::Recovery, 1);
        assert_eq!(
            apply_store(&mut flow, &IdentityInput::Secondary),
            StoreOutcome::Effect(StoreEffect::Plan)
        );
        assert_eq!(
            apply_store(&mut flow, &IdentityInput::Primary),
            StoreOutcome::Shown,
            "Try again is inert while the plan is built"
        );
        flow.planned(DiscardKind::Full);
        let page = store_page(&flow);
        assert!(page.dialog && page.dialog_danger);
        assert_eq!(page.dialog_keep, placeholder_en::text(UiText::Cancel));
        assert_eq!(
            apply_store(&mut flow, &IdentityInput::Back),
            StoreOutcome::Shown
        );
        assert_eq!(flow.phase(), StorePhase::Shown);
        apply_store(&mut flow, &IdentityInput::Secondary);
        flow.planned(DiscardKind::Full);
        assert_eq!(
            apply_store(&mut flow, &IdentityInput::Dialog(true)),
            StoreOutcome::Effect(StoreEffect::Discard)
        );
        assert_eq!(
            apply_store(&mut flow, &IdentityInput::Back),
            StoreOutcome::Shown,
            "Back is ignored while the discard runs"
        );
        assert_eq!(
            store_page(&flow).waiting,
            placeholder_en::text(UiText::StoreDiscarding)
        );
    }

    #[test]
    fn back_on_the_screen_and_close_leave_and_a_failure_is_the_banner() {
        let mut flow = StoreRecovery::new(StoreScreen::KeyLost, 1);
        assert_eq!(
            apply_store(&mut flow, &IdentityInput::Back),
            StoreOutcome::Close
        );
        assert_eq!(
            apply_store(&mut flow, &IdentityInput::Secondary),
            StoreOutcome::Close
        );
        apply_store(&mut flow, &IdentityInput::Primary);
        flow.failed();
        let page = store_page(&flow);
        assert_eq!(
            page.notice,
            placeholder_en::text(UiText::StoreDiscardFailed)
        );
        assert!(page.waiting.is_empty());
    }
}
