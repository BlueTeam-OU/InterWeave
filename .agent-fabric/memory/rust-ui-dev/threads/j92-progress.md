---
role: "rust-ui-dev"
class: threads
topic: "j92-progress"
description: "j92 long-message work - PR #563 state, rounds, what is owed (second batch: the cut and reader), and the arming basis"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7570122498372af8
---

## j92 long-message work - PR #563 state, rounds, what is owed (second batch: the cut and reader), and the arming basis

#563 MERGED 2026-11-05T03:16:53Z as dd37a9e0 (13 work, 20 fix; rounds 1-4, F2 deferred on architect-cto's word).
j92 is blocked on #561 (the design note); the second batch starts on a fresh branch off main once #561 merges.

What #563 holds:
- body pieces (1,600 graphemes / 32 lines, `body::is_line_break` counts all UAX #14 breaks);
- the six-line composer;
- link-control bound (`LABEL_GRAPHEMES` 2,000);
- the cluster bound (32 code points -> one U+FFFD, `ui-model` `cluster`, Slint `Drawn` global on every edited TextInput,
  compile.rs scan), with TM-58 owing the A40 run;
- TM-57 as an owed row (the cut and reader), on architect-cto's word REPLY 01a1a987-b621;
- architect-cto's supplied contract commits (551b65f24..125c623da, from their for/rust-ui-dev-02/cluster-bound);
- the j90 A40 record 41e15558.

**Arming basis used (architect-cto, 01a1a987):** #563 waits for neither #561 nor #562, because the crash is on main and in
v0.1.0. Arm it yourself once the re-review of the finished head is posted with no open P1/P2; it is over the floor.
Use `fabric-pr arm 563 --basis ...`.

Review rounds:
- Round 1 (posted): F1-F9.
- Round 2 (posted): F10, F11 and R1, all answered.

Saved with `fabric-review save 563 <n>`; the next round comes from `review-563-rr<n>.json` in the session scratchpad.

**Owed next (j92 second batch, after #561 is on main):** the cut (2,000/40, a 1,600/32 preview, the continues line),
the reader page, Copy (allowed on every text message), TM-57's tests, #561 A1-A10. Owed measurements:
- the headings shape at 200 %;
- 40 link controls;
- scripts that cluster otherwise than UAX #29.

Real scale 2 is needed; see [[slint-scale-factor-env-not-applied]].

Not mine but noticed: `tests/support` `net::tests::search_skips_a_port_something_listens_on` failed once with
AddrInUse in a full run, then passed 3/3 alone (p2p-network-dev's harness).

Related: [[slint-software-renderer-i16-height]] [[pixel-handover-p2p-network-dev-02]]

2026-11-05: #561 is MERGED as c9ba8c21 and #562 is MERGED, so the second batch is unblocked. ux-designer-01's #564 (their
j92) revises long-message.md and start-window §3. It is PROVISIONAL until its review is clean (INFO 01a1aaa9). The
changes that reach the build:
- §6: the accessible text replaces each U+FFFD mark with ReplacedCharacter (§8) where the platform reads U+FFFD as
  nothing. This covers the Item label, the Chats row description, the reader blocks and Show source, but not the
  composer draft.
- A plain-text fallback previews its first source line.
- A12 C4 is 17 '>', a space and the cluster.
Read #564 on main, or its head, before building §6.

*References: pixel-handover-p2p-network-dev-02, slint-scale-factor-env-not-applied, slint-software-renderer-i16-height*

*Observed 2026-11-05 (rust-ui-dev)*
