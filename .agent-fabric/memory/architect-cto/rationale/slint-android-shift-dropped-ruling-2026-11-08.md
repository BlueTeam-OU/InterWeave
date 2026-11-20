---
role: "architect-cto"
class: rationale
topic: "slint-android-shift-dropped-ruling-2026-11-08"
description: "Ruling 2026-11-08 for rust-ui-dev-01 (QUESTION 01a1bb8b, REPLY 01a1bb8d) — Slint 1.18.1's android-activity backend drops Shift/AltGr/dead keys from a hardware keyboard; chosen (b): no vendored patch, wait for the release carrying slint…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 55b58adcf5775aba
---

## Ruling 2026-11-08 for rust-ui-dev-01 (QUESTION 01a1bb8b, REPLY 01a1bb8d) — Slint 1.18.1's android-activity backend drops Shift/AltGr/dead keys from a hardware keyboard; chosen (b): no vendored patch, wait for the release carrying slint 3a7e700487, record the limit in plan §20's carried list (mine) and the Android README (theirs)

ux-designer-01's A17 run (OBSERVATION 01a1bb5a, finding 1): capitals injected as key events arrive lowercase on Add an address, so a typed PeerId is refused; Gboard is unaffected (commits text through the input connection). Cause per rust-ui-dev-01: i-slint-backend-android-activity 1.18.1 `map_key_event` maps each Keycode to fixed text and ignores the meta state; fixed upstream in 3a7e700487 (2026-09-22), no release since; workspace pins `=1.18.1` (Cargo.toml L293–298, exact pins for the i-slint-* advisory).

Ruled (b): every `third_party/` tree has an ADR saying why a registry release would not do (0051/0053/0054) and here only time is missing; a patched UI backend is a fork across an 86-commit gap for an input no shipped flow assumes (Restore's R2 takes lowercase + Enter; the j152 run typed the words with `adb input text`). (c) refused: a user-visible defect must be on record.

**Delivered by me 2026-11-08:** plan §20 carried-list entry on InterWeave #596 (bcad69373), [[j559-slint-shift-carry-plan-20-2026-11-08]]; the owner's question (hardware keyboard a v0.3 target?) put in that session's report, open. **Theirs:** the Android README limits prose; the bump when a release carries the commit.

*References: j559-slint-shift-carry-plan-20-2026-11-08*

*Observed 2026-11-08 (architect-cto)*
