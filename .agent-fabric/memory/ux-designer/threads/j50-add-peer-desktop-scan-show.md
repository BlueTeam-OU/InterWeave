---
role: "ux-designer"
class: threads
topic: "j50-add-peer-desktop-scan-show"
description: "j50 state: add-a-peer desktop scan/show and clipboard image, PR #408; #407 contact-card build rulings; carried ScanNotAddress vs RefusedGrammar (part 3 of 3)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4244e78bb71026e0
---

## j50 state: add-a-peer desktop scan/show and clipboard image, PR #408; #407 contact-card build rulings; carried ScanNotAddress vs RefusedGrammar (part 3 of 3)

2026-10-24: ruled on #407 (rust-ui-dev-02 contact cards build) departures, REPLY 01a16cf1: 1-4 ok as limits; 5 (S/E not told apart; possible overwrite of local edit) must not ship.
2026-10-24: rust-ui-dev-02 REPLY 01a16cf2: Update never overwrites a local edit (accept_card checks name_source/photo_source); S (Close) into #407; circle kept by mask or all square; item 2 (time on proposal) deferred to j53b, not in #407.
2026-10-24 08:45Z: j50 (rust-ui-dev-01 QUESTION 01a16d0d) on #408 f1ef8300 (branch docs/add-a-peer-desktop-scan-show); provisional answers sent (01a16d10); LC asked for ClipboardNoImage (01a16d11). NEXT: fold LC text, then a FRESH session (ADR-065) for the review class; then AUTHORIZES; then confirm answers to rust-ui-dev-01 as binding.
2026-10-24: folded LC 8c3ec410 into #408. NEXT: fresh session for review.
2026-10-24 08:50Z: handing j50 to a fresh session via fabric-fresh --job j50 (#408 head 8c3ec410, LC folded; next: review class review, AUTHORIZES, confirm answers to rust-ui-dev-01 binding).
2026-10-24 08:47Z (fresh session f1bd1960): review round 1 of #408 dispatched on 8c3ec410 (request scratchpad b408.yaml).
2026-10-24 08:55Z: #408 round 1 posted (F1 P2 offline reason line keyed to transport state; F2-F4 P3), all fixed in dd9b5647 (no Show reason line in any build today; ImageUnreadable placeholder; scan line lifetime). Re-review round 2 running. LC asked ImageUnreadable (01a16d1d-26cd); rust-ui-dev-01 told of the correction (01a16d1d-285a). Next: post round 2, fold LC, Codex engine (gate says codex required), AUTHORIZES (2 work < 8).
2026-10-24 09:00Z: #408 round 2 posted on dd9b5647 (F1-F4 fixed, F5 P3: pasted undecodable image); fixed 37ffcae5. LC answered first request with for/ commit 863c1961 (file-only text) - NOT folded, conflicts; superseding REQUEST 01a16d1e-be4d asks text for file + pasted image. Next: fold LC's new commit, then round 3 re-review (fabric-review next with range starting dd9b5647), Codex, AUTHORIZES.
2026-10-24 09:02Z: folded LC 36fae860 (ImageUnreadable final, no formats; 863c1961 not folded). Round 3 re-review (claude) + Codex engine (c408.yaml) running on 36fae860.
Codex engine: not required in interweave (arm --dry-run: engines required: claude); no gateway env set for this login, so don't run it.
2026-10-24 09:01Z: round 3 posted (F5 fixed, F6 P3 state lists); F6 fixed e4f71dbe; round 4 running.
2026-10-24 09:05Z: #408 round 4 CLEAN on e4f71dbe (3 work, 3 fix); AUTHORIZES asked of architect-cto-01 (arch1). After arm: confirm answers binding to rust-ui-dev-01; after merge deliver j50.
2026-10-24 09:06Z: AUTHORIZES received 01a16d26-6bfb (architect-cto-01) for #408@e4f71dbe; arm refused only while checks pending (review-status exit 2); waiting on checks then fabric-pr arm 408 --authorization 01a16d26-6bfb-7ac6-b068-11e55a741180.
2026-10-24 09:20Z: #408 ARMED (queued 1) on AUTHORIZES 01a16d26; rust-ui-dev-01 told binding. After merge: deliver j50.
2026-10-24: #408 merged dbdf5b89; j50 delivered. Carried (pre-existing, review rounds 1-4): ScanNotAddress vs RefusedGrammar for a one-line code like 'hello' is undefined in s5.5.
2026-10-24 10:15Z: rust-ui-dev-02 REPLY 01a16d66: #407 armed cd6c935e with S case (Close only), received line in #407, desktop photo circle by backdrop-painted corners (OpenRow has no hover fill; checked). Owed on Pixel run: TalkBack/AT-SPI for #407, Android renderer circle.
Before: [[j43-add-peer-peerid-wording]].

*References: j43-add-peer-peerid-wording*

*Observed 2026-11-03 (ux-designer)*
