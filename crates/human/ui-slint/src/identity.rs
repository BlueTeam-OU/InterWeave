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
}

/// Apply `input` to `flow`; the effects are the root's to carry out.
pub fn apply(flow: &mut IdentityFlow, input: IdentityInput) -> Vec<Effect> {
    let screen = flow.screen();
    let failed = matches!(flow.notice(), Some(Notice::Busy | Notice::StorageFailed));
    match input {
        IdentityInput::Primary => flow.primary(),
        IdentityInput::Secondary => match screen {
            // On Welcome a refusal's Try again takes the secondary's place.
            ScreenId::W if !failed => flow.restore(),
            ScreenId::R3 if !failed => {
                flow.recheck();
                Vec::new()
            }
            ScreenId::C3 => {
                flow.try_again();
                Vec::new()
            }
            // Section 6's "Try again" retries the step that failed.
            _ if failed => flow.primary(),
            _ => Vec::new(),
        },
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
                                enabled: enabled(letter),
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
        ScreenId::C3 => IdScreen::C3,
        ScreenId::C4 => IdScreen::C4,
        ScreenId::R1 => IdScreen::R1,
        ScreenId::R2 => IdScreen::R2,
        ScreenId::R3 => IdScreen::R3,
        ScreenId::R4 => IdScreen::R4,
        ScreenId::K1 => IdScreen::K1,
        ScreenId::S1 => IdScreen::S1,
        ScreenId::S2 => IdScreen::S2,
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
    let (title, paragraphs, primary): (UiText, Vec<UiText>, Option<UiText>) = match screen {
        ScreenId::W => (
            UiText::IdWelcomeTitle,
            vec![UiText::IdWelcomeBody],
            Some(UiText::IdCreate),
        ),
        ScreenId::C1 => (
            UiText::IdC1Title,
            vec![
                UiText::IdC1Point1,
                UiText::IdC1Point2,
                UiText::IdC1Point3,
                UiText::IdC1Point4,
                UiText::IdC1Point5,
            ],
            Some(UiText::IdShowWords),
        ),
        ScreenId::C2 => (
            UiText::IdWordsTitle,
            Vec::new(),
            Some(UiText::IdWrittenDown),
        ),
        ScreenId::C3 => (UiText::IdCheckTitle, vec![UiText::IdCheckBody], None),
        ScreenId::C4 => (
            UiText::IdCreatedTitle,
            vec![UiText::IdCreatedBody],
            Some(UiText::IdContinue),
        ),
        ScreenId::R1 => (
            UiText::IdR1Title,
            vec![UiText::IdR1Body, UiText::IdR1Settings],
            Some(UiText::IdEnterWords),
        ),
        ScreenId::R2 => (UiText::IdR1Title, Vec::new(), Some(UiText::IdCheckWords)),
        ScreenId::R3 => (
            UiText::IdR1Title,
            vec![UiText::IdR3Question],
            Some(UiText::IdR3Yes),
        ),
        ScreenId::R4 => (
            UiText::IdRestoredTitle,
            vec![UiText::IdCreatedBody],
            Some(UiText::IdContinue),
        ),
        ScreenId::K1 => {
            let reason = match flow.recovery().map(|(reason, _)| reason) {
                Some(RecoveryReason::Record) => UiText::IdQReasonRecord,
                _ => UiText::IdQReasonKey,
            };
            // With no PeerId on the record there is nothing to restore
            // against: the reason and the way out only (section 3.4).
            let primary = flow.can_restore().then_some(UiText::IdEnterPhrase);
            (UiText::IdQTitle, vec![reason], primary)
        }
        ScreenId::S1 => (
            UiText::IdS1Title,
            vec![UiText::IdS1Body],
            Some(UiText::IdS1Unlock),
        ),
        ScreenId::S2 => (UiText::IdWordsTitle, Vec::new(), Some(UiText::IdDone)),
        ScreenId::Done => return page,
    };
    page.title = text(title).into();
    // Last, after what the screen is for: the statements the flow owes
    // before R2 takes a word -- ADR-0043's warning (item 11), then the
    // identities' new keys (item 12) -- on whichever screen the model
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
    let peer_label = match screen {
        ScreenId::C2 | ScreenId::S2 => Some(UiText::IdPeerIdLabel),
        ScreenId::C4 | ScreenId::R4 => Some(UiText::IdPeerIdLabelShown),
        ScreenId::R3 => Some(UiText::IdDerivedPeerId),
        _ => None,
    };
    if let (Some(label), Some(peer)) = (peer_label, flow.peer()) {
        page.peer_label = text(label).into();
        page.peer_groups = grouped(peer).into();
    }
    if let Some((_, Some(expected))) = flow.recovery() {
        page.peer_label = text(UiText::IdExpectedPeerId).into();
        page.peer_groups = grouped(expected).into();
    }

    match flow.notice() {
        Some(Notice::ChecksumFailed) => page.notice = text(UiText::IdChecksumFailed).into(),
        Some(Notice::OtherIdentity { derived, expected }) => {
            page.notice = text(UiText::IdOtherIdentity).into();
            page.peer_label = text(UiText::IdDerivedPeerId).into();
            page.peer_groups = grouped(derived).into();
            page.second_peer_label = text(UiText::IdExpectedPeerId).into();
            page.second_peer_groups = grouped(expected).into();
        }
        Some(Notice::VoidPaper) => page.notice = text(UiText::IdVoidPaper).into(),
        Some(failed @ (Notice::Busy | Notice::StorageFailed)) => {
            page.notice = text(if *failed == Notice::Busy {
                UiText::IdBusy
            } else {
                UiText::IdStorageFailed
            })
            .into();
            page.secondary = text(UiText::TryAgain).into();
        }
        None => {}
    }

    page.chips_taken = flow.chips_taken().cast_signed();
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
    if let Some(entry) = flow.entry() {
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
    }

    match screen {
        ScreenId::W if page.secondary.is_empty() => {
            page.secondary = text(UiText::IdRestore).into();
        }
        ScreenId::R3 if page.secondary.is_empty() => {
            page.secondary = text(UiText::IdR3No).into();
            page.footnote = text(UiText::IdR3NoPeerId).into();
        }
        ScreenId::K1 => {
            page.heading_key = K1_HEADING.into();
            page.footnote = text(UiText::IdQLost).into();
        }
        ScreenId::S1 if flow.waiting() => page.waiting = text(UiText::IdGoingOffline).into(),
        _ => {}
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;
    use interweave_human_ui_model::{Start, StepFailed, Words};
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
        let flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: true });
        let page = page(&flow);
        assert_eq!(page.title, en(UiText::IdWelcomeTitle));
        assert_eq!(page.primary, en(UiText::IdCreate));
        assert_eq!(page.secondary, en(UiText::IdRestore));
        assert_eq!(page.notice, en(UiText::IdVoidPaper));
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
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
        let mut fresh = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        assert!(!shows(&page(&fresh)), "Welcome restores nothing");
        let r1 = page_of(&mut fresh, &[IdentityInput::Secondary]);
        assert_eq!(r1.paragraphs.row_data(1), Some(en(UiText::IdR1Settings)));
        assert!(shows(&r1));
        let r2 = page_of(&mut fresh, &[IdentityInput::Primary]);
        assert!(!shows(&r2));

        let k1 = |expected| {
            page(&IdentityFlow::new(
                LIST,
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

        let mut fresh = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        let r1 = page_of(&mut fresh, &[IdentityInput::Secondary]);
        assert_eq!(tail(&r1), both);
        let r2 = page_of(&mut fresh, &[IdentityInput::Primary]);
        assert!(!shows_new_keys(&r2));

        let k1 = |new_keys| {
            page(&IdentityFlow::new(
                LIST,
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

    #[test]
    fn c2_shows_24_numbered_words_and_the_peer_id_to_write_down() {
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        apply(&mut flow, IdentityInput::Primary);
        flow.created(words(), mine(), [3, 9]);
        let page = page_of(&mut flow, &[IdentityInput::Primary]);
        assert_eq!(page.words.row_count(), PHRASE_WORDS);
        let first = page.words.row_data(0).expect("a word");
        assert_eq!((first.number, first.word.as_str()), (1, "acid"));
        assert_eq!(page.peer_label, en(UiText::IdPeerIdLabel));
        assert_eq!(page.peer_groups, grouped(&mine()));
        assert_eq!(page.primary, en(UiText::IdWrittenDown));
    }

    #[test]
    fn the_keypad_disables_dead_letters_and_r2_checks_only_when_complete() {
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
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
        assert_eq!(failed.secondary, en(UiText::TryAgain));
        let effects = apply(&mut flow, IdentityInput::Secondary);
        assert!(
            matches!(effects.as_slice(), [Effect::Check(_)]),
            "Try again retries: {effects:?}"
        );
    }

    /// Welcome with the lock refused: the notice and Try again, which
    /// asks for the lock again rather than opening Restore (#309 F4).
    #[test]
    fn welcome_unheld_shows_busy_and_try_again_asks_for_the_lock() {
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        flow.held(Err(StepFailed::Busy));
        let page = page(&flow);
        assert_eq!(page.notice, en(UiText::IdBusy));
        assert_eq!(page.secondary, en(UiText::TryAgain));
        assert_eq!(apply(&mut flow, IdentityInput::Secondary), [Effect::Hold]);
        assert_eq!(flow.screen(), ScreenId::W);
    }

    /// S1's primary raises the system prompt: it has a key of its own,
    /// never C4's and R4's Continue, which moves on (the owner, on the
    /// Pixel; language-culture-en, REPLY 01a15d22).
    #[test]
    fn s1_asks_to_unlock_where_c4_continues() {
        let flow = IdentityFlow::new(LIST, Start::ShowPhrase);
        assert_eq!(page(&flow).primary, en(UiText::IdS1Unlock));
        assert_ne!(en(UiText::IdS1Unlock), en(UiText::IdContinue));

        let flow = at_c4();
        assert_eq!(page(&flow).primary, en(UiText::IdContinue));
    }

    fn at_c4() -> IdentityFlow {
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        apply(&mut flow, IdentityInput::Primary);
        flow.created(words(), mine(), [3, 9]);
        apply(&mut flow, IdentityInput::Primary);
        apply(&mut flow, IdentityInput::Primary);
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
        let mut flow = IdentityFlow::new(LIST, Start::ShowPhrase);
        apply(&mut flow, IdentityInput::Primary);
        flow.prompted(true);
        let page = page(&flow);
        assert_eq!(page.waiting, en(UiText::IdGoingOffline));
    }

    /// The app bar's arrow is drawn inactive while Back waits: S1 going
    /// offline (acceptance 15).
    #[test]
    fn the_back_arrow_is_inactive_while_the_page_waits() {
        let mut flow = IdentityFlow::new(LIST, Start::ShowPhrase);
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
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
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
}
