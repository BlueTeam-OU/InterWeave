---
role: "ux-designer"
class: threads
topic: "j120-c3-r2-focus"
description: "j120 delivered #646 merged ac621e95f — identity §8: C3 wrong word focuses IdCheckWrong; R2 last empty filled focuses IdCheckWords (navigating); PR #646"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 257cb0ce7e50157c
---

## j120 delivered #646 merged ac621e95f — identity §8: C3 wrong word focuses IdCheckWrong; R2 last empty filled focuses IdCheckWords (navigating); PR #646

2026-11-12, owner's ask, carried from #645's reviews. PR #646 (head ec3c44c29, 1 work commit) on
design/2026-10-19-android-identity.md:
- C3 wrong word: focus to IdCheckWrong (focusable), typing mode kept; error-banner rule wins over chip rule.
  Try again removes the error + both actions, focus to first enabled key, typing (added to the exception list).
  The error stays while typing until the next word is taken (matches identity_flow.rs try_again/choose).
- R2: next empty wraps past 24 (matches picker.rs choose). A chip that leaves no position empty, also after an
  edit, sets navigating with focus on IdCheckWords. Reason: the check's result depends only on the words, so an
  early press loses nothing (failure keeps the words; R3 asks; after K1 restores only the record's PeerId).
  Double-Enter is a hypothesis for the device run.
Round 1 (review 5880234204): F1 F2 real, F3 partly; fixed in 086bcac29 (Try again keeps letters typed since
the wrong word; wrong word named in the typing exception). Round 2 re-review dispatched on 086bcac29. LC-en took
the pass as j301 (REPLY 01a1d119); told to base on 086bcac29 (01a1d11d). Round 2 clean (5880287527). LC j301 folded 729ff9c00; answers a (actions show exactly while the error shows) and
b (R2 rule covers edit and letters typed after full) at 2efa88aba; LC pass on those asked (01a1d122); round 3
re-review dispatched on 2efa88aba. LC pass on answers 706af9bdd folded 386de3a38; rounds 3-4 clean; armed on the owner's word asked in session
(4 work, 1 fix, 2 merge). Merged ac621e95f 2026-11-12. Sent after merge (INFO 01a1d14d): an INFO to rust-ui-dev-01
with the new rules and the stale `try_again` doc comment ("the entry is already clear", identity_flow.rs:747). Under 8 work commits: arm only on the
owner's word, asked in session. Related: [[j119-tap-focus]], [[lc-pass-before-arm]].

*References: j119-tap-focus, lc-pass-before-arm*

*Observed 2026-11-12 (ux-designer)*
