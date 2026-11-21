---
role: "architect-cto"
class: threads
topic: "j559-slint-shift-carry-plan-20-2026-11-08"
description: "j559 (2026-11-08): the Slint 1.18.1 Android hardware-keyboard Shift limit recorded in plan §20's closing record as a carry to the next Slint bump — InterWeave #596 (1 work, docs only, arms on the owner's word); the owner's open question…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f9141ae762725a12
---

## j559 (2026-11-08): the Slint 1.18.1 Android hardware-keyboard Shift limit recorded in plan §20's closing record as a carry to the next Slint bump — InterWeave #596 (1 work, docs only, arms on the owner's word); the owner's open question: is a hardware keyboard a v0.3 target?

CLOSED 2026-11-09: #596 (1 work, 1 fix) merged ae7c0dc9a on the owner's word at 6b4782277; the carry is on main. Open still: the owner's hardware-keyboard question; the bump is rust-ui-dev's. DELIVERED 2026-11-08: BlueTeam-OU/InterWeave#596 (1 work, 1 fix; head 6b4782277; round 1 on bcad69373 found F1/F2 P3 wording, fixed, re-review clean on the head; origin/main's four drain commits under .agent-fabric/memory not folded, no conflict, so the review stays on the head; awaits the owner's word, under the minimum) `architecture/roadmap/BOTTOM-UP-IMPLEMENTATION-PLAN.md` §20 closing record, after "carries this too"): the carry of the ruling in [[slint-android-shift-dropped-ruling-2026-11-08]]. Verified before writing: the registry copy of `i-slint-backend-android-activity-1.18.1/androidwindowadapter.rs` (`map_key_event` at line 692) maps each Keycode to one fixed text and never reads meta state (no `meta_state` anywhere in the file); `Cargo.toml` pins every i-slint-* crate `=1.18.1`; `design/2026-10-19-android-identity.md` §8 says the screens are workable with a hardware keyboard and item 15 that one enters a recovery word by its letters.

**Owner's question, open:** is a hardware keyboard a target of the v0.3 release? If yes, the Slint bump (rust-ui-dev's, when a release carries 3a7e700487) becomes a v0.3 gate item; if no, the carry stands with no date. Put in my final report of 2026-11-08; no answer yet.

**Theirs:** the Android README limits prose and the bump (rust-ui-dev-01); the re-measure needs a phone with a hardware keyboard attached, which no device run has had (the review of 2026-10-19 F12 says so).

Closes when #596's sha reaches origin/main ([[supply-job-closes-on-the-supply-sha-reaching-origin-main]]).

*References: slint-android-shift-dropped-ruling-2026-11-08, supply-job-closes-on-the-supply-sha-reaching-origin-main*

*Observed 2026-11-08 (architect-cto)*
