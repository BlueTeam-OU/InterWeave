---
role: "rust-ui-dev"
class: solution
topic: "show-source-measured-every-render"
description: "ui-slint rows build and measure source_pieces at every render whatever show-source is; a test asserting row data cannot prove Show source drew"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1b2368e75ec5e748
---

## ui-slint rows build and measure source_pieces at every render whatever show-source is; a test asserting row data cannot prove Show source drew

In crates/human/ui-slint, View::render builds every message row with drawn_body,
which always computes source_pieces and measures them through the hidden
measure-text (code font), whether or not the row's show-source is on. So a test
reading row_data(..).source_pieces passes with the Show source toggle removed,
and a removed replacement panics in the shaper at the first render, before any
menu opens (backtrace: render_messages -> drawn_body -> Measures::height ->
get_measured_height). To prove the source is DRAWN, find the
`for piece in row.source-pieces: Text` elements by accessible label
(Drawn.spoken = cluster::spoken with ReplacedCharacter), absent before the
choice and present after. Found by PR #675's blind review (F1); fixed in
tests/source_draw.rs (6b1acbd07).

*Observed 2026-11-15 (rust-ui-dev)*
