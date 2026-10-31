// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! When two names are the same name (contact-cards design 5.4.1): the
//! three comparison keys of a name and the rule over them. Here, beside
//! the identifiers, because the store asks it of a card against its
//! contacts by indexed columns (STATE.md, schema v12) and the client's
//! model asks it of two contacts for their titles (`human-client-ui.md`
//! §13 bullet 9), and neither may pull the other in. One rule, so the
//! warning and the titles can never disagree on whether two names are
//! the same.
//!
//! A key is for comparison: never shown, and never stored in place of the
//! name it was made from.

use unicode_normalization::UnicodeNormalization as _;

/// A name's three keys. Two names are the same when any one key is equal
/// to the same key of the other ([`NameKeys::same_as`]). The rule is not
/// transitive ("Ian" is the same name as "ian" and as "lan", which are
/// not the same name), so it is asked one pair at a time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NameKeys {
    /// NFKC, spaces, case fold, skeleton: catches a lookalike of another
    /// case and every compatibility spelling ("Ａnna", "Anna Rossi" with a
    /// no-break space).
    pub case: String,
    /// Spaces, skeleton, case fold, skeleton: reads the name as written,
    /// so a capital lookalike the fold would turn un-Latin ("Βob" with a
    /// Greek Beta) or a letter NFKC would rewrite ("Alſie") still meets
    /// its Latin twin.
    pub shape: String,
    /// NFKC, then the shape key's steps: a character that looks Latin only
    /// once NFKC has rewritten it ("J０hn" with a fullwidth zero).
    pub compat_shape: String,
}

impl NameKeys {
    /// The three keys of `name`.
    #[must_use]
    pub fn of(name: &str) -> Self {
        let nfkc: String = name.nfkc().collect();
        Self {
            case: case_key(name),
            shape: shape_key(name),
            compat_shape: shape_key(&nfkc),
        }
    }

    /// Whether the names these keys were made from are the same name:
    /// any one key equal, each compared only with its own kind.
    #[must_use]
    pub fn same_as(&self, other: &Self) -> bool {
        self.case == other.case
            || self.shape == other.shape
            || self.compat_shape == other.compat_shape
    }
}

/// Whether `a` and `b` are the same name (design 5.4.1).
#[must_use]
pub fn same_name(a: &str, b: &str) -> bool {
    NameKeys::of(a).same_as(&NameKeys::of(b))
}

/// The case key: NFKC, spaces, full case folding, then Unicode TS #39's
/// confusable skeleton.
#[must_use]
pub fn case_key(name: &str) -> String {
    let nfkc: String = name.nfkc().collect();
    skeleton(&caseless::default_case_fold_str(&spaces(&nfkc)))
}

/// The shape key: spaces, skeleton, full case folding, skeleton again.
fn shape_key(name: &str) -> String {
    skeleton(&caseless::default_case_fold_str(&skeleton(&spaces(name))))
}

