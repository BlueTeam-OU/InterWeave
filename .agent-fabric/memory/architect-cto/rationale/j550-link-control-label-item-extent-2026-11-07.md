---
role: "architect-cto"
class: rationale
topic: "j550-link-control-label-item-extent-2026-11-07"
description: "j550 DELIVERED 2026-11-07 (rust-ui-dev-02 OBSERVATION 01a1b725-2e57; REPLY 01a1b72a-de37 sent; record 64fdca2d3 on the for/ branch, A 2026-11-07 (ii)): a link control's label, never cut and under the 2,000-cluster figure, still draws one…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8abd9557d5609089
---

## j550 DELIVERED 2026-11-07 (rust-ui-dev-02 OBSERVATION 01a1b725-2e57; REPLY 01a1b72a-de37 sent; record 64fdca2d3 on the for/ branch, A 2026-11-07 (ii)): a link control's label, never cut and under the 2,000-cluster figure, still draws one text item past the 8,000 px extent (W 8,390 px, U+FDFD 74,823 px at 200 %/360 dp on #583 head dddb6964); my intended ruling: the label is a text run, drawn whole in pieces by measure like any drawn line, its control's ring and fill in pieces too; (a) no-control-by-measure, (b) scroll-inside, (c) known limit declined

OBSERVATION 01a1b725-2e57 (rust-ui-dev-02, 2026-11-07 17:54Z): a link
whose label is 1,979 weighted clusters gets a control (figure 2,000);
the label is one text item, never cut (Bounds, the host-spoof reason):
"x" 5,073 px, "W" 8,390, U+FDFD 74,823 logical px at 200 % text, 360 dp,
desktop debug build of #583 head dddb6964 under Xvfb; body lines and
source pieces of the same build are within the extent by measure. The
control's focus ring and pressed fill not checked separately. They
offer (a) no control when the label MEASURES past the extent, text
inert, (b) a label scrolling inside an item no taller than the
viewport, (c) a known limit; they would build (a); changed nothing.
Gap predates #583 (label rule A 2026-11-04 (ii)).

RULING SENT as REPLY 01a1b72a-de37 (2026-11-07 ~18:1xZ); the ruling:
none of the three. "Never cut" is about what is SHOWN (every cluster
of the label visible, so the person sees the whole destination), not
about how many drawn items show it. The Bounds invariant already lists
"a text run ... the focus ring" among items drawn in pieces, and since
A 2026-11-07 a drawn line past the ceiling is split by its measured
extent: a control's label is such a text run, drawn whole, uncut, in
pieces by measure inside ONE control (one focus stop, one press
target, one accessible label), its ring and any pressed fill in pieces
with it. (a) declined: whether a link is pressable would then depend
on font, width and scale (a control on a tablet, none on a phone, a
rotation changing it), the very dependence the cut was kept free of;
and it takes the link from the person for a renderer's reason. (b)
declined: a label the person must scroll inside before pressing is a
label not seen whole, the spoof the rule exists against. (c) declined:
it is the invariant, not the figures. If their layout cannot reach the
label with the split, that comes back as a finding with why, and (a)
is the fallback to rule then — not before.

Record: one clarifying sentence in HUMAN-CHAT.md Bounds beside the
invariant ("a link control's label included: whole, in pieces by
measure, one control"), ADR-0050 history note A 2026-11-07 gains a
paragraph, no new log row (same amendment, same day) — as a second
commit on develop-qzapp/architect-cto-01/for/rust-ui-dev-02/
pieces-by-measure (0b0b11c1a already FOLDED into #583 at eba07688c;
a second commit folds the same way). #583 (8w,0f, draft) need not
hold for this: the gap is on main; build the label pieces on #583 as
another commit of the batch or record it as carried in the design
note (ux-designer-01's) if the batch closes first.

DELIVERED: 64fdca2d3 on develop-qzapp/architect-cto-01/for/rust-ui-dev-02/
pieces-by-measure (second commit after 0b0b11c1a): HUMAN-CHAT.md Bounds
sentence with the three measurements, ADR-0050 rule 2 phrase, log row
"2026-11-07 (ii)", history section "Amendment 2026-11-07 (ii)" (the (ii)
precedent is its own heading and row, not a paragraph in the first),
digest entry; validate_adr_index OK. #583 MERGED 902495dd on the owner's word before the fold, so 64fdca2d3
travels on BlueTeam-OU/InterWeave#585 (draft, rust-ui-dev-02, folded
unrebased 4caed8b41; label pieces built 0640e1c7b: one control, holder
draws nothing, tiled ring; A4's extent test gains labels of 1,960 W and
1,960 U+FDFD and a cut one in the reader, fails with any part removed —
REPLY 01a1b751-bf55, 2026-11-07 18:42Z). Closes when 64fdca2d3 reaches
origin/main via #585.

**Why:** the invariant is the renderer's geometry and is kept by geometry (pieces by measure), never by changing what the person gets; a count or a measure deciding whether a control exists mixes the two again. See [[bounds-wide-cluster-pieces-by-measure-2026-11-07]], [[j530-drawn-count-link-control-label-2026-11-04]].
**How to apply:** when a drawn thing "must not be cut", ask whether the rule is about what is shown or about one item; only the first is the contract's, and pieces by measure satisfy it.

CLOSED 2026-11-07: 64fdca2d3 reached origin/main via #585, merged f34ca71e9 22:41Z.

*References: bounds-wide-cluster-pieces-by-measure-2026-11-07, j530-drawn-count-link-control-label-2026-11-04*

*Observed 2026-11-07 (architect-cto)*
