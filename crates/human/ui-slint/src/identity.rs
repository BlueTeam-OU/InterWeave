// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identity screens between ui-model's [`IdentityFlow`] and the view's
//! [`IdentityPage`] (design/2026-10-19-android-identity.md): a person's
//! tap becomes one flow call, and the flow's state becomes one page. The
//! root carries out the [`Effect`]s the flow returns; nothing here seals,
//! restores or reaches the network.

use interweave_human_ui_model::{
    Effect, IdentityFlow, Notice, PHRASE_WORDS, RecoveryReason, ScreenId, Suggestions, UiText,
    fill, placeholder_en,
};
use interweave_transport_api::TransportIdentity;
use slint::{ModelRc, SharedString, VecModel};

use crate::generated::{IdKey, IdKeyRow, IdScreen, IdSlot, IdentityPage};

/// K1's heading as the focus trace names it: a refused start that raises
/// K1 puts focus on it (section 8).
pub(crate) const K1_HEADING: &str = "k1-heading";
/// R3's question, which takes the stop a title would (R3 has no title),
/// and R4's title: a valid check, or R3's Yes, puts focus on what the
/// screen asks or on what was restored before the action that answers it
/// (design section 8, #697 review P-1).
pub(crate) const R3_QUESTION: &str = "r3-question";
pub(crate) const R4_HEADING: &str = "r4-heading";
/// C4's title, a stop that takes focus when C4 opens, so Continue, which
/// seals and starts the service, is reached by Tab and not by a second
/// Enter (design section 8).
pub(crate) const C4_HEADING: &str = "c4-heading";
/// The title of C2's and S2's `PeerId` page, a stop that takes focus when
/// the words page's primary opens it, so the reading starts at the top
/// (design section 8).
pub(crate) const PEER_ID_HEADING: &str = "peer-id-heading";

/// QWERTY rows of 10, 9 and 7 (section 4.3).
const QWERTY: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
/// Alphabetical rows of seven, for a screen narrower than 340.
const ALPHABET: [&str; 4] = ["abcdefg", "hijklmn", "opqrstu", "vwxyz"];

/// What the person did on an identity page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityInput {
    /// The page's primary action.
    Primary,
    /// Its secondary action.
    Secondary,
    /// Its third action (C3's "Show my words again").
    Tertiary,
    /// Back.
    Back,
    /// A keypad letter, or a hardware keyboard's.
    Key(char),
    /// Delete the last letter.
    Delete,
    /// Clear the position's letters.
    Clear,
    /// A suggestion chip.
    Chip(String),
    /// Enter while typing: the flow takes the chip the letters applied
    /// before it leave, if any (section 8).
    Enter,
    /// A position in R2's grid.
    Slot(usize),
    /// The discard dialog: delete the words (`true`) or continue.
    Dialog(bool),
    /// R2's words as a full list: "Show all my words" opens it (`true`);
    /// the text falling to 130 % or below closes it (section 4.4).
    Review(bool),
    /// The action under the text of R2's or R3's notice (section 6):
    /// Try again, or `IdRestoreOther`'s Continue.
    NoticeAction,
}

/// Apply `input` to `flow`; the effects are the root's to carry out.
pub fn apply(flow: &mut IdentityFlow, input: IdentityInput) -> Vec<Effect> {
    let screen = flow.screen();
    let failed = flow.notice().is_some_and(Notice::retries);
    match input {
        IdentityInput::Primary => flow.primary(),
        IdentityInput::Secondary => match screen {
            // On Welcome a refusal's Try again takes the secondary's place.
            ScreenId::W if !failed => flow.restore(),
            // R3's secondary is always "no, check my words": its notice's
            // Try again is the notice's own action (`NoticeAction`).
            ScreenId::R3 => {
                flow.recheck();
                Vec::new()
            }
            ScreenId::C3 => {
                flow.try_again();
                Vec::new()
            }
            // R2's Try again is its notice's own action, C4's and R4's
            // their primary (section 6). C4's secondary is
            // `IdStartUnchecked`, while the flow offers it (section 6, A
            // start from C4 after a failed read-back); R2 and R4 have none.
            ScreenId::C4 => flow.start_unchecked(),
            ScreenId::R2 | ScreenId::R4 => Vec::new(),
            // Section 6's "Try again" retries the step that failed.
            _ if failed => flow.primary(),
            _ => Vec::new(),
        },
        // Try again or Continue under R2's or R3's notice; the flow holds
        // the screen while what it starts runs, so a second press starts
        // nothing.
        IdentityInput::NoticeAction => flow.notice_action(),
        IdentityInput::Tertiary => {
            flow.show_words_again();
            Vec::new()
        }
        IdentityInput::Back => flow.back(),
        IdentityInput::Key(letter) => {
            flow.press(letter);
            Vec::new()
        }
        IdentityInput::Delete => {
            flow.delete();
            Vec::new()
        }
        IdentityInput::Clear => {
            flow.clear_letters();
            Vec::new()
        }
        IdentityInput::Chip(word) => flow.choose(&word),
        IdentityInput::Enter => flow.enter(),
        IdentityInput::Slot(position) => {
            flow.edit(position);
            Vec::new()
        }
        IdentityInput::Dialog(delete) => {
            flow.discard(delete);
            Vec::new()
        }
        IdentityInput::Review(open) => {
            flow.review_words(open);
            Vec::new()
        }
    }
}

/// Whether `page` carries ADR-0043's warning among its paragraphs, as
/// [`page`] words it: the trace's reading of what the window was handed.
#[cfg(feature = "ui-trace")]
pub(crate) fn warns_active_elsewhere(page: &IdentityPage) -> bool {
    use slint::Model as _;
    let warning = text(UiText::IdActiveElsewhere);
    page.paragraphs.iter().any(|p| p.as_str() == warning)
}

fn text(t: UiText) -> String {
    placeholder_en::text(t).to_owned()
}

fn numbered(t: UiText, n: usize) -> String {
    fill(placeholder_en::text(t), &[("n", &n.to_string())])
}

