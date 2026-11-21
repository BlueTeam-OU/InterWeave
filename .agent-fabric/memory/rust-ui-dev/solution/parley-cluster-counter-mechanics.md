---
role: "rust-ui-dev"
class: solution
topic: "parley-cluster-counter-mechanics"
description: "What parley 0.11.1's 8-bit cluster counter counts (ICU grapheme segment, shaping-contributing code points only); tags never count; Khmer/Myanmar stacks are one UAX cluster"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 07213728b6344216
---

## What parley 0.11.1's 8-bit cluster counter counts (ICU grapheme segment, shaping-contributing code points only); tags never count; Khmer/Myanmar stacks are one UAX cluster

The panic behind TM-58 (parley-0.11.1 src/shape/mod.rs:270 "attempt to add
with overflow") is `map_len: u8` in `fill_cluster_in_place`: one count per
grapheme cluster as parley's own segmenter breaks the text (icu_segmenter
2.3 GraphemeClusterSegmenter), of code points that contribute to shaping
(not Control, Zl/Zp, or Format unless script Inherited). So:
- the shaper's counted unit is ICU's extended grapheme cluster, not a
  harfrust cluster; `crates/human/ui-slint/tests/cluster_scripts.rs` holds
  ICU's breaks equal to unicode-segmentation's for every text it draws;
- emoji tag characters (Cf, Common) never count: a tag sequence of any
  length cannot overflow it, so no mutation of the replacement reaches it;
- unicode-segmentation 1.13.3 and ICU 2.3 join Khmer coeng (U+17D2) and
  Myanmar virama (U+1039) chains as GB9c linkers: a 302-cp coeng chain is
  ONE cluster (replaced, not shaper-joined). Don't assume they split.
Measured 2026-11-14, j111, branch commit fb822d109. Related: [[slint-test-glyph-depends-on-host-fonts]].

**Why:** a test written from the belief "Khmer coeng splits under UAX #29"
failed its own premise; the tag case looks like a missed mutation but is
unreachable by construction.
**How to apply:** before claiming a shaper overflow for a script, check
ICU's segmentation and the contributes_to_shaping filter, not harfrust.

*References: slint-test-glyph-depends-on-host-fonts*

*Anchors: `crates/human/ui-slint/tests/cluster_scripts.rs::same_boundaries`, `Cargo.toml::icu_segmenter`*

*Observed 2026-11-14 (rust-ui-dev)*
