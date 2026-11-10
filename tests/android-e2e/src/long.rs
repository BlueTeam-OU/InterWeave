// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The long-message shapes design/2026-11-04-long-message.md section 10
//! runs on a phone, each "the largest whose envelope fits the 196 608-byte
//! ceiling": grown by whole units for as long as the sender's own encoder
//! (`encode_outbound`, raw or brotli under the payload limit) takes the
//! envelope that carries it.
//!
//! In the order a driver stepping by TAB meets them cheapest: a row's link
//! controls follow its Read the whole message, so each shape's link
//! controls are crossed on the way to the next shape's, and the two shapes
//! whose previews hold many go last, the reference link's very many at the
//! end.

use interweave_human_chat_protocol::{HumanChatV2, MessageKind, encode_outbound};
use interweave_transport_api::MAX_PAYLOAD_BYTES;

/// The table's rows: the envelope's table bound (`MAX_TABLE_ROWS`), its
/// header included.
const TABLE_ROWS: usize = 256;

/// One shape: a body of `unit` repeated, the count the largest that
/// fits, between `head` and `tail`.
struct Grown {
    name: &'static str,
    head: String,
    unit: &'static str,
    tail: String,
}

/// The shapes, by name, each at its largest.
#[must_use]
pub fn shapes() -> Vec<(&'static str, String)> {
    let w50 = "abcdefghij".repeat(5);
    let grown = [
        Grown {
            name: "paragraph",
            head: String::new(),
            unit: "word ",
            tail: "end".to_owned(),
        },
        Grown {
            name: "list",
            head: String::new(),
            unit: "- a\n",
            tail: "- z".to_owned(),
        },
        Grown {
            name: "code",
            head: "```\n".to_owned(),
            unit: "abcdefghijabcdefghijabcdefghijabcdefghijabcdefghij\n",
            tail: "```".to_owned(),
        },
        Grown {
            name: "headings",
            head: String::new(),
            unit: "# abcdefghijabcdefghijabcdefghijabcdefghijabcdefghij\n\n",
            tail: format!("# {w50}"),
        },
        Grown {
            name: "images",
            head: String::new(),
            unit: "![](https://e.org/x) ",
            tail: "end".to_owned(),
        },
        Grown {
            name: "padded link",
            head: "[a](https://e.org/".to_owned(),
            unit: "\u{200B}",
            tail: "x)".to_owned(),
        },
        Grown {
            name: "heading autolinks",
            head: String::new(),
            unit: "# <https://example.org/pppppppppppppppppppp>\n\n",
            tail: "end".to_owned(),
        },
        Grown {
            name: "reference link",
            head: String::new(),
            unit: "[a][r] ",
            tail: "\n\n[r]: https://e.org/x".to_owned(),
        },
    ];
    let mut out: Vec<(&'static str, String)> = grown
        .iter()
        .map(|g| {
            (
                g.name,
                largest(|n| format!("{}{}{}", g.head, g.unit.repeat(n), g.tail)),
            )
        })
        .collect();
    // The table at its row bound, its cells as wide as fits: rows of two
    // cells, wrapped as a paragraph's line is (A3's note).
    let table = largest(|n| {
        let cell = "a".repeat(n);
        let row = format!("| {cell} | {cell} |\n");
        format!("| h | i |\n|---|---|\n{}", row.repeat(TABLE_ROWS - 1))
    });
    out.insert(3, ("table", table));
    out
}

/// The body `body(n)` for the largest `n` whose envelope the sender
/// encodes, found by doubling and then halving: `fits` holds for every
/// smaller `n` of these shapes, since each grows by appending.
fn largest(body: impl Fn(usize) -> String) -> String {
    let fits = |n: usize| encode_outbound(&envelope(body(n)), MAX_PAYLOAD_BYTES).is_ok();
    assert!(fits(1), "one unit of a shape is over the ceiling");
    let mut high = 2;
    while fits(high) {
        high *= 2;
    }
    let mut low = high / 2;
    while high - low > 1 {
        let mid = low + (high - low) / 2;
        if fits(mid) {
            low = mid;
        } else {
            high = mid;
        }
    }
    body(low)
}

/// The envelope a body goes in, as the desktop's facade sends it: what
/// is measured is what crosses.
#[must_use]
pub fn envelope(text: String) -> HumanChatV2 {
    HumanChatV2 {
        v: 2,
        kind: MessageKind::Text,
        app_message_id: format!("{:032x}", u128::MAX),
        text,
        reply_to: None,
        sent_at_ms: Some(1_762_000_000_000),
        from_endpoint: Some(crate::human()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_shape_is_the_largest_its_envelope_fits() {
        let shapes = shapes();
        let names: Vec<_> = shapes.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            [
                "paragraph",
                "list",
                "code",
                "table",
                "headings",
                "images",
                "padded link",
                "heading autolinks",
                "reference link"
            ]
        );
        for (name, text) in shapes {
            assert!(
                encode_outbound(&envelope(text.clone()), MAX_PAYLOAD_BYTES).is_ok(),
                "{name} fits"
            );
            // Far past the 2 000 characters of the cut as source; whether
            // the row cuts it drawn is the next test's.
            assert!(
                text.chars().count() > 10_000,
                "{name}: {} characters",
                text.chars().count()
            );
        }
    }

    /// The padded link draws one character: its label is past the label
    /// bound, so the link has no control and its text "a" is drawn inert
    /// (`body::has_control`). The row shows it whole and the device case
    /// has no Read the whole message to press for it; every other shape
    /// is cut. The view's image text, as `tests/long_message.rs` gives it.
    #[test]
    fn the_row_cuts_every_shape_but_the_padded_link() {
        use interweave_human_ui_model::{UiText, cut, fill, placeholder_en};
        let image = |alt: &str| {
            if alt.trim().is_empty() {
                placeholder_en::text(UiText::ImageNotShownWithoutAlt).to_owned()
            } else {
                fill(placeholder_en::text(UiText::ImageNotShown), &[("alt", alt)])
            }
        };
        let whole: Vec<_> = shapes()
            .into_iter()
            .filter(|(_, text)| {
                let rendered = interweave_human_chat_protocol::render(text);
                cut::formatted(&rendered, &image).shown_percent.is_none()
            })
            .map(|(name, _)| name)
            .collect();
        assert_eq!(whole, ["padded link"]);
    }

    #[test]
    fn one_more_unit_does_not_fit() {
        let body = |n: usize| "word ".repeat(n);
        let found = largest(body);
        let n = found.len() / "word ".len();
        assert!(encode_outbound(&envelope(body(n)), MAX_PAYLOAD_BYTES).is_ok());
        assert!(encode_outbound(&envelope(body(n + 1)), MAX_PAYLOAD_BYTES).is_err());
    }
}
