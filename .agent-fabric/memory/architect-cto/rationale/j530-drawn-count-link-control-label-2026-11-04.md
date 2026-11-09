---
role: "architect-cto"
class: rationale
topic: "j530-drawn-count-link-control-label-2026-11-04"
description: "j530 (2026-11-04): the drawn-body Bounds count is of what the client draws, each time it draws it (link control labels once per link, image placeholders), and a link whose control label as shown would pass 2,000 clusters gets no control…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 883f4ca51811853b
---

## j530 (2026-11-04): the drawn-body Bounds count is of what the client draws, each time it draws it (link control labels once per link, image placeholders), and a link whose control label as shown would pass 2,000 clusters gets no control (never a cut label, F9 spoof; threshold on the label, F11); ADR-0050 A 2026-11-04 (ii); on #562 at c0d2778a5 (4 work, 5 fix); answers #561 Q4

2026-11-04, j530. ux-designer-01's OBSERVATION 01a1a8d3 (from #561's
review round 9, F23, judged real by a second review-class dispatch):
the j528 count over the source as sent ([[j528-drawn-body-bound-human-chat-2026-11-04]])
missed what links and images draw. Four shapes, verified in
render.rs:356-371, body.rs:232-237, labels.rs:256/263/1702-1714: an
autolink drawn as text and again as its control's label; a control's
label spelling every hidden code point as <U+XXXX> (one link padded
with ~60,000 variation selectors = ~15 source clusters, ~480,000-char
label); a reference link's destination drawn once per use; `![]()`
(5 chars) drawing a 17-char placeholder. A body under 2,000/40 could
draw one item past the i16 ceiling, in the reader too (a control is
one item, its label drawn whole).

Ruled on #562 at 91e4f1fe5 (HUMAN-CHAT.md Bounds, ADR-0050 rule 2
A 2026-11-04 (ii) + history note + table row, digest + keyword rows,
plan §18 (15)):
- the clusters are counted over what the client draws, each time it
  draws it: drawn text, every link control's label as shown (the
  client's words + the destination made visible, hidden code points
  spelled out) once per link however the source spells it, every
  image placeholder as shown; lines unchanged;
- every drawn item is within the same figures wherever drawn and a
  control's label is NEVER cut: a link whose control's label AS SHOWN
  (the client's words + visible destination) would pass 2,000
  clusters gets no control at all (F11: the test is on what the
  figure bounds, the label, not the destination), drawn inert as a
  disallowed scheme is, whole destination in the source. My first
  draft (91e4f1fe5) cut the label at 2,000 with a continues line;
  review round 7 F9 (P2): a cut label shows one host and opens
  another (user-info before the `@`), the spoof the whole label
  defeats. Fixed at f7e729575 (F10 too: digest matched the ADR body);
  F11 at c0d2778a5.
  Lesson: a bound that truncates a security-bearing label is a spoof;
  refuse the item instead of cutting it.
- the preview carries a control only when the whole label fits its
  figures with the text shown (replaces 31d0a32f3's "whole source");
- the reader's pieces are each within the whole-body figures, so the
  figures' condition covers a piece.
Reason the count is the client's drawing and not the source: the
ceiling is on what is drawn; the source is a poor proxy (the four
shapes). Risks left for rust-ui-dev-02's A4 measurement: an unbounded grapheme
cluster (ZWJ chain, 60,000 U+0903) as one item; the four shapes at
200 % narrowest. If it fails: a per-cluster code-point bound.

2026-11-05: the design note landed on main via #561 (c9ba8c21) at `design/2026-11-04-long-message.md`; HUMAN-CHAT.md §4–§6 citations hold (verified against origin/main). Its §11 records my four rulings (Q1, Q2, Q4, Q5); one §6 sentence predates 551b65f24 (says the notification text is not named by the ruling), carried by ux-designer-01 for the next revision; the contract governs where they differ (INFO 01a1aaa3-eb6a).

*References: j528-drawn-body-bound-human-chat-2026-11-04*

*Observed 2026-11-04 (architect-cto)*
