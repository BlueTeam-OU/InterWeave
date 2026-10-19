// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The recovery-phrase word picker's state (design/2026-10-19-android-
//! identity.md §4): one position at a time, entered by letters from an
//! in-app keypad, narrowed to the words of a list, and filled only by the
//! person's tap on a word. Pure: no IME, no text field and no clipboard
//! exist here, which is the point (android-key-custody.md's recovery-screen
//! rules) -- the view draws keys, chips and positions from this and
//! returns taps.
//!
//! The word list is the caller's: the platform hands in BIP-39's English
//! list, sorted as BIP-39 publishes it, and the tests a short one. Nothing
//! here keeps a word beyond the [`PhraseEntry`] that holds it, and a
//! `Debug` of either prints no word: a log line or a panic message must
//! never carry one (the design's acceptance item 16).

use std::fmt;

/// Words in a recovery phrase (ADR-0033).
pub const PHRASE_WORDS: usize = 24;

/// At most this many matching words show as chips; above it, the count
/// shows instead, so no row of arbitrary words invites a wrong tap (§4).
pub const MAX_CHIPS: usize = 4;

/// What the suggestions row shows for the letters typed so far.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Suggestions {
    /// More than [`MAX_CHIPS`] words match: how many, and the letters
    /// typed as a chip when they are a whole word that also begins the
    /// others -- BIP-39's act, can, car, cat, man and win, which no fourth
    /// letter could reach otherwise (design §4, #303 review F1).
    Count {
        /// How many words match.
        n: usize,
        /// The typed letters, when they are a word.
        exact: Option<&'static str>,
    },
    /// These words match, each a chip, in list order.
    Words(&'static [&'static str]),
}

impl fmt::Debug for Suggestions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Count { n, exact } => {
                write!(f, "Count({n}, exact: {})", exact.is_some())
            }
            Self::Words(words) => write!(f, "Words(<{} redacted>)", words.len()),
        }
    }
}

/// One position's entry: the letters typed, narrowed against `words`.
pub struct WordPicker {
    words: &'static [&'static str],
    typed: String,
}

impl WordPicker {
    /// A picker over `words`, which must be sorted, as BIP-39's lists are;
    /// a sorted list is what makes the matches one contiguous range.
    #[must_use]
    pub fn new(words: &'static [&'static str]) -> Self {
        debug_assert!(words.is_sorted(), "a picker's list is sorted");
        Self {
            words,
            typed: String::new(),
        }
    }

    /// The letters typed so far, for the letters display (not a field).
    #[must_use]
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// The words the letters typed so far begin.
    #[must_use]
    pub fn matches(&self) -> &'static [&'static str] {
        Self::matching(self.words, &self.typed)
    }

    fn matching(words: &'static [&'static str], prefix: &str) -> &'static [&'static str] {
        let start = words.partition_point(|word| *word < prefix);
        let len = words[start..].partition_point(|word| word.starts_with(prefix));
        &words[start..start + len]
    }

    /// Whether `letter` continues some word: a key no word continues is
    /// shown disabled, never removed (§4).
    #[must_use]
    pub fn enabled(&self, letter: char) -> bool {
        let mut prefix = self.typed.clone();
        prefix.push(letter);
        !Self::matching(self.words, &prefix).is_empty()
    }

    /// Type `letter`. A letter no word continues is refused, so the typed
    /// letters always begin at least one word.
    pub fn press(&mut self, letter: char) -> bool {
        if !letter.is_ascii_lowercase() || !self.enabled(letter) {
            return false;
        }
        self.typed.push(letter);
        true
    }

    /// Delete the last letter typed.
    pub fn delete(&mut self) {
        self.typed.pop();
    }

    /// Clear the position's letters.
    pub fn clear(&mut self) {
        self.typed.clear();
    }

    /// What the suggestions row shows. Nothing is ever chosen here: a
    /// single match is still a chip the person taps (§4).
    #[must_use]
    pub fn suggestions(&self) -> Suggestions {
        let matches = self.matches();
        if matches.len() > MAX_CHIPS {
            // A sorted list puts the whole word first among its matches.
            let exact = matches.first().copied().filter(|w| *w == self.typed);
            Suggestions::Count {
                n: matches.len(),
                exact,
            }
        } else {
            Suggestions::Words(matches)
        }
    }
}

