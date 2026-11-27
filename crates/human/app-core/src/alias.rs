// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The alias a new identity is given (`design/2026-11-04-identities.md`
//! §6.2, "The alias"; the grammar is ADR-0067 decision 3's, profile-config's
//! [`EndpointAlias`]): made from the name the person typed, never asked
//! for, and never changed when the card's name is.
//!
//! The rule: the name lowercased; its ASCII letters and digits kept, with
//! no transliteration; every other run of characters one `-`, and a `-` at
//! either end dropped. Empty is `identity`; a leading digit is prefixed
//! `identity-`. Where the profile already holds that alias, `-2`, `-3` …
//! is added, the lowest free one, the text before it cut so the whole
//! still fits the grammar's 64 bytes.

use interweave_profile_config::{AliasError, EndpointAlias};

/// What an alias is when the name gives nothing the grammar keeps.
const FALLBACK: &str = "identity";

/// The alias for a new identity named `name`, given the aliases the
/// profile already holds: the name's alias by the rule above, with the
/// lowest suffix from `-2` that no alias in `in_use` has.
///
/// Only ASCII letters are kept, and lowercased as ASCII: a character
/// whose Unicode lowercase is an ASCII letter -- the Kelvin sign, a dotted
/// capital I -- is not one, since keeping it would be the transliteration
/// the rule excludes (`the_alias_follows_the_names_rule`).
///
/// # Errors
/// The grammar's refusal of the text made, which the rule cannot produce
/// (`every_alias_made_is_one_the_grammar_takes`): passed on rather than
/// replaced by another alias.
pub fn alias_for(name: &str, in_use: &[EndpointAlias]) -> Result<EndpointAlias, AliasError> {
    let base = base_of(name);
    // Ends: `in_use` is finite and the candidates past the bare one each
    // end in their own suffix.
    let mut n = 1_usize;
    loop {
        let candidate = with_suffix(&base, n);
        if !in_use.iter().any(|a| a.as_str() == candidate) {
            return EndpointAlias::parse(candidate);
        }
        n += 1;
    }
}

/// The name's alias before any suffix, uncut: ASCII letters and digits,
/// every other run one `-`, no `-` at either end, `identity` for nothing
/// and `identity-` before a digit.
fn base_of(name: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('-');
            }
            gap = false;
            out.push(c.to_ascii_lowercase());
        } else {
            gap = true;
        }
    }
    if out.is_empty() {
        FALLBACK.to_owned()
    } else if out.starts_with(|c: char| c.is_ascii_digit()) {
        format!("{FALLBACK}-{out}")
    } else {
        out
    }
}

