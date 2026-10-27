// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An ask of the Activity and its argument in the one `long` that
//! `Native.waitIdentityAsk` returns: the ask in the low [`ASK_BITS`], the
//! argument above them. On every target, so the host tests it, and that
//! Kotlin's `Ask.BITS` says the same width (#483 F5).

/// The bits of a waited ask that name it; the rest carry its argument.
/// Kotlin's `Ask.BITS` (Native.kt) is the same number.
pub(crate) const ASK_BITS: u32 = 8;

/// The largest argument a word carries: bit 63 stays clear, so the word is
/// never negative to Kotlin. An argument is a photo request, counted from 1
/// in the process, which does not come near it.
pub(crate) const MAX_ARG: u64 = (1 << (63 - ASK_BITS)) - 1;

/// `ask` and `arg` as one word; `arg` past [`MAX_ARG`] is sent as 0, which
/// names no request.
pub(crate) fn pack(ask: i32, arg: u64) -> i64 {
    debug_assert!((0..1 << ASK_BITS).contains(&ask), "an ask fits its bits");
    let arg = if arg > MAX_ARG {
        0
    } else {
        i64::try_from(arg).unwrap_or(0)
    };
    (arg << ASK_BITS) | i64::from(ask)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unpacked as Kotlin's `identityAsk` relay does it.
    fn kotlin(word: i64) -> (i32, u64) {
        let mask = (1_i64 << ASK_BITS) - 1;
        let ask = i32::try_from(word & mask).expect("an ask");
        let arg = u64::try_from(word).expect("never negative") >> ASK_BITS;
        (ask, arg)
    }

    /// An ask with no argument is the word it always was, and one with an
    /// argument comes back whole, at the largest too. Mutation: the
    /// argument not shifted.
    #[test]
    fn an_ask_and_its_argument_come_back_as_kotlin_reads_them() {
        assert_eq!(pack(14, 0), 14, "an ask without an argument is unchanged");
        assert_eq!(kotlin(pack(14, 7)), (14, 7));
        assert_eq!(kotlin(pack(15, MAX_ARG)), (15, MAX_ARG));
        assert!(pack(255, MAX_ARG) > 0, "bit 63 clear");
        assert_eq!(
            kotlin(pack(14, MAX_ARG + 1)),
            (14, 0),
            "past it: no request"
        );
    }

    /// Kotlin's width is this one: a change on one side alone fails here,
    /// not on a phone. Mutation: `ASK_BITS` 7.
    #[test]
    fn kotlin_names_the_same_width() {
        let native = include_str!("../android/app/src/main/kotlin/org/interweave/human/Native.kt");
        let line = native
            .lines()
            .map(str::trim)
            .find(|l| l.starts_with("const val BITS = "))
            .expect("Ask.BITS in Native.kt");
        assert_eq!(line, format!("const val BITS = {ASK_BITS}"));
    }
}