impl WordPicker {
    /// `chip`, if it is one the row shows now: a suggestion, or the
    /// whole-word chip beside a count.
    #[must_use]
    pub fn chip(&self, chip: &str) -> Option<&'static str> {
        match self.suggestions() {
            Suggestions::Words(words) => words.iter().copied().find(|w| *w == chip),
            Suggestions::Count { exact, .. } => exact.filter(|w| *w == chip),
        }
    }
}

impl fmt::Debug for WordPicker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WordPicker(<{} letters redacted>)", self.typed.len())
    }
}

/// The whole phrase being entered: 24 positions, the one being edited,
/// and its picker.
pub struct PhraseEntry {
    positions: [Option<&'static str>; PHRASE_WORDS],
    current: usize,
    picker: WordPicker,
}

impl PhraseEntry {
    /// An empty phrase over `words`, editing position 1.
    #[must_use]
    pub fn new(words: &'static [&'static str]) -> Self {
        Self {
            positions: [None; PHRASE_WORDS],
            current: 0,
            picker: WordPicker::new(words),
        }
    }

    /// The position being edited, from 0.
    #[must_use]
    pub fn current(&self) -> usize {
        self.current
    }

    /// Its picker.
    #[must_use]
    pub fn picker(&self) -> &WordPicker {
        &self.picker
    }

    /// Its picker, for the keypad's taps.
    pub fn picker_mut(&mut self) -> &mut WordPicker {
        &mut self.picker
    }

    /// The word at `position`, once filled.
    #[must_use]
    pub fn word(&self, position: usize) -> Option<&'static str> {
        self.positions.get(position).copied().flatten()
    }

    /// The person tapped `chip`: it fills the current position if it is
    /// one of the current suggestions, and the entry moves to the next
    /// empty position (or stays, when none is left). A word that is not a
    /// current chip is refused: only a word the person saw is entered.
    pub fn choose(&mut self, chip: &str) -> bool {
        let Some(word) = self.picker.chip(chip) else {
            return false;
        };
        self.positions[self.current] = Some(word);
        self.picker.clear();
        if let Some(next) = (1..=PHRASE_WORDS)
            .map(|step| (self.current + step) % PHRASE_WORDS)
            .find(|&position| self.positions[position].is_none())
        {
            self.current = next;
        }
        true
    }

    /// The person tapped `position` in the grid: it becomes the current
    /// one, and its word, if any, goes back into the letters to delete or
    /// replace (§4). The word stays in the phrase until it is replaced.
    pub fn edit(&mut self, position: usize) {
        if position >= PHRASE_WORDS {
            return;
        }
        self.current = position;
        self.picker.clear();
        if let Some(word) = self.positions[position] {
            for letter in word.chars() {
                self.picker.press(letter);
            }
        }
    }

    /// Whether every position is filled: the check is enabled (§3.3 R2).
    #[must_use]
    pub fn complete(&self) -> bool {
        self.positions.iter().all(Option::is_some)
    }

    /// The phrase as one line, for the platform to parse and verify; `None`
    /// until complete. The caller drops it as soon as it is parsed.
    #[must_use]
    pub fn phrase(&self) -> Option<String> {
        let words: Option<Vec<&str>> = self.positions.iter().copied().collect();
        words.map(|words| words.join(" "))
    }

    /// Drop every word (leaving the screen, §7).
    pub fn clear(&mut self) {
        self.positions = [None; PHRASE_WORDS];
        self.current = 0;
        self.picker.clear();
    }
}

