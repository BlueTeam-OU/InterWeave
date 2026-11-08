---
role: "architect-cto"
class: threads
topic: "stage-17-closing-plan-2026-10-19"
description: "Stage 17 close (plan §20) as it ran: the owner's 2026-10-19 direction, the ux-designer gate clause (Android app against the Telegram/WhatsApp standard), the per-clause gap read from main, the role sequence (j148) and the rulings up to the…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0036d315bfa413db
  - a61efc3cef66800c
---

## Stage 17 close (plan §20) as it ran: the owner's 2026-10-19 direction, the ux-designer gate clause (Android app against the Telegram/WhatsApp standard), the per-clause gap read from main, the role sequence (j148) and the rulings up to the close on #373 (2026-10-22)

The owner said on 2026-10-19 "we have to close stage 17" and added
that the Android app's aesthetics are far below standard: ux-designer
must review it to bring it to the level of the mainstream messaging
apps (Telegram, WhatsApp). Job j148.
Gap read from origin/main (e254aaba) on 2026-10-19, per §20 gate clause:
- met or ready: (a) spikes; (d) and (g) tests exist stand-in + device;
  (e) endpoint_overlay.rs in profile-config; (f) TalkBack carried
  (#297); (i) no Proof::Stage(17); steps 1-6 and 9 in the app; OSV
  scan documented.
- missing: steps 7-8 (recovery Activity, BIP-39 picker, no clipboard);
  platform tests FLAG_SECURE, notifications, backup exclusion,
  user-presence diagnostic; API 34+ run recorded (ADR-0041
  remoteMessaging); PeerId copy on device and render parity (f);
  interoperability desktop-Android matrix row (c) or re-carry; device
  run with relayed route-indicator statement; step 10 store metadata
  and Play-policy quote; OSV at closing head (h).

[… 9 earlier entries of this thread moved to stage-17-closing-plan-2026-10-19-log.md on 2026-10-28, not drained …]
2026-10-20 11:30Z the owner (via fabric-coordinator 01a15913-dafa): YES
M1/M2 carried again by name; YES no in-app replace-identity carried as
an open product item; the Pixel window is TODAY, address relayed to
rust-ui-dev-01 and p2p-network-dev-02. INFOs sent to the three lanes
(one install of #313's finished head; p2p-02 commits the six-case log
with API 37 = step 3's API 34+ record; ux round 2 last). RULED: a
Platform-tests line whose instrumented case is not built before the
address arrives is run by hand in the window, read through logcat and
committed as a run record; the record cites it and carries the case by
name to rust-ui-dev's next branch. F3 ruled (b): the handle residual is
Slint 1.18.1's Android backend (Disable arm hides the keyboard only,
verified in the registry source), carried by name with the upstream
issue; no third vendored crate. The CLOSING RECORD goes as a work
commit on #317 (my open PR, the same plan file), not a new branch.
Draft closing record written 2026-10-20 ~11:50Z in the session
scratchpad (s17-closing-record-draft.md, `[[...]]` placeholders for the
window's shas, the eight-line mapping, (h), round 2, the upstream issue
number); if the scratchpad is gone, its facts are in this file and in the
plan's §18/§19 Met blocks (the model). Status slug coined for Stage 18:
stage-18-adversarial-security-gate (none existed in the tree); the flip
touches Cargo.toml status + README/IMPLEMENTATION/CLAUDE.md §1
(check_stage_status.sh). The error-contract ledger (Proof::Stage rows in
tests/transport-contract/tests/error_contract_matrix.rs) has no Stage 17
row. No Android instrumented render-parity case exists: (f)'s Android
half is carried with the TalkBack subsection as the reason.
2026-10-20 16:50Z: gate (c) EVIDENCE = #322 (p2p-network-dev-02, 5 work
6 fix, authorized on my word at bae27538): tests/android-e2e/runs/
2026-10-20-pixel-9a-api37.md — six cases on #313@35c71169, Pixel 9a API
37; relayed paths failed exchange 3 first (test-design race, TRANSPORT
outcome-unknown kept; close_exchange fix, unproved on device, carried);
relayed route indicator PROVED live (ui-model read; phone answered
path=relayed pass). The cases do NOT record §20 step 3 (FGS; they
force-stop the app) nor gate (b)'s notification line (count stayed 0):
both come from rust-ui-dev-01's hand runs of the installed app on the
Pixel (their run record on #313, still unpushed). Draft updated.
2026-10-20 17:07Z: #322 (5 work, 6 fix) merged a4538593: plan §20 gate (c), the Pixel run record, is on main. j148 still waits on rust-ui-dev-01's #313 (gate b run record, F16 cause, F4 fix) and ux round 3.
**2026-10-21 ~18:50Z, clause (b) decided.** The owner ("go as your
recommendation for clause b"): for this close a Platform-tests line
is met by an instrumented device test OR a dated device-run record
(device, API level, build, commands, run by a person); the unbuilt
instrumented cases — Keystore user-presence (needs a biometric) and
network callbacks (needs the Wi-Fi toggle) — are carried by name to
the release gate; TalkBack items with the TalkBack carry. Written into
plan §20 on IW #344 (458bcc8e). The evidence inventory of main per
clause is scratchpad/j148/gate-gaps-2026-10-21.md (subagent read,
2026-10-21 16:30Z). Lane shape: rust-ui-dev-01 one batch after
SPIKE-011's PR (F9/F16/F10/F17, device rows incl. notification
arrival, gate (d) row, human_chat on device, STORE-CHECK, README, two
instrumented cases); the network-rebind row ran with the owner at the
phone 18:49Z. Still open for the record: (c) platform matrix re-carry;
(h) OSV scan at the closing head (devex-tooling asked 2026-10-19);
(j) identity screens never reviewed by ux-designer (needs the owner's
unlock); p2p-02's relayed-paths run on a device.

2026-10-31: owner gave the Pixel 9a at a private-network address:45649; rust-ui-dev-01 (their j183) ran both owed rows, record e63313a9 (apps/human-android/device-runs/2026-10-31-pixel-9a.md): arrival 3/3 under the person's grant, Wi-Fi drop in the app's log, loss callback still silent (instrumented case stays carried). My SPIKES.md + plan §20 measured clauses supplied at d82e22a1 on for/rust-ui-dev-01/pixel-10-31-measured-items; they fold, open the PR (2 work), I give the word. j157 closes when on main. Phone came back at a private-network address:35475.
2026-10-31: #526 MERGED 4a7fd48f; j157 closed.

*References: ...*

*Observed 2026-10-31 (architect-cto)*