/// `base` with the `n`th suffix (none for 1, `-n` from 2), the base cut so
/// the whole fits [`EndpointAlias::MAX_BYTES`] and a `-` the cut leaves at
/// its end dropped.
fn with_suffix(base: &str, n: usize) -> String {
    let suffix = if n <= 1 {
        String::new()
    } else {
        format!("-{n}")
    };
    let room = EndpointAlias::MAX_BYTES.saturating_sub(suffix.len());
    // The base is ASCII, so any byte index is a char boundary.
    let cut = base.get(..room.min(base.len())).unwrap_or(base);
    format!("{}{suffix}", cut.trim_end_matches('-'))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn alias(text: &str) -> EndpointAlias {
        EndpointAlias::parse(text).expect("an alias")
    }

    fn made(name: &str, in_use: &[&str]) -> String {
        let in_use: Vec<EndpointAlias> = in_use.iter().map(|a| alias(a)).collect();
        alias_for(name, &in_use)
            .expect("an alias")
            .as_str()
            .to_owned()
    }

    /// The design's own examples, and the edges of its rule. Mutation:
    /// keeping a character whose lowercase is an ASCII letter
    /// (`char::to_lowercase` before the ASCII test) -- the Kelvin sign
    /// gives `k`, the transliteration the rule excludes.
    #[test]
    fn the_alias_follows_the_names_rule() {
        assert_eq!(made("Anna Rossi", &[]), "anna-rossi");
        assert_eq!(made("Anna Rossi", &["anna-rossi"]), "anna-rossi-2");
        assert_eq!(made("Ελένη", &[]), "identity");
        assert_eq!(made("Ελένη", &["identity"]), "identity-2");
        assert_eq!(made("!?. --", &[]), "identity", "punctuation alone");
        assert_eq!(made("2024 team", &[]), "identity-2024-team");
        assert_eq!(
            made("  Anna -- Rossi!  ", &[]),
            "anna-rossi",
            "runs and ends"
        );
        assert_eq!(made("José Ñoño", &[]), "jos-o-o", "no transliteration");
        assert_eq!(made("\u{212A}im", &[]), "im", "the Kelvin sign is not k");
        assert_eq!(
            made("\u{0130}van", &[]),
            "van",
            "a dotted capital I is not i"
        );
    }

    /// Mutations: the base never cut -- a 70-letter name gives 70 bytes and
    /// the grammar refuses it; the `-` a cut leaves not dropped -- `--2`.
    #[test]
    fn a_long_name_is_cut_so_the_suffix_fits_64() {
        let long = "a".repeat(70);
        let first = made(&long, &[]);
        assert_eq!(first, "a".repeat(64));
        let second = made(&long, &[&first]);
        assert_eq!(second, format!("{}-2", "a".repeat(62)));
        assert_eq!(second.len(), 64);
        // A cut that ends at a `-` drops it: no `--2`.
        let hyphened = format!("{}-bc", "a".repeat(61));
        assert_eq!(made(&hyphened, &[]), hyphened, "64 bytes, kept whole");
        assert_eq!(
            made(&hyphened, &[&hyphened]),
            format!("{}-2", "a".repeat(61))
        );
    }

    /// Mutation: the highest suffix plus one -- `-4` is made where `-2`
    /// is free.
    #[test]
    fn the_suffix_is_the_lowest_free_one() {
        assert_eq!(
            made("Anna", &["anna", "anna-3", "work"]),
            "anna-2",
            "a gap is filled"
        );
        assert_eq!(made("Anna", &["anna", "anna-2", "anna-3"]), "anna-4");
        assert_eq!(made("Anna", &["anna-2"]), "anna", "the bare one first");
        let mut taken = vec!["identity".to_owned()];
        taken.extend((2..=64).map(|n| format!("identity-{n}")));
        let taken: Vec<&str> = taken.iter().map(String::as_str).collect();
        assert_eq!(made("", &taken), "identity-65", "past a full profile");
    }

    /// Every alias the rule can make is one the grammar takes, and none is
    /// one already in use, over names of every class and length with more
    /// than a profile's worth taken. Mutations: the `identity-` prefix
    /// before a digit left out -- `9` gives an alias the grammar refuses;
    /// `to_ascii_lowercase` left out -- `ANNA` does.
    #[test]
    fn every_alias_made_is_one_the_grammar_takes() {
        let names = [
            String::new(),
            "Anna Rossi".to_owned(),
            "ANNA".to_owned(),
            "9".to_owned(),
            "-".to_owned(),
            "Ελένη".to_owned(),
            "a.b_c".to_owned(),
            "x".repeat(200),
            "9".repeat(200),
            format!("{}-{}", "a".repeat(63), "b".repeat(10)),
            "Ünïcödé 名前 🙂 tail".to_owned(),
        ];
        let mut in_use = Vec::new();
        for name in &names {
            for _ in 0..66 {
                let made = alias_for(name, &in_use).expect("an alias");
                let text = made.as_str().to_owned();
                assert_eq!(
                    EndpointAlias::parse(text.clone()).as_ref(),
                    Ok(&made),
                    "{name:?} gave {text:?}"
                );
                assert!(!in_use.contains(&made), "{text:?} was already in use");
                in_use.push(made);
            }
        }
    }
}