/// A `PeerId` in groups of four, for comparing by eye (section 5): the
/// spaces are display only and the text wraps between groups.
#[must_use]
pub fn grouped(peer: &TransportIdentity) -> String {
    let chars: Vec<char> = peer.as_str().chars().collect();
    chars
        .chunks(4)
        .map(|group| group.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

fn strings(items: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        items
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
}

fn rows(flow: &IdentityFlow, rows: &[&str]) -> ModelRc<IdKeyRow> {
    let picker = flow.picker();
    let enabled = |letter| picker.is_some_and(|p| p.enabled(letter));
    // After `IdRestoreOther`'s Continue every key shows disabled until the
    // next screen opens (section 8); the first enabled key stays the one
    // the letters say, for focus.
    let held = flow.start_held();
    // Where focus goes after a chip fills a position (section 8): the
    // first enabled key in this layout's reading order.
    let first = rows
        .iter()
        .flat_map(|row| row.chars())
        .find(|&l| enabled(l));
    let mut index = 0;
    ModelRc::new(VecModel::from(
        rows.iter()
            .map(|row| IdKeyRow {
                keys: ModelRc::new(VecModel::from(
                    row.chars()
                        .map(|letter| {
                            index += 1;
                            IdKey {
                                letter: letter.to_string().into(),
                                enabled: enabled(letter) && !held,
                                index: index - 1,
                                first: first == Some(letter),
                            }
                        })
                        .collect::<Vec<_>>(),
                )),
            })
            .collect::<Vec<_>>(),
    ))
}

fn screen_id(screen: ScreenId) -> IdScreen {
    match screen {
        ScreenId::W => IdScreen::W,
        ScreenId::C1 => IdScreen::C1,
        ScreenId::C2 => IdScreen::C2,
        ScreenId::C2PeerId => IdScreen::C2PeerId,
        ScreenId::C3 => IdScreen::C3,
        ScreenId::C4 => IdScreen::C4,
        ScreenId::R1 => IdScreen::R1,
        ScreenId::R2 => IdScreen::R2,
        ScreenId::R3 => IdScreen::R3,
        ScreenId::R4 => IdScreen::R4,
        ScreenId::K1 => IdScreen::K1,
        ScreenId::S1 => IdScreen::S1,
        ScreenId::S2 => IdScreen::S2,
        ScreenId::S2PeerId => IdScreen::S2PeerId,
        ScreenId::Done => IdScreen::Done,
    }
}

/// C4 or R4 held after Continue: Continue disabled at once, and the
/// root's 1 s line under it (add-a-peer section 5.1).
fn held_start(flow: &IdentityFlow, page: &mut IdentityPage) {
    if flow.start_held() {
        page.primary_enabled = false;
    }
    if flow.starting() {
        page.starting = text(UiText::ConnectStarting).into();
    }
}

/// C4's `IdStartUnchecked`, a text button under Try again in C4's order,
/// and `IdStorageUnknownRestore`, text that takes no focus, under it, while
/// the flow offers it (section 6, A start from C4 after a failed
/// read-back; section 8). While C4 is held -- Try again's read-back, or
/// this control's own start -- it keeps its stop, shown disabled, as the
/// primary does, so a second press starts nothing.
fn c4_unchecked(flow: &IdentityFlow, page: &mut IdentityPage) {
    if flow.offers_unchecked() {
        page.secondary = text(UiText::IdStartUnchecked).into();
        page.secondary_held = flow.start_held();
        page.footnote = text(UiText::IdStorageUnknownRestore).into();
    }
}

/// R2's grid, its position line and its primary: enabled once all 24
/// are filled and no check runs, and while one runs every word is held
/// (section 8, `IdentityFlow::words_held`).
fn entry_grid(flow: &IdentityFlow, page: &mut IdentityPage) {
    let Some(entry) = flow.entry() else {
        return;
    };
    page.position = numbered(UiText::IdWordPosition, entry.current() + 1).into();
    page.grid = ModelRc::new(VecModel::from(
        (0..PHRASE_WORDS)
            .map(|i| IdSlot {
                number: i32::try_from(i + 1).unwrap_or(0),
                word: entry.word(i).unwrap_or_default().into(),
                current: i == entry.current(),
            })
            .collect::<Vec<_>>(),
    ));
    page.current = i32::try_from(entry.current()).unwrap_or(0);
    page.review_label = text(UiText::IdReviewWords).into();
    page.review = flow.reviewing();
    page.primary_enabled = entry.complete() && !flow.waiting();
    // On R2, waiting is the check running.
    page.checking = flow.waiting();
    page.notice_action_held = flow.waiting();
    // After `IdRestoreOther`'s Continue every held control shows disabled
    // (section 8): the start lasts up to the 10 s bound.
    page.held = flow.start_held();
}

/// R3's question, the line under it, and its secondary, "no, check my
/// words". While the restore runs Yes, No and the notice's action keep
/// their stops, shown disabled, and Back waits (section 8, While R3's
/// restore runs; `back_active` in [`page`]).
fn r3(flow: &IdentityFlow, page: &mut IdentityPage) {
    page.question = text(UiText::IdR3Question).into();
    page.question_key = R3_QUESTION.into();
    // Under the question, text that takes no focus (sections 3.3 and 8).
    page.question_note = text(UiText::IdR3NoPeerId).into();
    page.secondary = text(UiText::IdR3No).into();
    let restoring = flow.waiting();
    page.primary_enabled = !restoring;
    page.secondary_held = restoring;
    page.notice_action_held = restoring;
}

/// The notice of the flow's screen on `page` (section 6), its `PeerId` and
/// its action; true when the banner's `PeerIds` take the place of the
/// screen's own, which then has no Copy.
fn notice(flow: &IdentityFlow, screen: ScreenId, page: &mut IdentityPage) -> bool {
    let mut takes_peer = false;
    match flow.notice() {
        Some(Notice::ChecksumFailed) => page.notice = text(UiText::IdChecksumFailed).into(),
        Some(Notice::OtherIdentity { derived, expected }) => {
            page.notice = text(UiText::IdOtherIdentity).into();
            page.peer_label = text(UiText::IdDerivedPeerId).into();
            page.peer_groups = grouped(derived).into();
            // A banner's PeerIds are for the eye alone (section 5).
            takes_peer = true;
            page.second_peer_label = text(UiText::IdExpectedPeerId).into();
            page.second_peer_groups = grouped(expected).into();
        }
        Some(Notice::AlreadyCreated) => page.notice = text(UiText::IdAlreadyCreated).into(),
        Some(Notice::RestoreOther { recorded }) => {
            page.notice = text(UiText::IdRestoreOther).into();
            // In full under the notice, for the eye alone: no Copy, since
            // it is not the PeerId the person restored (section 5).
            page.notice_peer = grouped(recorded).into();
            page.notice_action = text(UiText::IdContinue).into();
        }
        // In the notice place `IdRestoreOther` left on R2 and R3, its
        // PeerId kept under it and Try again in its Continue's place; on
        // C4 and R4 Try again is the primary, in Continue's (section 6).
        Some(Notice::StartBusy { recorded, .. }) => {
            page.notice = text(UiText::IdStartBusy).into();
            if let Some(recorded) = recorded {
                page.notice_peer = grouped(recorded).into();
            }
            if matches!(screen, ScreenId::R2 | ScreenId::R3) {
                page.notice_action = text(UiText::TryAgain).into();
            }
        }
        Some(failed @ (Notice::Busy | Notice::StorageFailed | Notice::StorageUnknown)) => {
            page.notice = text(match failed {
                Notice::Busy => UiText::IdBusy,
                Notice::StorageFailed => UiText::IdStorageFailed,
                _ => UiText::IdStorageUnknown,
            })
            .into();
            // R2's and R3's notice carries Try again under its text; C4's
            // is its primary, in Continue's place (section 6); on the
            // first screens and S1 it takes the secondary's place.
            match screen {
                ScreenId::R2 | ScreenId::R3 => page.notice_action = text(UiText::TryAgain).into(),
                ScreenId::C4 => {}
                _ => page.secondary = text(UiText::TryAgain).into(),
            }
        }
        None => {}
    }
    takes_peer
}
/// A screen's title, paragraphs and primary, before the statements the
/// flow owes; `None` on Done, which shows nothing. R3 alone has no title
/// (section 3.3): its question is what it asks.
fn screen_texts(
    flow: &IdentityFlow,
    screen: ScreenId,
    already_created: bool,
    start_busy: bool,
) -> Option<(Option<UiText>, Vec<UiText>, Option<UiText>)> {
    let texts = match screen {
        ScreenId::W => (
            Some(UiText::IdWelcomeTitle),
            vec![UiText::IdWelcomeBody],
            Some(UiText::IdCreate),
        ),
        ScreenId::C1 => (
            Some(UiText::IdC1Title),
            vec![
                UiText::IdC1Point1,
                UiText::IdC1Point2,
                UiText::IdC1Point3,
                UiText::IdC1Point4,
                UiText::IdC1Point5,
                UiText::IdC1Point6,
            ],
            Some(UiText::IdShowWords),
        ),
        ScreenId::C2 => (
            Some(UiText::IdWordsTitle),
            Vec::new(),
            Some(UiText::IdWrittenDown),
        ),
        ScreenId::C2PeerId => (
            Some(UiText::IdPeerIdTitle),
            Vec::new(),
            Some(UiText::IdPeerIdWritten),
        ),
        ScreenId::C3 => (Some(UiText::IdCheckTitle), vec![UiText::IdCheckBody], None),
        // Under `IdAlreadyCreated` no PeerId was created: the title says
        // the phone has one, and the body, which calls the PeerId shown
        // this identity, goes. Try again takes Continue's place while a
        // notice that retries shows (section 6).
        ScreenId::C4 if already_created => (
            Some(UiText::IdAlreadyTitle),
            Vec::new(),
            Some(continue_or_try_again(start_busy)),
        ),
        ScreenId::C4 => (
            Some(UiText::IdCreatedTitle),
            vec![UiText::IdCreatedBody],
            Some(continue_or_try_again(
                start_busy || flow.notice().is_some_and(Notice::retries),
            )),
        ),
        ScreenId::R1 => (
            Some(UiText::IdR1Title),
            vec![UiText::IdR1Body, UiText::IdR1Settings],
            Some(UiText::IdEnterWords),
        ),
        ScreenId::R2 => (
            Some(UiText::IdR1Title),
            Vec::new(),
            Some(UiText::IdCheckWords),
        ),
        ScreenId::R3 => (None, Vec::new(), Some(UiText::IdR3Yes)),
        ScreenId::R4 => (
            Some(UiText::IdRestoredTitle),
            vec![UiText::IdCreatedBody],
            Some(continue_or_try_again(start_busy)),
        ),
        ScreenId::K1 => k1_texts(flow),
        ScreenId::S1 => (
            Some(UiText::IdS1Title),
            vec![UiText::IdS1Body, UiText::IdC1Point5],
            Some(UiText::IdS1Unlock),
        ),
        ScreenId::S2 => (Some(UiText::IdWordsTitle), Vec::new(), Some(UiText::IdNext)),
        ScreenId::S2PeerId => (
            Some(UiText::IdPeerIdTitle),
            Vec::new(),
            Some(UiText::IdDone),
        ),
        ScreenId::Done => return None,
    };
    Some(texts)
}

/// K1's title, reason and primary. With no `PeerId` on the record there is
/// nothing to restore against: no primary, and a title and reason that
/// ask for no phrase (section 3.4).
fn k1_texts(flow: &IdentityFlow) -> (Option<UiText>, Vec<UiText>, Option<UiText>) {
    if !flow.can_restore() {
        return (
            Some(UiText::IdQTitleNoPath),
            vec![UiText::IdQReasonNoPath],
            None,
        );
    }
    let reason = match flow.recovery().map(|(reason, _)| reason) {
        Some(RecoveryReason::Record) => UiText::IdQReasonRecord,
        _ => UiText::IdQReasonKey,
    };
    (
        Some(UiText::IdQTitle),
        vec![reason],
        Some(UiText::IdEnterPhrase),
    )
}

/// C4's and R4's primary: Try again in Continue's place while a notice
/// it answers shows (section 6).
const fn continue_or_try_again(again: bool) -> UiText {
    if again {
        UiText::TryAgain
    } else {
        UiText::IdContinue
    }
}

/// The page for the flow's current screen.
#[must_use]
pub fn page(flow: &IdentityFlow) -> IdentityPage {
    let screen = flow.screen();
    let mut page = IdentityPage {
        key_delete_label: text(UiText::IdKeyDelete).into(),
        key_clear_label: text(UiText::IdKeyClear).into(),
        primary_enabled: true,
        dialog_title: text(UiText::IdDiscardTitle).into(),
        dialog_delete: text(UiText::IdDiscard).into(),
        dialog_keep: text(UiText::IdContinueEntering).into(),
        dialog: flow.discard_asked(),
        screen: screen_id(screen),
        // One label for the page's arrow and the full list's (section 3).
        back_arrow: flow.back_arrow(),
        back_active: !flow.waiting(),
        back_label: text(UiText::AddClose).into(),
        ..IdentityPage::default()
    };
    // `IdStartBusy` after `IdAlreadyCreated`'s Continue leaves C4 as that
    // notice left it: the words shown were still not used (section 6).
    let already_created = screen == ScreenId::C4
        && matches!(
            flow.notice(),
            Some(Notice::AlreadyCreated | Notice::StartBusy { already: true, .. })
        );
    // Try again in Continue's place: C4's and R4's primary makes one
    // service start under `IdStartBusy` (section 6).
    let start_busy = matches!(flow.notice(), Some(Notice::StartBusy { .. }));
    let Some((title, paragraphs, primary)) =
        screen_texts(flow, screen, already_created, start_busy)
    else {
        return page;
    };
    page.title = title.map(text).unwrap_or_default().into();
    // Last, after what the screen is for: the statements the flow owes
    // before R2 takes a word -- ADR-0043's warning (item 11), then that
    // the identities' keys do not come back (item 12) -- on whichever
    // screen the model
    // says carries each.
    let warning = flow
        .warns_active_elsewhere()
        .then_some(UiText::IdActiveElsewhere);
    let new_keys = flow.says_new_keys().then_some(UiText::IdR1NewKeys);
    page.paragraphs = strings(
        paragraphs
            .into_iter()
            .chain(warning)
            .chain(new_keys)
            .map(text),
    );
    page.primary = primary.map(text).unwrap_or_default().into();
    held_start(flow, &mut page);

    if screen == ScreenId::W && flow.void_paper() {
        page.notice = text(UiText::IdVoidPaper).into();
    }
    // Above the words: what the phrase is, then its format identifier
    // after a label (section 3.2). The identifier is the flow's, which the
    // root handed over from the recovery record, so it shows only where
    // the words do (`c2_shows_the_words_then_the_peer_id_on_its_own_page`).
    if let Some(format) = flow.format() {
        page.words_note = text(UiText::IdNotWalletSeed).into();
        page.format_label = text(UiText::IdFormatLine).into();
        page.format_id = format.into();
    }
    if let Some(words) = flow.words() {
        page.words = ModelRc::new(VecModel::from(
            words
                .iter()
                .enumerate()
                .map(|(i, word)| IdSlot {
                    number: i32::try_from(i + 1).unwrap_or(0),
                    word: (*word).into(),
                    current: false,
                })
                .collect::<Vec<_>>(),
        ));
    }
    // On C2's and S2's PeerId page the title names the PeerId, and the
    // note follows it, under the PeerId (sections 5 and 9).
    let peer_note = match screen {
        ScreenId::C2PeerId => Some(UiText::IdPeerIdLabel),
        ScreenId::S2PeerId => Some(UiText::IdPeerIdNoteShown),
        _ => None,
    };
    if let (Some(note), Some(peer)) = (peer_note, flow.peer()) {
        page.peer_note = text(note).into();
        page.peer_groups = grouped(peer).into();
    }
    let peer_label = match screen {
        ScreenId::C4 | ScreenId::R4 => Some(UiText::IdPeerIdLabelShown),
        ScreenId::R3 => Some(UiText::IdDerivedPeerId),
        _ => None,
    };
    // The PeerId `peer_groups` shows, for a Copy under it.
    let mut shown = peer_note.and(flow.peer());
    // Under `IdAlreadyCreated` C4's PeerId is not the person's: hidden,
    // with its label and Copy (section 6).
    if let (Some(label), Some(peer), false) = (peer_label, flow.peer(), already_created) {
        page.peer_label = text(label).into();
        page.peer_groups = grouped(peer).into();
        shown = Some(peer);
    }
    // What it hides, for the view to take its own copy of it off the
    // clipboard (section 6).
    if let (true, Some(peer)) = (already_created, flow.peer()) {
        page.void_peer = peer.as_str().into();
    }
    if let Some((_, Some(expected))) = flow.recovery() {
        page.peer_label = text(UiText::IdExpectedPeerId).into();
        page.peer_groups = grouped(expected).into();
        shown = Some(expected);
    }

    if notice(flow, screen, &mut page) {
        shown = None;
    }

    page.chips_taken = flow.chips_taken().cast_signed();
    page.positions_chosen = flow.positions_chosen().cast_signed();
    page.notices_raised = flow.notices_raised().cast_signed();
    if let Some(picker) = flow.picker() {
        page.picker = true;
        page.typed = picker.typed().into();
        match picker.suggestions() {
            Suggestions::Count { n, exact } => {
                page.matches = numbered(UiText::IdWordMatches, n).into();
                // The whole word typed is a chip of its own, beside the count.
                page.chips = strings(exact.map(str::to_owned));
            }
            Suggestions::Words(words) => {
                page.chips = strings(words.iter().map(|w| (*w).to_owned()));
            }
        }
        page.rows = rows(flow, &QWERTY);
        page.alpha_rows = rows(flow, &ALPHABET);
        page.any_key = flow
            .picker()
            .is_some_and(|p| ('a'..='z').any(|l| p.enabled(l)));
    }
    if let Some((position, wrong)) = flow.check() {
        page.position = numbered(UiText::IdWordPosition, position + 1).into();
        if let Some(wrong) = wrong {
            page.notice = numbered(UiText::IdCheckWrong, wrong + 1).into();
            page.secondary = text(UiText::TryAgain).into();
            page.tertiary = text(UiText::IdShowAgain).into();
        }
    }
    entry_grid(flow, &mut page);

    match screen {
        ScreenId::W if page.secondary.is_empty() => {
            page.secondary = text(UiText::IdRestore).into();
        }
        ScreenId::R3 => r3(flow, &mut page),
        ScreenId::R4 => page.heading_key = R4_HEADING.into(),
        ScreenId::C4 => {
            page.heading_key = C4_HEADING.into();
            c4_unchecked(flow, &mut page);
        }
        ScreenId::C2PeerId | ScreenId::S2PeerId => page.heading_key = PEER_ID_HEADING.into(),
        ScreenId::K1 => {
            page.heading_key = K1_HEADING.into();
            let way_out = if flow.can_restore() {
                UiText::IdQLost
            } else {
                UiText::IdQNoPath
            };
            page.footnote = text(way_out).into();
        }
        ScreenId::S1 if flow.waiting() => page.waiting = text(UiText::IdGoingOffline).into(),
        _ => {}
    }
    // Copy PeerId under the PeerId of C2's and S2's PeerId pages, C4, R3
    // and R4 (section 5); not on C4 while `IdAlreadyCreated` says the
    // words were not used, since its PeerId is not the person's (section
    // 6); a banner's PeerId has none, and neither has a words page, which
    // shows no PeerId (`copy_peer_id_is_built_where_section_5_puts_it`).
    if let (
        ScreenId::C2PeerId | ScreenId::S2PeerId | ScreenId::C4 | ScreenId::R3 | ScreenId::R4,
        Some(peer),
        false,
    ) = (screen, shown, already_created)
    {
        page.copy_label = text(UiText::CopyPeerId).into();
        page.peer_id = peer.as_str().into();
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;
    use interweave_human_ui_model::{Start, StepFailed, UncheckedRefused, Words};
    use slint::Model as _;

    const LIST: &[&str] = &[
        "acid", "bean", "cabin", "dawn", "eagle", "fabric", "galaxy", "habit", "icon", "jacket",
        "kale", "label", "machine", "napkin", "oak", "paddle", "quarter", "rabbit", "saddle",
        "table", "ugly", "vacant", "wagon", "yard", "zebra", "zone",
    ];

    fn words() -> Words {
        let mut words = [""; PHRASE_WORDS];
        words.copy_from_slice(&LIST[..PHRASE_WORDS]);
        words
    }

    fn mine() -> TransportIdentity {
        TransportIdentity::parse("12D3KooW9tHTtS3inCZiYykw4u5G4frbjVFqhkmJX12gSNCVeH3e")
            .expect("a PeerId")
    }

    fn other() -> TransportIdentity {
        TransportIdentity::parse("12D3KooW9xCm2jWjNVrwh51SWCQBMYdMyeU3NpT85QhLVkF6PcNM")
            .expect("a PeerId")
    }

    fn en(t: UiText) -> SharedString {
        placeholder_en::text(t).into()
    }

    #[test]
    fn a_peer_id_is_grouped_by_four_and_keeps_every_character() {
        let grouped = grouped(&mine());
        assert!(grouped.starts_with("12D3 KooW 9tHT"));
        assert_eq!(grouped.replace(' ', ""), mine().as_str());
        assert!(grouped.split(' ').all(|group| group.chars().count() <= 4));
    }

    #[test]
    fn welcome_offers_create_and_restore_and_warns_of_a_void_paper() {
        let flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: true },
        );
        let page = page(&flow);
        assert_eq!(page.title, en(UiText::IdWelcomeTitle));
        assert_eq!(page.primary, en(UiText::IdCreate));
        assert_eq!(page.secondary, en(UiText::IdRestore));
        assert_eq!(page.notice, en(UiText::IdVoidPaper));
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        assert_eq!(page_of(&mut flow, &[]).notice, "");
        apply(&mut flow, IdentityInput::Secondary);
        assert_eq!(flow.screen(), ScreenId::R1);
    }

    fn page_of(flow: &mut IdentityFlow, inputs: &[IdentityInput]) -> IdentityPage {
        for input in inputs {
            apply(flow, input.clone());
        }
        page(flow)
    }

    /// The page binds the model's ADR-0043 warning after the screen's own
    /// paragraphs on R1 and on K1 with a restore, and Welcome, R2 and K1
    /// with no restore show none; where it stands beside item 12's line is
    /// `the_new_keys_line_follows_the_warning_where_the_model_says`'s.
    #[test]
    fn r1_and_k1_show_the_active_elsewhere_warning() {
        let shows = |page: &IdentityPage| {
            page.paragraphs
                .iter()
                .any(|p| p == en(UiText::IdActiveElsewhere))
        };
        let mut fresh = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        assert!(!shows(&page(&fresh)), "Welcome restores nothing");
        let r1 = page_of(&mut fresh, &[IdentityInput::Secondary]);
        assert_eq!(r1.paragraphs.row_data(1), Some(en(UiText::IdR1Settings)));
        assert!(shows(&r1));
        let r2 = page_of(&mut fresh, &[IdentityInput::Primary]);
        assert!(!shows(&r2));

        let k1 = |expected| {
            page(&IdentityFlow::new(
                LIST,
                interweave_profile_identity::FORMAT,
                Start::Recovery {
                    reason: RecoveryReason::Key,
                    expected,
                    new_keys: false,
                },
            ))
        };
        let with_restore = k1(Some(mine()));
        assert_eq!(
            with_restore.paragraphs.row_data(0),
            Some(en(UiText::IdQReasonKey))
        );
        assert!(shows(&with_restore));
        assert!(!shows(&k1(None)), "no restore, no phrase taken");
    }

    /// K1 over a record that names no readable `PeerId` offers no restore,
    /// so nothing on it asks for the phrase or suggests the words would
    /// help: its own title, reason and way out, and no primary (section
    /// 3.4), whatever the cause.
    #[test]
    fn k1_with_no_path_asks_for_no_phrase() {
        for reason in [RecoveryReason::Key, RecoveryReason::Record] {
            let k1 = |expected| {
                page(&IdentityFlow::new(
                    LIST,
                    interweave_profile_identity::FORMAT,
                    Start::Recovery {
                        reason,
                        expected,
                        new_keys: false,
                    },
                ))
            };
            let none = k1(None);
            assert_eq!(none.title, en(UiText::IdQTitleNoPath), "{reason:?}");
            assert_eq!(none.paragraphs.row_count(), 1, "{reason:?}");
            assert_eq!(
                none.paragraphs.row_data(0),
                Some(en(UiText::IdQReasonNoPath)),
                "{reason:?}"
            );
            assert_eq!(none.footnote, en(UiText::IdQNoPath), "{reason:?}");
            assert!(none.primary.is_empty(), "{reason:?}");

            let some = k1(Some(mine()));
            assert_eq!(some.title, en(UiText::IdQTitle), "{reason:?}");
            assert_eq!(some.footnote, en(UiText::IdQLost), "{reason:?}");
            assert_eq!(some.primary, en(UiText::IdEnterPhrase), "{reason:?}");
        }
    }

    /// The page binds item 12's line right after the ADR-0043 warning,
    /// as the last paragraph: on R1, and on K1 with a restore only when
    /// the root found no `identity-keys/`; R2 shows neither.
    #[test]
    fn the_new_keys_line_follows_the_warning_where_the_model_says() {
        let tail = |page: &IdentityPage| {
            let n = page.paragraphs.row_count();
            (n.checked_sub(2).and_then(|i| page.paragraphs.row_data(i)))
                .into_iter()
                .chain(n.checked_sub(1).and_then(|i| page.paragraphs.row_data(i)))
                .collect::<Vec<_>>()
        };
        let both = vec![en(UiText::IdActiveElsewhere), en(UiText::IdR1NewKeys)];
        let shows_new_keys =
            |page: &IdentityPage| page.paragraphs.iter().any(|p| p == en(UiText::IdR1NewKeys));

        let mut fresh = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        let r1 = page_of(&mut fresh, &[IdentityInput::Secondary]);
        assert_eq!(tail(&r1), both);
        let r2 = page_of(&mut fresh, &[IdentityInput::Primary]);
        assert!(!shows_new_keys(&r2));

        let k1 = |new_keys| {
            page(&IdentityFlow::new(
                LIST,
                interweave_profile_identity::FORMAT,
                Start::Recovery {
                    reason: RecoveryReason::Key,
                    expected: Some(mine()),
                    new_keys,
                },
            ))
        };
        assert_eq!(tail(&k1(true)), both);
        let kept = k1(false);
        assert!(
            !shows_new_keys(&kept),
            "the identities' keys are still here"
        );
        assert_eq!(
            kept.paragraphs.row_data(kept.paragraphs.row_count() - 1),
            Some(en(UiText::IdActiveElsewhere))
        );
    }

    /// C2's words page: the 24 numbered words under the not-a-wallet-seed
    /// line and the format line, which carries the identifier the flow was
    /// given, and no `PeerId` and no Copy (section 3.2). Its `PeerId`
    /// page: the title, the `PeerId` with its note and Copy, no word and
    /// no format, and the title a stop (section 8). Mutation: dropping
    /// the format lines from `page`, or giving the words page the
    /// `PeerId`, fails here.
    #[test]
    fn c2_shows_the_words_then_the_peer_id_on_its_own_page() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        apply(&mut flow, IdentityInput::Primary);
        flow.created(words(), mine(), [3, 9]);
        let page = page_of(&mut flow, &[IdentityInput::Primary]);
        assert_eq!(page.screen, IdScreen::C2);
        assert_eq!(page.title, en(UiText::IdWordsTitle));
        assert_eq!(page.words.row_count(), PHRASE_WORDS);
        let first = page.words.row_data(0).expect("a word");
        assert_eq!((first.number, first.word.as_str()), (1, "acid"));
        assert_eq!(page.words_note, en(UiText::IdNotWalletSeed));
        assert_eq!(page.format_label, en(UiText::IdFormatLine));
        assert_eq!(page.format_id, interweave_profile_identity::FORMAT);
        assert_eq!(page.peer_groups, "", "no PeerId beside the words");
        assert_eq!(page.peer_id, "");
        assert_eq!(page.copy_label, "");
        assert_eq!(page.heading_key, "");
        assert_eq!(page.primary, en(UiText::IdWrittenDown));

        let page = page_of(&mut flow, &[IdentityInput::Primary]);
        assert_eq!(page.screen, IdScreen::C2PeerId);
        assert_eq!(page.title, en(UiText::IdPeerIdTitle));
        assert_eq!(page.heading_key, PEER_ID_HEADING, "the title is a stop");
        assert_eq!(page.words.row_count(), 0, "no word beside the PeerId");
        assert_eq!(page.words_note, "");
        assert_eq!(page.format_id, "");
        assert_eq!(page.peer_groups, grouped(&mine()));
        assert_eq!(page.peer_label, "", "the title names it");
        assert_eq!(page.peer_note, en(UiText::IdPeerIdLabel));
        assert_eq!(page.copy_label, en(UiText::CopyPeerId));
        assert_eq!(page.peer_id.as_str(), mine().as_str());
        assert_eq!(page.primary, en(UiText::IdPeerIdWritten));
        assert!(page.back_arrow);

        let page = page_of(&mut flow, &[IdentityInput::Back]);
        assert_eq!(page.screen, IdScreen::C2, "Back: the words again");
        assert_eq!(page.words.row_count(), PHRASE_WORDS);
    }

    /// S2's words page has Next where C2 has `IdWrittenDown`, and its
    /// `PeerId` page the note for a `PeerId` already written and Done
    /// (section 3.5). Mutation: S2's words page with Done, or its
    /// `PeerId` page with C2's note, fails here.
    #[test]
    fn s2_shows_the_words_with_next_then_the_peer_id_with_done() {
        let mut flow =
            IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
        apply(&mut flow, IdentityInput::Primary);
        flow.prompted(true);
        flow.offline(Ok((words(), mine())));
        let page = page(&flow);
        assert_eq!(page.screen, IdScreen::S2);
        assert_eq!(page.title, en(UiText::IdWordsTitle));
        assert_eq!(page.words.row_count(), PHRASE_WORDS);
        assert_eq!(page.words_note, en(UiText::IdNotWalletSeed));
        assert_eq!(page.format_id, interweave_profile_identity::FORMAT);
        assert_eq!(page.peer_groups, "");
        assert_eq!(page.primary, en(UiText::IdNext));

        let page = page_of(&mut flow, &[IdentityInput::Primary]);
        assert_eq!(page.screen, IdScreen::S2PeerId);
        assert_eq!(page.title, en(UiText::IdPeerIdTitle));
        assert_eq!(page.heading_key, PEER_ID_HEADING);
        assert_eq!(page.words.row_count(), 0);
        assert_eq!(page.format_id, "");
        assert_eq!(page.peer_groups, grouped(&mine()));
        assert_eq!(page.peer_note, en(UiText::IdPeerIdNoteShown));
        assert_eq!(page.copy_label, en(UiText::CopyPeerId));
        assert_eq!(page.primary, en(UiText::IdDone));
    }

    #[test]
    fn the_keypad_disables_dead_letters_and_r2_checks_only_when_complete() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        let page = page_of(
            &mut flow,
            &[IdentityInput::Secondary, IdentityInput::Primary],
        );
        assert!(page.picker);
        let count = |rows: &ModelRc<IdKeyRow>| {
            rows.iter()
                .map(|row| row.keys.row_count())
                .collect::<Vec<_>>()
        };
        assert_eq!(count(&page.rows), [10, 9, 7]);
        assert_eq!(count(&page.alpha_rows), [7, 7, 7, 5]);
        let q = page
            .rows
            .row_data(0)
            .and_then(|r| r.keys.row_data(0))
            .expect("q");
        assert_eq!((q.letter.as_str(), q.enabled), ("q", true));
        let x = page
            .alpha_rows
            .row_data(3)
            .and_then(|r| r.keys.row_data(2))
            .expect("x");
        assert_eq!(
            (x.letter.as_str(), x.enabled),
            ("x", false),
            "no word starts with x"
        );
        assert_eq!(page.position, "Word 1 of 24");
        assert_eq!(page.matches, "26 words fit these letters");
        assert!(!page.primary_enabled, "check waits for 24 words");
        assert_eq!(page.grid.row_count(), PHRASE_WORDS);

        let page = page_of(&mut flow, &[IdentityInput::Key('z')]);
        assert_eq!(page.chips.row_count(), 2);
        assert_eq!(page.typed, "z");
    }

    #[test]
    fn another_identity_shows_both_peer_ids_and_storage_failure_offers_try_again() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            interweave_human_ui_model::Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: false,
            },
        );
        let k1 = page(&flow);
        assert_eq!(k1.paragraphs.row_data(0), Some(en(UiText::IdQReasonKey)));
        assert_eq!(k1.peer_label, en(UiText::IdExpectedPeerId));
        assert_eq!(k1.footnote, en(UiText::IdQLost));
        apply(&mut flow, IdentityInput::Primary);
        for word in words() {
            for letter in word.chars() {
                if flow.picker().is_some_and(
                    |p| matches!(p.suggestions(), Suggestions::Words(w) if w.contains(&word)),
                ) {
                    break;
                }
                apply(&mut flow, IdentityInput::Key(letter));
            }
            apply(&mut flow, IdentityInput::Chip(word.to_owned()));
        }
        assert!(page(&flow).primary_enabled);
        apply(&mut flow, IdentityInput::Primary);
        flow.checked(Some(other()));
        let page = page(&flow);
        assert_eq!(page.notice, en(UiText::IdOtherIdentity));
        assert_eq!(page.peer_groups, grouped(&other()));
        assert_eq!(page.second_peer_groups, grouped(&mine()));

        apply(&mut flow, IdentityInput::Primary);
        flow.checked(Some(mine()));
        flow.restored(Err(StepFailed::Storage));
        let failed = super::page(&flow);
        assert_eq!(failed.notice, en(UiText::IdStorageFailed));
        // Under the notice's text, not in the secondary's place (section 6).
        assert_eq!(failed.notice_action, en(UiText::TryAgain));
        assert_eq!(failed.secondary, "");
        assert_eq!(apply(&mut flow, IdentityInput::Secondary), []);
        let effects = apply(&mut flow, IdentityInput::NoticeAction);
        assert!(
            matches!(effects.as_slice(), [Effect::Check(_)]),
            "Try again retries: {effects:?}"
        );
        assert!(
            super::page(&flow).notice_action_held,
            "held while it checks"
        );
    }

    /// R2 after K1 at the restore's result.
    fn r2_after_k1_restoring() -> IdentityFlow {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: false,
            },
        );
        apply(&mut flow, IdentityInput::Primary);
        type_all(&mut flow);
        apply(&mut flow, IdentityInput::Primary);
        assert!(
            matches!(
                flow.checked(Some(mine())).as_slice(),
                [Effect::RestoreRecorded(_)]
            ),
            "control: the restore"
        );
        flow
    }

    /// `IdStorageUnknown` and `IdRestoreOther` on R2 and R3 (section 6):
    /// the notice in the one place above the content, its action under
    /// its text -- Try again, or Continue -- and `IdRestoreOther`'s `PeerId`
    /// under its text for the eye alone, with no Copy; R3 keeps the Copy
    /// of its own `PeerId`. After that Continue, R2 holds and shows every
    /// held control disabled (section 8): the keys, the action, Check my
    /// words, and the back arrow inactive. Mutation: `NoticeAction`
    /// routed to `primary` again, `notice_peer` left unset, or the keys
    /// left enabled under the hold, fails here.
    #[test]
    fn r2_and_r3_carry_storage_unknown_and_restore_other_with_their_actions() {
        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::StorageUnknown));
        let unknown = page(&r2);
        assert_eq!(unknown.notice, en(UiText::IdStorageUnknown));
        assert_eq!(unknown.notice_action, en(UiText::TryAgain));
        assert_eq!(unknown.secondary, "");
        assert_eq!(
            apply(&mut r2, IdentityInput::NoticeAction),
            [Effect::ReadBack(mine())]
        );

        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::RestoreOther(other())));
        let shown = page(&r2);
        assert_eq!(shown.notice, en(UiText::IdRestoreOther));
        assert_eq!(shown.notice_peer, grouped(&other()));
        assert_eq!(shown.notice_action, en(UiText::IdContinue));
        assert_eq!(shown.copy_label, "", "the record's PeerId has no Copy");
        assert!(!shown.held && !shown.notice_action_held, "control");
        let key_enabled = |page: &IdentityPage| {
            page.rows
                .iter()
                .flat_map(|row| row.keys.iter().collect::<Vec<_>>())
                .any(|key| key.enabled)
        };
        assert!(key_enabled(&shown), "control: keys enabled");
        assert_eq!(
            apply(&mut r2, IdentityInput::NoticeAction),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        let held = page(&r2);
        assert!(held.held, "every held control shows disabled");
        assert!(held.notice_action_held, "Continue disabled");
        assert!(!held.primary_enabled, "Check my words disabled");
        assert!(!held.back_active, "the back arrow inactive");
        assert!(!key_enabled(&held), "every key disabled");
        assert_eq!(held.notice_peer, grouped(&other()), "the PeerId stays");
        assert_eq!(apply(&mut r2, IdentityInput::NoticeAction), []);
        r2.set_starting(true);
        assert_eq!(page(&r2).starting, en(UiText::ConnectStarting));

        let mut r3 = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        apply(&mut r3, IdentityInput::Secondary);
        apply(&mut r3, IdentityInput::Primary);
        type_all(&mut r3);
        apply(&mut r3, IdentityInput::Primary);
        r3.checked(Some(mine()));
        apply(&mut r3, IdentityInput::Primary);
        r3.restored(Err(StepFailed::RestoreOther(other())));
        let r3_page = page(&r3);
        assert_eq!(r3_page.notice, en(UiText::IdRestoreOther));
        assert_eq!(r3_page.notice_peer, grouped(&other()));
        assert_eq!(r3_page.notice_action, en(UiText::IdContinue));
        assert_eq!(r3_page.secondary, en(UiText::IdR3No));
        assert_eq!(
            r3_page.peer_id.as_str(),
            mine().as_str(),
            "Copy copies R3's own PeerId, never the record's"
        );
        apply(&mut r3, IdentityInput::NoticeAction);
        let held = page(&r3);
        assert!(held.notice_action_held && held.secondary_held && !held.primary_enabled);
    }

    /// C4's notices (section 6): no notice carries an action; under
    /// `IdBusy`, `IdStorageFailed` and `IdStorageUnknown` the primary is
    /// Try again in Continue's place, and under `IdStorageUnknown` it
    /// reads back; under `IdAlreadyCreated` the title is `IdAlreadyTitle`,
    /// the primary Continue, and the `PeerId`, its label, Copy and
    /// `IdCreatedBody` are hidden. C4's title is a stop (section 8).
    /// Mutation: the primary left `IdContinue` under a retrying notice,
    /// Try again put back in the secondary, or the `PeerId` shown under
    /// `IdAlreadyCreated`, fails here.
    #[test]
    fn c4s_try_again_is_its_primary_and_already_created_hides_the_peer_id() {
        assert_eq!(page(&at_c4()).heading_key, C4_HEADING, "the title a stop");
        for (failed, notice) in [
            (StepFailed::Busy, UiText::IdBusy),
            (StepFailed::Storage, UiText::IdStorageFailed),
            (StepFailed::StorageUnknown, UiText::IdStorageUnknown),
        ] {
            let mut flow = at_c4();
            apply(&mut flow, IdentityInput::Primary);
            flow.sealed(Err(failed.clone()));
            let c4 = page(&flow);
            assert_eq!(c4.notice, en(notice), "{failed:?}");
            assert_eq!(c4.primary, en(UiText::TryAgain), "{failed:?}");
            // The only second control: `IdStartUnchecked`, after a
            // read-back answered as failing, never Try again.
            let second = if failed == StepFailed::StorageUnknown {
                en(UiText::IdStartUnchecked)
            } else {
                SharedString::new()
            };
            assert_eq!(c4.secondary, second, "{failed:?}: the second control");
            assert_eq!(c4.notice_action, "", "{failed:?}: no action of its own");
            assert_eq!(c4.title, en(UiText::IdCreatedTitle));
            assert_eq!(c4.peer_groups, grouped(&mine()), "{failed:?}: kept");
            let again = apply(&mut flow, IdentityInput::Primary);
            if failed == StepFailed::StorageUnknown {
                assert_eq!(again, [Effect::ReadBack(mine())], "reads back");
            } else {
                assert_eq!(again, [Effect::Seal], "{failed:?}: seals again");
            }
        }

        let mut flow = at_c4();
        apply(&mut flow, IdentityInput::Primary);
        flow.sealed(Err(StepFailed::AlreadyProvisioned));
        let c4 = page(&flow);
        assert_eq!(c4.title, en(UiText::IdAlreadyTitle));
        assert_eq!(c4.notice, en(UiText::IdAlreadyCreated));
        assert_eq!(c4.primary, en(UiText::IdContinue));
        assert_eq!((c4.peer_label.as_str(), c4.peer_groups.as_str()), ("", ""));
        assert_eq!(c4.copy_label, "");
        assert!(
            !c4.paragraphs.iter().any(|p| p == en(UiText::IdCreatedBody)),
            "no IdCreatedBody"
        );
    }

    /// R3's page (sections 3.3, 6 and 8): `IdR3NoPeerId` under the
    /// question, a failed restore's Try again under the notice with No
    /// still the secondary, and while a restore runs Yes, No and Try
    /// again held and Back inactive. Mutation: Try again back in the
    /// secondary's place, or any of the four held fields left as they
    /// were, fails here.
    #[test]
    fn r3s_notice_action_is_its_own_and_the_restore_holds_the_page() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        apply(&mut flow, IdentityInput::Secondary);
        apply(&mut flow, IdentityInput::Primary);
        for (at, word) in words().into_iter().enumerate() {
            apply(&mut flow, IdentityInput::Slot(at));
            for letter in word.chars() {
                if flow.picker().and_then(|p| p.chip(word)).is_some() {
                    break;
                }
                apply(&mut flow, IdentityInput::Key(letter));
            }
            apply(&mut flow, IdentityInput::Chip(word.to_owned()));
        }
        apply(&mut flow, IdentityInput::Primary);
        flow.checked(Some(mine()));
        assert_eq!(flow.screen(), ScreenId::R3, "control: R3");
        let r3 = page(&flow);
        assert_eq!(r3.question_note, en(UiText::IdR3NoPeerId));
        assert_eq!(r3.footnote, "", "not below the actions");
        assert_eq!(r3.secondary, en(UiText::IdR3No));
        assert!(r3.primary_enabled && !r3.secondary_held && r3.back_active);

        assert!(matches!(
            apply(&mut flow, IdentityInput::Primary).as_slice(),
            [Effect::RestoreConfirmed(..)]
        ));
        let held = page(&flow);
        assert!(!held.primary_enabled, "Yes held");
        assert!(held.secondary_held, "No held");
        assert!(!held.back_active, "Back inactive");
        assert_eq!(apply(&mut flow, IdentityInput::Secondary), []);
        assert_eq!(flow.screen(), ScreenId::R3, "No did nothing");

        flow.restored(Err(StepFailed::Storage));
        let failed = page(&flow);
        assert_eq!(failed.notice, en(UiText::IdStorageFailed));
        assert_eq!(failed.notice_action, en(UiText::TryAgain));
        assert_eq!(failed.secondary, en(UiText::IdR3No), "No keeps its place");
        assert!(!failed.notice_action_held, "control: Try again enabled");
        assert!(matches!(
            apply(&mut flow, IdentityInput::NoticeAction).as_slice(),
            [Effect::RestoreConfirmed(..)]
        ));
        let again = page(&flow);
        assert!(again.notice_action_held, "Try again held");
        assert_eq!(again.notice, en(UiText::IdStorageFailed), "it stays");
        assert_eq!(apply(&mut flow, IdentityInput::NoticeAction), []);
    }

    /// Every word of `words()` typed into `flow`, which is on R2.
    fn type_all(flow: &mut IdentityFlow) {
        for (at, word) in words().into_iter().enumerate() {
            apply(flow, IdentityInput::Slot(at));
            for letter in word.chars() {
                if flow.picker().and_then(|p| p.chip(word)).is_some() {
                    break;
                }
                apply(flow, IdentityInput::Key(letter));
            }
            apply(flow, IdentityInput::Chip(word.to_owned()));
        }
        assert_eq!(flow.screen(), ScreenId::R2, "control: R2");
    }

    /// Copy `PeerId` is built under the `PeerId` of C2's and S2's `PeerId`
    /// pages, C4, R3 and R4 (section 5), and on no other page: never on
    /// a words page, and not under R2's `IdOtherIdentity` banner.
    /// Mutation: `page`'s copy arm matching any screen (`(_, Some(peer))`)
    /// fails here, as does dropping the `PeerId` pages from it.
    /// `IdOtherIdentity` shows only on R2, which has no Copy, so its
    /// `shown = None` is not observable; `IdRestoreOther` on R3 is.
    #[test]
    fn copy_peer_id_is_built_where_section_5_puts_it() {
        let copies = |page: &IdentityPage| {
            (
                page.copy_label == en(UiText::CopyPeerId),
                !page.peer_id.is_empty(),
            )
        };

        let mut c2 = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        apply(&mut c2, IdentityInput::Primary);
        c2.created(words(), mine(), [3, 9]);
        let c2_words = page_of(&mut c2, &[IdentityInput::Primary]);
        assert!(
            c2_words.words.row_count() > 0,
            "control: C2 shows its words"
        );
        assert_eq!(copies(&c2_words), (false, false), "C2's words page");
        let c2 = page_of(&mut c2, &[IdentityInput::Primary]);
        assert!(!c2.peer_groups.is_empty(), "control: C2 shows its PeerId");
        assert_eq!(copies(&c2), (true, true), "C2's PeerId page");

        let mut s2 =
            IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
        apply(&mut s2, IdentityInput::Primary);
        s2.prompted(true);
        s2.offline(Ok((words(), mine())));
        assert_eq!(s2.screen(), ScreenId::S2, "control: S2");
        let s2_words = page(&s2);
        assert!(
            s2_words.words.row_count() > 0,
            "control: S2 shows its words"
        );
        assert_eq!(copies(&s2_words), (false, false), "S2's words page");
        let s2 = page_of(&mut s2, &[IdentityInput::Primary]);
        assert!(!s2.peer_groups.is_empty(), "control: S2 shows its PeerId");
        assert_eq!(copies(&s2), (true, true), "S2's PeerId page");

        let c4 = page(&at_c4());
        assert!(!c4.peer_groups.is_empty(), "control: C4 shows its PeerId");
        assert_eq!(copies(&c4), (true, true), "C4");

        let mut r2 = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        apply(&mut r2, IdentityInput::Secondary);
        apply(&mut r2, IdentityInput::Primary);
        type_all(&mut r2);
        apply(&mut r2, IdentityInput::Primary);
        // An OtherIdentity banner needs an expected PeerId: R2 after K1.
        let mut k1 = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: false,
            },
        );
        apply(&mut k1, IdentityInput::Primary);
        type_all(&mut k1);
        apply(&mut k1, IdentityInput::Primary);
        k1.checked(Some(other()));
        let banner = page(&k1);
        assert_eq!(banner.notice, en(UiText::IdOtherIdentity), "control");
        assert_eq!(copies(&banner), (false, false), "R2's banner");

        r2.checked(Some(mine()));
        assert_eq!(r2.screen(), ScreenId::R3, "control: R3");
        assert_eq!(copies(&page(&r2)), (true, true), "R3");
        r2.restored(Ok(()));
        assert_eq!(r2.screen(), ScreenId::R4, "control: R4");
        assert_eq!(copies(&page(&r2)), (true, true), "R4");
    }

    /// Welcome with the lock refused: the notice and Try again, which
    /// asks for the lock again rather than opening Restore (#309 F4).
    #[test]
    fn welcome_unheld_shows_busy_and_try_again_asks_for_the_lock() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.held(Err(StepFailed::Busy));
        let page = page(&flow);
        assert_eq!(page.notice, en(UiText::IdBusy));
        assert_eq!(page.secondary, en(UiText::TryAgain));
        assert_eq!(apply(&mut flow, IdentityInput::Secondary), [Effect::Hold]);
        assert_eq!(flow.screen(), ScreenId::W);
    }

    /// S1 says, under its body, what the words do not restore: C1's fifth
    /// point (identity note section 3.5, the `IdC1Point5` row).
    #[test]
    fn s1_says_what_the_words_do_not_restore_under_its_body() {
        let flow = IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
        let s1 = page(&flow);
        assert_eq!(s1.paragraphs.row_data(0), Some(en(UiText::IdS1Body)));
        assert_eq!(s1.paragraphs.row_data(1), Some(en(UiText::IdC1Point5)));
    }

    /// S1's primary raises the system prompt: it has a key of its own,
    /// never C4's and R4's Continue, which moves on (the owner, on the
    /// Pixel; language-culture-en, REPLY 01a15d22).
    #[test]
    fn s1_asks_to_unlock_where_c4_continues() {
        let flow = IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
        assert_eq!(page(&flow).primary, en(UiText::IdS1Unlock));
        assert_ne!(en(UiText::IdS1Unlock), en(UiText::IdContinue));

        let flow = at_c4();
        assert_eq!(page(&flow).primary, en(UiText::IdContinue));
    }

    fn at_c4() -> IdentityFlow {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        apply(&mut flow, IdentityInput::Primary);
        flow.created(words(), mine(), [3, 9]);
        // C2's words, its PeerId page, then C3.
        for _ in 0..3 {
            apply(&mut flow, IdentityInput::Primary);
        }
        for _ in 0..2 {
            let (position, _) = flow.check().expect("C3 checks a word");
            let word = words()[position];
            for letter in word.chars() {
                if flow.picker().and_then(|p| p.chip(word)).is_some() {
                    break;
                }
                assert!(flow.press(letter), "{letter} is enabled");
            }
            flow.choose(word);
        }
        assert_eq!(flow.screen(), ScreenId::C4, "control: at C4");
        flow
    }

    /// C4 after a read-back answered as failing (section 6, A start from
    /// C4 after a failed read-back; section 9): `IdStartUnchecked` is the
    /// secondary -- a text button after the primary, Try again, in C4's
    /// order -- with `IdStorageUnknownRestore` under it as text that takes
    /// no focus; neither after a cancelled read-back prompt. While Try
    /// again's read-back runs, and while its own start runs, it keeps its
    /// stop, shown disabled, and a second press starts nothing. After a
    /// mismatch only Try again is left; under `IdStartBusy` after its
    /// start, Try again alone, the line gone. Mutations: `c4_unchecked`
    /// not called, or its `secondary_held` dropped, or the Secondary
    /// routed nowhere on C4, fail here.
    #[test]
    fn c4_offers_start_unchecked_under_try_again_after_a_failed_read_back() {
        let mut cancelled = at_c4();
        apply(&mut cancelled, IdentityInput::Primary);
        cancelled.sealed(Err(StepFailed::ReadBackNeedsAuthentication));
        cancelled.step_not_run(&Effect::ReadBack(mine()));
        let shown = page(&cancelled);
        assert_eq!(shown.notice, en(UiText::IdStorageUnknown), "control");
        assert_eq!(
            (shown.secondary.as_str(), shown.footnote.as_str()),
            ("", "")
        );
        assert_eq!(apply(&mut cancelled, IdentityInput::Secondary), []);

        let mut flow = at_c4();
        apply(&mut flow, IdentityInput::Primary);
        flow.sealed(Err(StepFailed::StorageUnknown));
        let shown = page(&flow);
        assert_eq!(shown.notice, en(UiText::IdStorageUnknown));
        assert_eq!(shown.primary, en(UiText::TryAgain));
        assert_eq!(shown.secondary, en(UiText::IdStartUnchecked));
        assert_eq!(shown.footnote, en(UiText::IdStorageUnknownRestore));
        assert!(shown.primary_enabled && !shown.secondary_held, "enabled");
        assert_eq!(shown.peer_groups, grouped(&mine()), "the PeerId it names");

        // Try again's read-back: both shown, disabled.
        assert_eq!(
            apply(&mut flow, IdentityInput::Primary),
            [Effect::ReadBack(mine())]
        );
        let held = page(&flow);
        assert!(!held.primary_enabled && held.secondary_held, "both held");
        assert_eq!(held.secondary, en(UiText::IdStartUnchecked), "kept");
        assert_eq!(apply(&mut flow, IdentityInput::Secondary), [], "inert");
        flow.sealed(Err(StepFailed::StorageUnknown));

        // Its own press: a start, nothing written; held as Continue is.
        assert_eq!(
            apply(&mut flow, IdentityInput::Secondary),
            [Effect::Discard, Effect::StartService]
        );
        let running = page(&flow);
        assert!(!running.primary_enabled && running.secondary_held);
        assert_eq!(running.secondary, en(UiText::IdStartUnchecked));
        assert_eq!(running.notice, en(UiText::IdStorageUnknown), "stays");
        assert_eq!(running.peer_groups, grouped(&mine()), "stays");
        assert_eq!(apply(&mut flow, IdentityInput::Secondary), [], "once");
        assert_eq!(apply(&mut flow, IdentityInput::Primary), [], "Try again");

        // `LockHeld`: `IdStartBusy`, Try again in place of both.
        flow.start_busy();
        let busy = page(&flow);
        assert_eq!(busy.notice, en(UiText::IdStartBusy));
        assert_eq!(busy.primary, en(UiText::TryAgain));
        assert_eq!((busy.secondary.as_str(), busy.footnote.as_str()), ("", ""));

        // Its Try again, met by another PeerId: Try again only.
        assert_eq!(
            apply(&mut flow, IdentityInput::Primary),
            [Effect::StartService]
        );
        flow.unchecked_refused(UncheckedRefused::OtherIdentity);
        let only = page(&flow);
        assert_eq!(only.notice, en(UiText::IdStorageUnknown));
        assert!(only.primary_enabled);
        assert_eq!((only.secondary.as_str(), only.footnote.as_str()), ("", ""));
        assert_eq!(apply(&mut flow, IdentityInput::Secondary), []);
    }

    /// A held C4 keeps Continue in its place, disabled, and shows
    /// `ConnectStarting` under it only once the root says so; a failed
    /// seal gives Continue back with nothing under it (add-a-peer §5.1).
    /// Mutation: dropping either assignment in `page` fails here.
    #[test]
    fn a_held_continue_is_disabled_and_carries_the_starting_line_when_told() {
        let mut flow = at_c4();
        let before = page(&flow);
        assert!(
            before.primary_enabled && before.starting.is_empty(),
            "control"
        );
        apply(&mut flow, IdentityInput::Primary);
        let held = page(&flow);
        assert_eq!(held.primary, en(UiText::IdContinue), "Continue stays");
        assert!(!held.primary_enabled, "disabled at once");
        assert!(held.starting.is_empty(), "nothing before the 1 s");
        flow.set_starting(true);
        assert_eq!(page(&flow).starting, en(UiText::ConnectStarting));
        assert!(
            page(&flow).waiting.is_empty(),
            "under Continue, not in its place"
        );
        flow.sealed(Err(interweave_human_ui_model::StepFailed::Storage));
        let failed = page(&flow);
        assert!(failed.primary_enabled && failed.starting.is_empty());
    }

    #[test]
    fn s1_shows_going_offline_in_place_of_its_action() {
        let mut flow =
            IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
        apply(&mut flow, IdentityInput::Primary);
        flow.prompted(true);
        let page = page(&flow);
        assert_eq!(page.waiting, en(UiText::IdGoingOffline));
    }

    /// The app bar's arrow is drawn inactive while Back waits: S1 going
    /// offline (acceptance 15).
    #[test]
    fn the_back_arrow_is_inactive_while_the_page_waits() {
        let mut flow =
            IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
        apply(&mut flow, IdentityInput::Primary);
        let s1 = page(&flow);
        assert!(s1.back_arrow && s1.back_active, "control: S1, not waiting");
        flow.prompted(true);
        assert!(flow.waiting(), "control: going offline");
        let waiting = page(&flow);
        assert!(waiting.back_arrow, "the arrow stays");
        assert!(!waiting.back_active, "and is inactive");
    }

    /// "The first enabled key" falls back to delete when what is typed
    /// is a word no letter continues (section 8).
    #[test]
    fn any_key_is_false_once_no_letter_continues_what_is_typed() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.restore();
        flow.primary();
        assert!(page(&flow).any_key, "control: nothing typed");
        for letter in "acid".chars() {
            flow.press(letter);
        }
        assert_eq!(
            flow.picker()
                .map(interweave_human_ui_model::WordPicker::typed),
            Some("acid")
        );
        assert!(!page(&flow).any_key);
    }

    /// C4 carries Copy `PeerId` under its `PeerId`, with the canonical
    /// string (section 5), and not while `IdAlreadyCreated` says the words were
    /// not used (section 6); C2's words page shows no `PeerId` and
    /// carries none. The `PeerId` it hides then is the page's void one,
    /// for the view to take its copy off the clipboard (section 6), and no
    /// other page names one. Mutation: leaving `ScreenId::C4` out of
    /// `page`'s Copy match, or dropping the void-paper condition, fails
    /// here; so does `void_peer` set without `already_created`.
    #[test]
    fn c4_copies_its_peer_id_unless_the_words_were_not_used() {
        let flow = at_c4();
        let shown = page(&flow);
        assert_eq!(shown.copy_label, en(UiText::CopyPeerId));
        assert_eq!(shown.peer_id.as_str(), mine().as_str());
        assert!(shown.peer_groups.contains(' '), "control: grouped");
        assert_eq!(shown.void_peer, "", "C4's PeerId is the person's");

        for failed in [StepFailed::Busy, StepFailed::Storage] {
            let mut flow = at_c4();
            flow.primary();
            flow.sealed(Err(failed.clone()));
            assert_eq!(
                page(&flow).copy_label,
                en(UiText::CopyPeerId),
                "{failed:?}: kept"
            );
            assert_eq!(page(&flow).void_peer, "", "{failed:?}: none void");
        }
        let mut flow = at_c4();
        flow.primary();
        flow.sealed(Err(StepFailed::AlreadyProvisioned));
        assert_eq!(flow.notice(), Some(&Notice::AlreadyCreated), "control");
        assert_eq!(
            page(&flow).copy_label,
            "",
            "not while the words were unused"
        );
        assert_eq!(page(&flow).void_peer.as_str(), mine().as_str(), "void");

        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        apply(&mut flow, IdentityInput::Primary);
        flow.created(words(), mine(), [3, 9]);
        apply(&mut flow, IdentityInput::Primary);
        assert_eq!(flow.screen(), ScreenId::C2, "control: C2");
        assert_eq!(page(&flow).copy_label, "", "C2's words page");
    }

    /// R3, its restore refused `IdRestoreOther` and that notice's
    /// Continue pressed: its start asked.
    fn r3_continued() -> IdentityFlow {
        let mut r3 = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        apply(&mut r3, IdentityInput::Secondary);
        apply(&mut r3, IdentityInput::Primary);
        type_all(&mut r3);
        apply(&mut r3, IdentityInput::Primary);
        r3.checked(Some(mine()));
        apply(&mut r3, IdentityInput::Primary);
        r3.restored(Err(StepFailed::RestoreOther(other())));
        apply(&mut r3, IdentityInput::NoticeAction);
        r3
    }

    /// Acceptance 12's `IdStartBusy` places, as far as a page says them
    /// (design section 6, the row for a service start refused with the
    /// profile in use): on C4 after a seal and on R4, the notice in C4's
    /// notice place with Try again in Continue's and the `PeerId` with its
    /// Copy; on C4 under `IdAlreadyCreated`, `IdAlreadyTitle` kept, Try
    /// again in Continue's place, and the `PeerId`, its label, Copy and
    /// `IdCreatedBody` hidden; on R3 and on R2 after K1, Try again in
    /// `IdRestoreOther`'s Continue's place under the notice, with the
    /// `PeerId` that notice showed kept under it. Pressing Try again
    /// starts the service and nothing else, and holds the page. Where each
    /// place is on screen and in the accessibility tree is the tester's to
    /// look at on the phone: BLOCKED, not passed, here.
    ///
    /// Mutations: C4's or R4's primary left `IdContinue` under the notice,
    /// the title back to `IdCreatedTitle` under `already`, or
    /// `notice_peer` left unset under `IdStartBusy`, fails here.
    #[test]
    fn start_busy_shows_in_each_screens_notice_place_with_try_again() {
        // C4 after a seal.
        let mut c4 = at_c4();
        apply(&mut c4, IdentityInput::Primary);
        c4.sealed(Ok(()));
        c4.start_busy();
        let shown = page(&c4);
        assert_eq!(shown.notice, en(UiText::IdStartBusy));
        assert_eq!(shown.title, en(UiText::IdCreatedTitle));
        assert_eq!(shown.primary, en(UiText::TryAgain));
        assert!(shown.primary_enabled, "the hold ended");
        assert_eq!(shown.notice_action, "", "no action of its own on C4");
        assert_eq!(shown.peer_groups, grouped(&mine()), "the PeerId shows");
        assert_eq!(shown.copy_label, en(UiText::CopyPeerId));
        assert_eq!(shown.starting, "", "no progress under Try again");
        assert_eq!(
            apply(&mut c4, IdentityInput::Primary),
            [Effect::StartService],
            "one start, no seal"
        );
        assert!(!page(&c4).primary_enabled, "held from the press");
        assert_eq!(page(&c4).notice, en(UiText::IdStartBusy), "stays");

        // C4 under `IdAlreadyCreated`.
        let mut already = at_c4();
        apply(&mut already, IdentityInput::Primary);
        already.sealed(Err(StepFailed::AlreadyProvisioned));
        apply(&mut already, IdentityInput::Primary);
        already.start_busy();
        let shown = page(&already);
        assert_eq!(shown.notice, en(UiText::IdStartBusy));
        assert_eq!(shown.title, en(UiText::IdAlreadyTitle), "title kept");
        assert_eq!(shown.primary, en(UiText::TryAgain));
        assert_eq!(
            (shown.peer_label.as_str(), shown.peer_groups.as_str()),
            ("", "")
        );
        assert_eq!(shown.copy_label, "");
        assert!(
            !shown
                .paragraphs
                .iter()
                .any(|p| p == en(UiText::IdCreatedBody)),
            "no IdCreatedBody"
        );

        // R4.
        let mut r4 = r2_after_k1_restoring();
        r4.restored(Ok(()));
        assert_eq!(r4.screen(), ScreenId::R4, "control: R4");
        apply(&mut r4, IdentityInput::Primary);
        r4.start_busy();
        let shown = page(&r4);
        assert_eq!(shown.notice, en(UiText::IdStartBusy));
        assert_eq!(shown.title, en(UiText::IdRestoredTitle));
        assert_eq!(shown.primary, en(UiText::TryAgain));
        assert_eq!(shown.peer_groups, grouped(&mine()));
        assert_eq!(apply(&mut r4, IdentityInput::Secondary), [], "no secondary");
        assert_eq!(
            apply(&mut r4, IdentityInput::Primary),
            [Effect::StartService]
        );

        // R2 after K1, and R3.
        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::RestoreOther(other())));
        apply(&mut r2, IdentityInput::NoticeAction);
        r2.start_busy();
        let mut r3 = r3_continued();
        r3.start_busy();
        for (place, flow) in [("R2", &mut r2), ("R3", &mut r3)] {
            let shown = page(flow);
            assert_eq!(shown.notice, en(UiText::IdStartBusy), "{place}");
            assert_eq!(shown.notice_peer, grouped(&other()), "{place}: kept");
            assert_eq!(shown.notice_action, en(UiText::TryAgain), "{place}");
            assert!(!shown.notice_action_held, "{place}: the hold ended");
            assert_eq!(
                apply(flow, IdentityInput::NoticeAction),
                [Effect::StartService],
                "{place}: one start"
            );
            let held = page(flow);
            assert!(held.notice_action_held, "{place}: Try again disabled");
            assert!(!held.back_active, "{place}: Back does nothing");
            assert_eq!(held.notice_peer, grouped(&other()), "{place}: stays");
        }
        assert!(page(&r2).held, "R2: every held control disabled");
        assert!(page(&r3).secondary_held, "R3: No held");
    }
}