/// Each run of white space one space, the ends trimmed. White space is
/// Rust's `White_Space`, a superset of the design's Zs and tab; a card's
/// name holds no other (HUMAN-CHAT.md refuses controls in it).
fn spaces(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn skeleton(text: &str) -> String {
    unicode_security::skeleton(text).collect()
}

#[cfg(test)]
mod tests {
    use super::{NameKeys, case_key, same_name};

    /// Design s9 item 7's pairs, the store's test vectors as well
    /// (STATE.md, schema v12), with the keys that join each. Every pair
    /// the design joins "by the shape key" escapes the case key; the
    /// compatibility shape key joins those NFKC leaves alone too. Each
    /// key has a pair it alone joins, so a key dropped from
    /// [`NameKeys::same_as`] fails here; one compared with another kind
    /// fails `a_key_equal_to_another_kind_alone_joins_nothing`.
    /// Mutations: each disjunct of `same_as` removed in turn;
    /// `compat_shape` made without NFKC.
    #[test]
    fn each_key_joins_the_pairs_the_design_says() {
        type Pick = fn(&NameKeys) -> &str;
        let case: Pick = |k| &k.case;
        let shape: Pick = |k| &k.shape;
        let compat: Pick = |k| &k.compat_shape;
        let keys: [(&str, Pick); 3] = [("case", case), ("shape", shape), ("compat", compat)];
        for (a, b, joins) in [
            ("\u{a4ee}da", "Ada", &["shape", "compat"][..]),
            ("\u{13a0}ave", "Dave", &["shape", "compat"]),
            ("\u{a4df}ax", "Max", &["shape", "compat"]),
            ("\u{392}ob", "Bob", &["shape", "compat"]),
            ("Al\u{17f}ie", "Alfie", &["shape"]),
            ("Ian", "ian", &["case"]),
            ("J\u{ff10}hn", "John", &["compat"]),
        ] {
            let (ka, kb) = (NameKeys::of(a), NameKeys::of(b));
            for (kind, pick) in keys {
                assert_eq!(
                    pick(&ka) == pick(&kb),
                    joins.contains(&kind),
                    "{a:?}/{b:?}: the {kind} key"
                );
            }
            assert!(same_name(a, b), "{a:?}/{b:?}");
        }
    }

    /// Each key is compared only with its own kind: keys equal across
    /// kinds alone join nothing, in every arrangement of the three, and the
    /// same keys in their own places join. The design's pairs cannot hold
    /// this, since some of them have one name's shape key equal to the
    /// other's case key ("Alſie"/"Alfie"). Mutations: `self.shape` compared
    /// with `other.case`; `self.compat_shape` with `other.shape`.
    #[test]
    fn a_key_equal_to_another_kind_alone_joins_nothing() {
        let keys = |case: &str, shape: &str, compat_shape: &str| NameKeys {
            case: case.to_owned(),
            shape: shape.to_owned(),
            compat_shape: compat_shape.to_owned(),
        };
        let a = keys("x", "y", "z");
        for b in [
            keys("y", "z", "x"),
            keys("z", "x", "y"),
            keys("y", "x", "q"),
            keys("z", "q", "x"),
            keys("q", "z", "y"),
        ] {
            assert!(!a.same_as(&b), "{a:?} against {b:?}");
            assert!(!b.same_as(&a), "{b:?} against {a:?}");
        }
        for b in [
            keys("x", "q", "q"),
            keys("q", "y", "q"),
            keys("q", "q", "z"),
        ] {
            assert!(a.same_as(&b), "the control: {b:?}");
        }
    }

    /// The controls: different names no key joins.
    #[test]
    fn different_names_are_not_the_same_name() {
        for (a, b) in [("Mia", "Max"), ("Hanna Rossi", "Anna Rossi")] {
            assert!(!same_name(a, b), "{a:?}/{b:?}");
        }
    }

    /// The design's named spellings of "Anna" and "Anna Rossi", by the
    /// rule as a whole.
    #[test]
    fn the_designs_lookalikes_are_the_same_name() {
        for (a, b) in [
            ("\u{410}nna", "Anna"),
            ("Anna\u{a0}Rossi", "Anna Rossi"),
            ("\u{ff21}nna", "Anna"),
            (" Anna \t Rossi ", "Anna Rossi"),
        ] {
            assert!(same_name(a, b), "{a:?}/{b:?}");
        }
    }

    /// The rule is not transitive, as the design says, so nothing may
    /// group by it: "Ian" is "ian" and "lan", which are not each other.
    #[test]
    fn the_rule_is_not_transitive() {
        assert!(same_name("Ian", "ian"));
        assert!(same_name("Ian", "lan"));
        assert!(!same_name("ian", "lan"));
    }

    /// The case key is the store's `name_key` of schema v10: its output
    /// for the names the store pinned at version 1 is unchanged.
    #[test]
    fn the_case_key_is_version_ones_key() {
        for (name, key) in [
            ("  Ada   Lovelace ", "ada lovelace"),
            ("\u{ff21}da", "ada"),
            ("Stra\u{df}e", "strasse"),
            ("\u{3a9}mega", "\u{3c9}rnega"),
        ] {
            assert_eq!(case_key(name), key, "{name:?}");
        }
    }
}
