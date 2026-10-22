// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! One event on the host line is bounded (`contracts/CHANNEL-EVENT.md`
//! §Content, A 2026-10-22): at most [`MAX_HOST_EVENT_BYTES`] as the host
//! receives it.
//!
//! The largest legal payload reaches that only through JSON escaping: a
//! `;ce=br` payload decoding to 196,608 bytes of control characters is
//! about 1.18 MB escaped once, six bytes a character, while its base64url
//! form is 262,144. So a UTF-8 text whose event would pass the bound is
//! carried as base64url instead, the representation rule for non-UTF-8
//! bytes, chosen by size.
//!
//! **The size is the larger of the two modes'.** A push event is its own
//! `notifications/claude/channel` line, escaped once; a pull event is a
//! member of `receive`'s JSON, which travels as the TEXT of a tool result
//! and is escaped again -- `\u0001` becomes `\\u0001`, seven bytes. The
//! contract has a pull event carry exactly what the push would, so one
//! measure decides for both, and it is the larger
//! (`the_choice_is_the_same_in_both_modes`).
//!
//! The shapes below mirror the server's: `apps/claude-channel`'s
//! `notification_line` and `receive` serialize the same fields in the same
//! order, and its tests compare this measure with the lines it writes
//! (`the_measure_is_the_lines_own_length`).

use serde::Serialize;

use crate::meta::ChannelMeta;

/// The most one event may take on the host line, in bytes.
pub const MAX_HOST_EVENT_BYTES: usize = 320 * 1024;

/// The bytes `content` and `meta` take as the host receives them: the
/// push line, or the pull event as escaped inside its tool result,
/// whichever is longer.
#[must_use]
pub fn host_event_bytes(content: &str, meta: &ChannelMeta) -> usize {
    let mut push = Count::default();
    // Serializing plain strings into a counter cannot fail.
    let _ = serde_json::to_writer(
        &mut push,
        &PushLine {
            jsonrpc: "2.0",
            method: "notifications/claude/channel",
            params: PushParams { content, meta },
        },
    );
    let mut pull = Count::default();
    let _ = serde_json::to_writer(
        &mut pull,
        &PullEvent {
            // The longer of the two kinds, so the measure holds for both.
            kind: "broadcast",
            content,
            meta,
        },
    );
    push.raw.max(pull.escaped)
}

#[derive(Serialize)]
struct PushLine<'a> {
    jsonrpc: &'static str,
    method: &'static str,
    params: PushParams<'a>,
}

#[derive(Serialize)]
struct PushParams<'a> {
    content: &'a str,
    meta: &'a ChannelMeta,
}

#[derive(Serialize)]
struct PullEvent<'a> {
    kind: &'static str,
    content: &'a str,
    meta: &'a ChannelMeta,
}

/// Bytes written, and what they take once escaped again as a JSON string
/// body: the measure without building a megabyte to measure it.
#[derive(Default)]
struct Count {
    raw: usize,
    escaped: usize,
}

impl std::io::Write for Count {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.raw = self.raw.saturating_add(buf.len());
        let escaped = buf.iter().map(|&b| escaped_len(b)).sum::<usize>();
        self.escaped = self.escaped.saturating_add(escaped);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// What `serde_json` writes for byte `b` inside a string: the quote, the
/// backslash and the five short control escapes take two bytes, any other
/// control character six (`\u00XX`), every other byte itself -- a UTF-8
/// continuation byte included, since it never needs escaping.
const fn escaped_len(b: u8) -> usize {
    match b {
        b'"' | b'\\' | 0x08 | 0x0c | b'\n' | b'\r' | b'\t' => 2,
        0x00..=0x1f => 6,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::meta::MetaKey;

    fn meta() -> ChannelMeta {
        let mut meta = ChannelMeta::new();
        meta.set(MetaKey::DeliveryMode, "direct").expect("set");
        meta.set(MetaKey::PayloadEncoding, "utf8").expect("set");
        meta
    }

    /// The counter agrees with `serde_json` byte for byte: every byte
    /// value, escaped once and twice.
    #[test]
    fn the_escape_cost_is_serde_jsons() {
        let all: String = (0u8..0x80).map(char::from).collect::<String>() + "é€😀";
        let once = serde_json::to_string(&all).expect("serializes");
        let twice = serde_json::to_string(&once).expect("serializes");
        let mut count = Count::default();
        std::io::Write::write_all(&mut count, once.as_bytes()).expect("counts");
        assert_eq!(count.raw, once.len());
        // `twice` is `once` escaped plus its two quotes.
        assert_eq!(count.escaped + 2, twice.len());
    }

    /// A control character costs six bytes on the push line and seven in
    /// the pull event, so past a few the pull measure decides; a plain
    /// character costs one in either.
    #[test]
    fn the_choice_is_the_same_in_both_modes() {
        let meta = meta();
        let controls = |n| host_event_bytes(&"\u{1}".repeat(n), &meta);
        assert_eq!(controls(2000) - controls(1000), 7000);
        let plain = |n| host_event_bytes(&"a".repeat(n), &meta);
        assert_eq!(plain(2000) - plain(1000), 1000);
    }
}
