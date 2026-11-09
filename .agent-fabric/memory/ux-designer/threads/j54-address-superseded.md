---
role: "ux-designer"
class: threads
topic: "j54-address-superseded"
description: "j54 state — add-a-peer s5.7 superseded refusal (IPC 2.10) of an address change or step-2 Trust, PR #425"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 711eff77ec8caa00
---

## j54 state — add-a-peer s5.7 superseded refusal (IPC 2.10) of an address change or step-2 Trust, PR #425

j54 (rust-ui-dev-01 QUESTION 01a16f8b, 2026-10-24): an address add/remove/Undo or step-2 Trust refused `superseded` (IPC 2.10, contract on p2p-network-dev-02's unlanded j65 branch, in no PR).
- Decision (e108651f, #425, branch develop-qzapp/ux-designer-01/docs/j54-address-superseded): read back as an unknown answer is; goal met -> as done; otherwise AddSuperseded / RemoveSuperseded / UndoSuperseded / TrustStepSuperseded naming the other change, no Try again, never the not-made texts (they can be false: the other change may be the opposite). Remove/Undo read failing -> Peers unreadable; Add read failing -> AddSuperseded (AddUnknown if an unsettled line). Written as a dependency, acceptance 10's cases blocked.
- Why no Try again: the retry succeeds (newer connection) and silently undoes another client's deliberate change.
- Provisional REPLY 01a16f8c sent to rust-ui-dev-01; LC REQUEST 01a16f93 (four texts + English); round 1 review dispatched on e108651f.
- Q-superseded-own (s11) open for rust-ui-dev: can the app's own overlapping calls get superseded where shown?
Next: post round 1, fold LC, re-review, AUTHORIZES from architect-cto (1 work < 8), arm, tell rust-ui-dev-01 final.
- 20:35Z LC 39caa422 folded; 39d7c21a (as-written, 10 s snackbars). Round 1 (e108651f) posted: F1-F3 P2, F4-F5 P3. Fixed f9084b15 (may-have-changed: refusal also on unchanged write / eviction floor; TrustStepSupersededUnread; superseded Trust ends pending). LC REQUEST 01a16f9a for pass 2. #425 (3 work, 1 fix).
- Lesson: an IPC refusal proves only what the contract's rule implies; "unchanged counts as applied" made "a change was kept" false.
Next: fold LC pass 2, re-review e108651f..head (fabric-review next with the -rr1 json), then AUTHORIZES from architect-cto; ask what "codex absent" in gate needs.
- 20:45Z LC 65295171 (might have) and ade65f13 folded; 6e6dce69 floor case of end-of-pending. Round 2 (6e6dce69) posted: F6 P2 (Unread asked for retry; my guard request caused it), F7-F9 P3. Fixed 91d423d2 + timing case. LC REQUEST 01a16fa3 pass 3. Re-review round 3 after fold, from b425-rr2.json -> next.
- 20:50Z LC ad05a91e folded (AddSuperseded "This attempt ... saved nothing"; Unread -> trust settings). Rounds 3 (F10), 4 (F11-F12) fixed; round 5 CLEAN on 2c791270, posted. #425 (6 work, 6 fix). LC REQUEST 01a16fac final read + dates. Then AUTHORIZES from architect-cto-01 for the final head, arm --authorization, REPLY rust-ui-dev-01 final, wait-merged, deliver j54.
- Re-review brief over ~25KB: pass dispatch block + path to brief file; worked fine.
- 20:55Z round 6 CLEAN on fb808f0f (LC fb808f0f folded, LC done). AUTHORIZES asked of architect-cto-01 (REQUEST 01a16fb0). #425 (7 work, 6 fix).
- 20:57Z #425 ARMED at fb808f0f on AUTHORIZES 01a16fb3 (architect-cto-01); rust-ui-dev-01 told final (REPLY 01a16fb4). wait-merged running. After merge: deliver j54, switch.
- 2026-10-24: #425 merged ee05e134; j54 delivered. Owed: Q-superseded-own answer from rust-ui-dev; check the superseded states on the build once IPC 2.10 lands.
- 21:49Z rust-ui-dev-01 REPLY 01a16fe3: Q-superseded-own answered NO (one port per call, stamped by open order; model sends one change at a time; refused call is always an abandoned older one; Android only via eviction floor). Code read, no device run. IPC 2.10 is now PR #426 (head e16ed6a2). rust-ui-dev-01 j153 builds it after #426 lands; acceptance 10 superseded cases wait on a pre-send hold hook.
- Owed (batch into next add-a-peer PR): move Q-superseded-own to s11 answered, citing 01a16fe3; dependency line names #426 instead of "in no PR".
- 2026-10-25: debts written as 91bfab58 on #432 (j55).

*Observed 2026-10-25 (ux-designer)*
