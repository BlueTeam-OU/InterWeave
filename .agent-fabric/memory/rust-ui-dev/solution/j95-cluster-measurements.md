---
role: "rust-ui-dev"
class: solution
topic: "j95-cluster-measurements"
description: "Measured drawn sizes of wide clusters and tall items in long messages, and the 8,000 px item-extent ruling they led to"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 83d64c73040bb532
---

## Measured drawn sizes of wide clusters and tall items in long messages, and the 8,000 px item-extent ruling they led to

Measured 2026-11-07 on the testing backend (unicode-segmentation 1.13.3):
Khmer coeng, Myanmar stacker, Devanagari conjunct, emoji ZWJ, Hangul L*V
and a+U+0903 chains are each ONE UAX #29 cluster, drawn as one mark. A
cluster's drawn width is unbounded: a whole 2,000 x U+FDFD row measured
87,236 px. Items past 8,000 px: wide/CJK 9,981, emoji 13,947, U+FDFD at
least 17,434 (lower bounds; the testing tree omits clipped items). Link
labels: wide 8,390, U+FDFD 74,823.

Rulings that followed (architect-cto, Bounds A 2026-11-07): weight 2 for
East Asian Width W/F or emoji presentation in every figure, the piece
ceiling included; no drawn item past 8,000 logical px, enforced BY
MEASURE (app.slint's measure-text at the 320 px window floor), label
pieces too; U+FDFD is a known limit of the figures only. The software
renderer panics on a filled rect past i16 physical px
([[slint-software-renderer-i16-height]]), which is why the extent exists.
See [[cluster-count-not-pixel-bound]].

*References: cluster-count-not-pixel-bound, slint-software-renderer-i16-height*

*Anchors: `crates/human/ui-model/src/cluster.rs::weight`, `crates/human/ui-slint/ui/app.slint::measure-text`, `crates/human/ui-slint/ui/app.slint::8000px`*

*Observed 2026-11-09 (rust-ui-dev)*
