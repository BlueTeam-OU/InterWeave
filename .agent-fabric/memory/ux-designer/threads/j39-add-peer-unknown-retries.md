---
role: "ux-designer"
class: threads
topic: "j39-add-peer-unknown-retries"
description: "j39 state — add-a-peer unknown-retry rule and Peers row contact names, PR #388"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d71690707ca50d67
---

## j39 state — add-a-peer unknown-retry rule and Peers row contact names, PR #388

j39 (rust-ui-dev-01 QUESTION 01a16651, #384 Codex round 7) on PR #388, branch
develop-qzapp/ux-designer-01/docs/add-a-peer-unknown-retries, first head a50b4b67.
- §5.7 one rule: an unknown change stays unknown until a read shows it; a no-write retry
  keeps RemoveUnknown/UndoUnknown/AddUnknown (same lines)/TrustStepUnknown. Declined: a row's
  step opened while trust unread is not unknown (its not-written text is true).
- §5.2 row lists every contact of the identity in front on its PeerId, collation order,
  PeerContactNames placeholder (LC REQUEST 01a16653-589b).
- Still open from j38: architect Q-own (01a165de-5378).
Lesson: never background a watch with output to /dev/null; and `pkill -f` matching the
pattern kills the calling shell (exit 144).

2026-10-23: rounds 1-4 (F1-F5 fixed), #388 clean at round 4 on 570129d2 (1 work, 3 fix).
AUTHORIZES asked: REQUEST 01a1665c-d632. rust-ui-dev told (INFO 01a1665c-d74a), asked about IPC call order.
Contact names: one per line, never joined (LC note 01a16654); LC's pattern commit e1392ddf not folded.
Residual risks: acceptance 10 l.838 / s5.7 l.602 older "later read" wording, no in-flight-read case;
read after Done vs queued Trust write (IPC order); late answer after Done; trust-unread row whose
opening read shows trusted (no block); scanned add "field kept".
2026-10-23 later: rounds 5-6 (F6-F9); #388 clean round 6 on 3661943f (2 work, 4 fix). AUTHORIZES asked again
REQUEST 01a16665-388e (01a1665c superseded). Architect questions open: Q-own, Q-pending (trust settings page),
Q-collation (interim: Unicode lowercase then code points). Carried P3: s9 TrustThisPeer row omits Connect's use.
2026-10-23: rounds 7-8 (F10-F11, best-effort 'known' limit, Q-late). #388 clean round 8 on c7c7a758 (3 work, 5 fix); AUTHORIZES asked again. Layout nit: l.407 next rule runs into the bold limit paragraph.
2026-10-23 03:16Z: rounds 9-13 (row-gone focus, F12-F15, PeerRowGone folded b0483154). #388 clean round 13 on 23a65918 (5 work, 10 fix); AUTHORIZES REQUEST 01a166xx sent. Open: focus after RemoveUnknown re-read shows row gone.
Owed in next revision: RemoveUnknown re-read without row = as Remove done, nothing announced (rust-ui-dev REPLY 01a166c7, built on #384).
2026-10-23 03:25Z: #388 ARMED at 23a65918 on architect AUTHORIZES 01a166c9-9f30. Q-own DECIDED (DECISION 01a166c9-b2ea): refusal own_peer in add_address/add_addresses (IPC 2.9 batch, p2p-network-dev-02 j62) and client refuses first; job j40 queued; LC asked for RefusedOwnAddress (REQUEST 01a166cb-2a6f). After #388 merges: new branch off main for j40 (s5.3, s8, s9, s11 'until the method lands, the client's refusal is the whole protection') + RemoveUnknown re-read sentence.
j40 inputs (LC REPLY 01a166cc-3813, by locator): RefusedOwnAddress = "This is this device's own address, so it cannot be used to reach another device. Add the other person's address instead."; PeerStateThisDevice corrected to "Own address: this address belongs to this device and cannot be used to reach another device". LC note: AddRefused puts "." right after {line}, which may be copied as part of the address; consider {line} as its own code-font span.
2026-10-23: #388 merged 4948bf3d, j39 delivered. j40 on #390 (branch docs/add-a-peer-own-peer), head c4397c99, review round 1 running; rust-ui-dev told (INFO 01a166dd-5912).
2026-10-23: #390 clean round 4 on fa00b156 (1 work, 3 fix); AUTHORIZES asked. After merge: deliver j40.
2026-10-23 04:22Z: #390 ARMED (queued) on architect AUTHORIZES 01a166fe. Fact for next edit: own_peer is built on p2p-network-dev-02's 2.9 branch (schema 06dc78cc); ADR-0062 amendment follows. When it lands on main, update s5.3/s5.7/s11 from 'planned' to recorded.
Owed next revision: AddRefused layout (b) per my REPLY to rust-ui-dev 01a16703: 'Not added:' / refused line alone in code font, wrapping / reason; label = filled AddRefused; replaces s9 span sentence.
2026-10-23: #390 merged e32574dc, j40 delivered. j41 (AddRefused layout b + carried P3s) on #391 fdca6d71, review running.
2026-10-23: j42 (rust-ui-dev-02 OBSERVATION 01a16711, ADR-0043 line) folded into #391 as 69029b45 with IdR1NewKeys (ADR-0063 fresh-state line, a gap I found); LC asked (01a16715-6856). #391 head f033e576, round 3 running.
2026-10-23: #391 clean round 5 on adcf0f58 (5 work, 4 fix); AUTHORIZES asked (arch14). Open: architect QUESTION 01a16718 (identity meaning s9 l.135) -> R1 title, IdR1Settings; IdActiveElsewhere text differs on #392 (told rust-ui-dev-02). Carried P3: s5.3 typed list includes scan-only reasons; s6 reading order no-line case; TalkBack-blocked marking on add-a-peer s8 rows.
2026-10-23 05:24Z: #391 ARMED (queued) on AUTHORIZES 01a16736-b0cf. Identity meaning decided (01a16736-cf7a): s9 l.135 = node's PeerId; R1 title + IdR1Settings say PeerId; LC asked (lc13). After #391 merges: deliver j41, j42; new revision for identity wording (job from 01a16736-cf7a).
j43 queued (PeerId wording). LC texts by locator in scratchpad lc_peerid.txt (REPLY 01a16739): R1 title, IdR1Settings, IdR1NewKeys revised, IdRestore, IdRestoredTitle, IdCreatedBody, IdOtherIdentity, IdRestoreOther, IdQ* , IdStorageUnknown/Failed. Create flow asked of architect (QUESTION 01a1673a).
2026-10-23: #391 merged ddf1db80; j41, j42 delivered. j43 (PeerId wording) on #396 b3ba5625, review running.
2026-10-23: #396 clean round 2 on e9833b33 (1 work, 2 fix); AUTHORIZES asked. After merge: deliver j43; tell rust-ui-dev to apply texts in labels.rs.
2026-10-23 07:55Z: j44 (rust-ui-dev-01 QUESTION 01a167be) answered on #396 7b676d33: list off only from a row (a); Back during add closes the page = unknown add. AUTHORIZES re-ask after round 3.
2026-10-23 08:05Z: #396 adds AddWaiting (folded 3faef206) and contracts-landed update 6cf25b9f (own_addresses 2.8, add_addresses/own_peer 2.9 recorded; RefusedLimit names a line); main merged c3377233; round 4 running. Tell rust-ui-dev-01: RefusedLimit names a line.
2026-10-23 08:07Z: #396 clean round 6 on 9927df50 (4 work, 7 fix, 1 merge); AUTHORIZES asked (arch18). Tell rust-ui-dev-01 after merge: RefusedLimit names a line; several-not-taken is a client check below 2.9; RefusedLimit text new.
2026-10-23 10:08Z: j45 (QUESTION 01a16839) on #396 6a1a03ef: second row-gone §8 row, focus however it got there; PeerRowGone placeholder until LC (01a1683a-0789). AUTHORIZES 01a167cc superseded.
2026-10-23 10:12Z: #396 round 7 F10/F11 on ed56237c; LC asked re PeerRowGone 4th case (app-placed focus) 01a1683d-7583. Tell rust-ui-dev-01 after: RemoveUnknown read back moves focus only if still on snackbar/row; read-back exclusion only for that row.
2026-10-23 10:20Z: #396 clean round 12 on 63680cb2 (6 work, 13 fix, 1 merge); AUTHORIZES asked (arch20); rust-ui-dev-01 told (ui22). After merge: deliver j43, j44, j45.
Owed next revision (s8): a target the app placed, dropped before focus reached it: next/previous/Add an address, nothing announced (rust-ui-dev-01 QUESTION 01a16856, answered 01a1685x).
2026-10-23 10:40Z: #396 CI red from RUSTSEC-2026-0385 (all PRs); fix is #386 (j115). After #386 merges: merge main into #396, re-review the merge, re-ask AUTHORIZES.
Owed next revision (s5.7): AddUnknown note goes when the field no longer holds the unknown add's lines; not restored by editing back (rust-ui-dev-01 QUESTION 01a16949, job j46). Also s8 placed-target-dropped-before-focus (01a16856).
Owed next revision (s5.7): pending per line for adds (like Trust per PeerId): every unknown add's lines pending until a read/answer decides or page closes; no-write Add including a pending line -> AddUnknown (QUESTION 01a16973, job j47). Corrects j46 reply's 'new add' sentence.
Owed next revision (s5.3, s8): a block settled by an unprompted read: focus stays, line announced once; if decided trusted and focus was on its Trust/Not now, focus as a decided block (QUESTION 01a169a2, job j48).
2026-10-23: #396 ARMED on owner's word ('merge 396'), queued at 2. After merge: deliver j43,j44 (j45-j48 go to next revision PR).
2026-10-23: #396 merged 46fa43d9; j43-j45 delivered. j46-j48 + j45 follow-up on #400 d504c32f, review running.
2026-10-23 18:37Z: #400 clean round 5 on 3f4062ad (1 work, 4 fix); AUTHORIZES asked (arch23). rust-ui-dev-01 told (ui28). After merge deliver j46-j48.
2026-10-23: #400 merged b82e437f; j46-j48 delivered. Queue empty. Still owed: rust-ui-dev-01 builds AddWaiting, unsettled lines etc. on their next PR; known risks: unasked read during TrustStepWaiting, double announcement on a focused changing line.
2026-10-24: j49 (architect REQUEST 01a16c4a, Stage 21 close step 5): flows already in add-a-peer s5.1/s5.4/s5.6; gaps closed on #405 dfaaaba1 (C4/R4 -> Connect in identity note; relay-only line in s5.4); reply 01a16c4e sent.
2026-10-24: #405 clean round 3 on 35f320f7 (1 work, 2 fix); AUTHORIZES asked (arch25). Deliver j49 after merge.
2026-10-24: #405 merged 5f4d9ecf; j49 delivered; architect told.
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

*Observed 2026-10-24 (ux-designer)*