impl fmt::Debug for PhraseEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let filled = self.positions.iter().filter(|word| word.is_some()).count();
        write!(
            f,
            "PhraseEntry(<{filled} of {PHRASE_WORDS} filled, redacted>, current {})",
            self.current
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shaped like BIP-39's English list where it matters: sorted, unique
    // by its first four letters, with words sharing longer prefixes.
    const WORDS: &[&str] = &[
        "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd",
        "abuse", "access", "accident", "car", "carbon", "card", "cargo", "carpet", "carry",
        "zebra", "zero", "zone", "zoo",
    ];

    #[test]
    fn letters_narrow_and_dead_letters_are_disabled_not_removed() {
        let mut picker = WordPicker::new(WORDS);
        assert!(picker.enabled('a') && picker.enabled('z'));
        assert!(!picker.enabled('q'), "no word starts with q");
        assert!(!picker.press('q'), "a disabled key types nothing");
        assert_eq!(picker.typed(), "");
        assert!(picker.press('a') && picker.press('b'));
        assert_eq!(picker.matches().len(), 10);
        assert!(picker.enabled('s') && picker.enabled('l') && picker.enabled('u'));
        assert!(!picker.enabled('c'), "no word continues ab with c");
    }

    #[test]
    fn more_than_four_matches_show_a_count_and_four_or_fewer_show_chips() {
        let mut picker = WordPicker::new(WORDS);
        picker.press('a');
        assert_eq!(
            picker.suggestions(),
            Suggestions::Count { n: 12, exact: None }
        );
        for letter in "bs".chars() {
            picker.press(letter);
        }
        assert_eq!(
            picker.suggestions(),
            Suggestions::Words(&["absent", "absorb", "abstract", "absurd"])
        );
        picker.press('o');
        assert_eq!(picker.suggestions(), Suggestions::Words(&["absorb"]));
        picker.delete();
        picker.delete();
        assert_eq!(picker.typed(), "ab");
    }

    #[test]
    fn a_single_match_is_still_a_chip_never_filled_by_itself() {
        let mut entry = PhraseEntry::new(WORDS);
        for letter in "zeb".chars() {
            entry.picker_mut().press(letter);
        }
        assert_eq!(entry.picker().suggestions(), Suggestions::Words(&["zebra"]));
        assert_eq!(entry.word(0), None, "nothing entered until the tap");
        assert!(!entry.choose("zero"), "not a current chip");
        assert!(entry.choose("zebra"));
        assert_eq!(entry.word(0), Some("zebra"));
        assert_eq!(entry.current(), 1);
        assert_eq!(entry.picker().typed(), "");
    }

    #[test]
    fn a_whole_word_that_begins_five_others_is_its_own_chip_beside_the_count() {
        let mut entry = PhraseEntry::new(WORDS);
        for letter in "car".chars() {
            entry.picker_mut().press(letter);
        }
        assert_eq!(
            entry.picker().suggestions(),
            Suggestions::Count {
                n: 6,
                exact: Some("car")
            }
        );
        assert!(!entry.choose("carbon"), "not shown as a chip");
        assert!(entry.choose("car"));
        assert_eq!(entry.word(0), Some("car"));
    }

    #[test]
    fn a_count_row_offers_no_chip_to_choose() {
        let mut entry = PhraseEntry::new(WORDS);
        entry.picker_mut().press('a');
        assert!(!entry.choose("able"), "five or more matches are a count");
        assert_eq!(entry.word(0), None);
    }

    #[test]
    fn editing_a_position_puts_its_word_back_and_the_next_chip_moves_to_an_empty_one() {
        let mut entry = PhraseEntry::new(WORDS);
        for position in 0..PHRASE_WORDS {
            entry.edit(position);
            for letter in "zoo".chars() {
                entry.picker_mut().press(letter);
            }
            assert!(entry.choose("zoo"), "position {position}");
        }
        assert!(entry.complete());
        assert_eq!(
            entry.phrase().map(|p| p.split(' ').count()),
            Some(PHRASE_WORDS)
        );

        entry.edit(6);
        assert_eq!(entry.picker().typed(), "zoo");
        entry.picker_mut().clear();
        for letter in "zon".chars() {
            entry.picker_mut().press(letter);
        }
        assert!(entry.choose("zone"));
        assert_eq!(entry.word(6), Some("zone"));
        assert!(entry.complete(), "a replaced word keeps the phrase whole");
    }

    #[test]
    fn an_incomplete_phrase_gives_nothing_and_clear_drops_every_word() {
        let mut entry = PhraseEntry::new(WORDS);
        for letter in "zeb".chars() {
            entry.picker_mut().press(letter);
        }
        entry.choose("zebra");
        assert_eq!(entry.phrase(), None);
        entry.clear();
        assert_eq!(entry.word(0), None);
        assert_eq!(entry.current(), 0);
    }

    #[test]
    fn debug_output_carries_no_word() {
        let mut entry = PhraseEntry::new(WORDS);
        for letter in "zeb".chars() {
            entry.picker_mut().press(letter);
        }
        let suggestions = format!("{:?}", entry.picker().suggestions());
        entry.choose("zebra");
        for printed in [
            format!("{entry:?}"),
            format!("{:?}", entry.picker()),
            suggestions,
        ] {
            assert!(!printed.contains("zeb"), "{printed}");
        }
    }
}
