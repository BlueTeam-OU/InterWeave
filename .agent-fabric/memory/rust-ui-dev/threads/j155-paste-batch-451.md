---
role: "rust-ui-dev"
class: threads
topic: "j155-paste-batch-451"
description: "j155 batch on #451 (2026-10-26): #409 P3s, ListOff test, ClipboardUnreadable, bounded Linux clipboard read, desktop-e2e paste case; what is left and owed"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2da794351d5d2fbc
---

## j155 batch on #451 (2026-10-26): #409 P3s, ListOff test, ClipboardUnreadable, bounded Linux clipboard read, desktop-e2e paste case; what is left and owed

j155's code batch rides PR #451 (branch develop-qzapp/rust-ui-dev-01/feat/j138-377-follow-up-android-store-screen, worktree interweave-j138-...), 8 work commits after merge 59858441:
- b63e3fef Show draws no code when lines exceed QR 40-L (#409 r3 F2); d2406830 scan press while scanning ignored (r3 F4); f1318b09 minimised_closes_show predicate + test (r3 F3); f30be4f4 ListOff-over-Limit test (rule was already on main).
- e1b544a5 ClipboardUnreadable (model input/note + PLACEHOLDER (en) key); f4e40033 ui-slint/src/clipboard.rs: bounded read (wl-clipboard-rs data-control, else x11rb incl. INCR; 32 MiB+1 / 5 s; rustix poll, unsafe is denied); 1b477f80 human-desktop SCANS const, Paste on Linux only; 4385f929 desktop-e2e codes.rs paste case with an in-test x11rb clipboard owner (no xclip in CI).
- Evidence: X11 real path `xvfb-run -a cargo test -p interweave-human-ui-slint --features desktop --lib -- --ignored x11_reads` (needs xclip); e2e under tools/ci/with_display.sh; mutations each failing. Wayland path never run against a compositor (none on this host).
- LC kept the draft (REPLY 01a175fc; 11820edd removes the mark). ux REPLY 01a1761b: ShowTooManyForCode line + Paste only where bounded (provisional, their j65 branch 88b5fa40) -> my job j165.
- Round 7 review (5646734518, whole PR 05d3f9cd..11820edd): F1 P2 deadline only checked when queue/pipe empty -> 9e7b9585 (next_before, per-turn check); F2 P3 ignored X11 test + unnamed never-comments -> ff223399 (desktop-e2e a_hostile_clipboard_owner_is_given_up_at_the_deadline: Silent=answers nothing incl TARGETS -> ClipboardUnreadable; Endless INCR -> ImageUnreadable). Measured: on Xvfb an endless INCR owner never out-paces the client (empty chunk after a few bytes), so the never-drained queue is unit-tested only. Re-review of 11820edd..ff223399 dispatched.
- j164 (ux install request) delivered: Pixel a private-network address:44599 refuses rust-ui-dev-01 pairing; APK of 05d3f9cd in a scratch path + a host scratch path.
- Still in j155 after this: real-scan device check of #418 (owner + phones), Connect s5.1, #400 rules (see j157), TalkBack under the viewfinder.
- Lesson again: `pkill -f <pattern>` matched my own shell (exit 144) -- see [[commit-and-shell-traps]].
Related: [[j138-store-answers-pr451]] [[j134-camera-scan-pr418]]
- 2026-10-26 ~05:50: rounds 8-15 answered (F3-F8; F2/F8 carried as stated limits). j165 (ShowTooManyForCode, LC final) built on #451. #451 ARMED at df7a2d4c (18 work, 17 fix). OBSERVATIONs: identity_recovery flake -> p2p-network-dev-02 (fixed class on their j75 branch; my two loops df7a2d4c); featureless ui-slint clippy not in CI -> devex-tooling (01a176bc-b25a).
- Lesson: deadline asserts in poll loops go FIRST in the body (a pass that settles past the deadline must end the loop, not fail it).
- Lesson: an e2e owner counting what a client takes needs an ordering point (DestroyNotify of the requestor window) before reading the count.
- #451 MERGED 2026-10-26 06:14Z as 906fb21e. Device rows of j138/j139 (Keystore store key, R/K, discard, human_chat Wi-Fi) still owed: the Pixel refuses rust-ui-dev-01's pairing.

*References: commit-and-shell-traps, j134-camera-scan-pr418, j138-store-answers-pr451*

*Observed 2026-10-26 (rust-ui-dev)*
