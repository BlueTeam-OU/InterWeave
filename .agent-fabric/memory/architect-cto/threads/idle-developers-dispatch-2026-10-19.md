---
role: "architect-cto"
class: threads
topic: "idle-developers-dispatch-2026-10-19"
description: "2026-10-19 round feeding six idle developers (gzapp five, InterWeave one): what each REQUEST asks, what was verified on main, and the two decisions I owe from it (step-up MFA mechanism; ADR-078 rider-side sign after gzapp#1119)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 32baf2200032f6bc
  - da35a4e1dc3465e9
---

## 2026-10-19 round feeding six idle developers (gzapp five, InterWeave one): what each REQUEST asks, what was verified on main, and the two decisions I owe from it (step-up MFA mechanism; ADR-078 rider-side sign after gzapp#1119)

fabric-coordinator INFO 01a1558a (2026-10-19): ten developers idle, the
first six mine to feed in gzapp and InterWeave. Six REQUESTs sent after
verifying origin/main, one login each, REPLY-EXPECTED yes:

- backend-dev-01, gzapp, 01a15594-4282: completion plan PR 6 (planning-
  time point selection: shape resolution, StopCandidateGenerator, mapper
  emits dynamic_point, dataset_superseded trigger, planning shadow
  field); PR 4's compiler v2 items (D-05..D-08) first if absent. Verified:
  PR 3 landed (cross_city/unresolved_city); publish path exists without
  step-up MFA (off in production); PRs 6-11 absent (no
  StopCandidateGenerator/StopCallConsolidator/stop_call_id on main).
- db-admin, gzapp, 01a15594-4e89: ADR-028 retention conformance audit —
  per window the sweep statement, its EXPLAIN, index verdict; no-sweep
  → OBSERVATION to backend-dev; unindexed → their migration PR.
- flutter-dev-02, gzapp, 01a15594-5cde: ADR-077 app half (FCM
  registration minted at Start journey, deleted at terminal, no-config
  build). Partition: no edit to journey_dtos/journey_sheet/map_screen
  while flutter-dev-01's #1119 is open. The ad-offer entry point does not
  exist in the app. No push SDK in pubspec today.
- web-dev-01, gzapp, 01a15594-6cac: admin_web + web-shared to the 80 %
  line floor (coverage.yml is workflow_dispatch only; thresholds in
  vite.config.ts; web-shared has no coverage script). web-dev-02,
  01a15594-7b08: advertiser_web, operator_web, status_web. Trigger flip
  is devex-tooling's.
- rust-ui-dev-02, InterWeave, 01a15594-8ad2: Stage 18 S10 store half —
  corrupt store, failed migration, full disk in crates/human/store/tests;
  TM-44/45/46 in tests/human-retention; matrix rows updated; branch name
  to p2p-network-dev-02 (holds S10); nothing under #313's files.

**Owed by me from this round:** (1) the step-up MFA mechanism decision
for gzapp (ADR-032 §2.2, ADR-025 §5.7 rule 16; admin_web's reason-gate.md
says "not yet built anywhere"; StopRestrictionPublicationEndpoints keeps
publish off until it exists) — completion plan PR 5's last item;
(2) ADR-078 §2.4 rider-side call sign (journey-state 1.3.0 call_sign,
persistent on the sheet and the map affordance) to flutter-dev-01 once
gzapp#1119 merges — not built; it lands on #1119's files.
INFO 01a15595 to fabric-coordinator carries the ids. Job j158.

**Acknowledgements (19:13–19:17Z):** db-admin j27; rust-ui-dev-02 j29
(feat/j29-stage-18-s10-the-store-half); backend-dev-01 j41
(feat/j41-completion-plan-pr-6-the-hinge; PR 4 state: D-05 not on main,
D-07/D-08/C-03 on main, D-06 half — window join folded into PR 6);
flutter-dev-02 j11 (feat/j11-adr-077-s-passenger-app-half). gzapp's
origin is gzapi-org/gzapp — my DELIVER-TOs said BlueTeam-OU, wrong.
**Owed by me (3):** journey.push-wake 1.0.0 contract (ADR-077 §2.1
payload; PushWakeSender.cs:146-155), job added. **The owner's:** the
Pixel/a device for the doze run, the Firebase app config and the
backend's JourneyWake:Fcm credentials (deployment material). flutter-dev-02
asks language-culture for the wake and Start-journey keys themselves.

**2026-10-21 ~11:50Z, second round (owner away, Pixel unavailable):**
p2p-network-dev-02 idle after #329 merged (1a4fa590). Read on
origin/main: `tests/security-local` still the S0 skeleton; matrix rows
S9 (02) TM-10/12/21, S21-09 spoofed client.kind + real uid, C-03, and
S11 (02) TM-22/23/24/48, S21-11 transportctl tamper, TM-32's daemon
half all `owed`; ci.yml has no `sudo -u` / `--include-ignored` step
(P2 placed it in S0's PR, never landed). REQUEST 01a15e4b-4eff: S9+S11
as one job, one PR theirs; the ci.yml step is devex-tooling's wiring
(they REQUEST it with the variable and command); CI row stays `owed`
until it lands; boundary PR, my word under the minimum. Job j223.
Lesson: "the Rust lanes ran dry" (2026-10-20) was wrong for S9/S11 —
read the matrix's `owed` cells per lane before calling a lane idle.
- 2026-10-24 23:25Z: the 10-21 ux-designer-01 gzapp placement REQUEST (01a15f3e) was sent TO a non-existent login (develop-qzapp/fabric-coordinator; the real address is develop-qzapp/user) and never arrived. Resent as 01a17038 to develop-qzapp/user with the owner's 10-21 word quoted; j228 re-blocked on it. LESSON: fabric-coordinator's login is `user`; gzcoord-send's presence check cannot catch an address no host places when sent --force or before the check existed — verify the login in fabric-ctl all presence before TO.
- 23:24Z fabric-coordinator undertook the placement as their j392 (REPLY 01a1703b): remit from my proposal + language-culture-en's two passes on design/ specs + product/i18n named by path; PR from their next gzapp session, working copy by bootstrap, then the first work and web-dev-01's two OBSERVATIONs are ux-designer-01's. j228 clears on their PR.
- 2026-10-28 13:38Z: fabric-coordinator's REPLY 01a182bb-8da6 to 01a17038: the ux-designer gzapp remit is on gzapp #1147 (22eca958b, the flutter/web/language-culture seams 60478c8d2); #1147 unarmed, held on devex-tooling's Makefile rows (ROLE_STACK_ux-designer, port offsets) then the review gate; ux-designer-01 has NO gzapp clone until python-dev-01 builds a control-plane action to place one on an existing account — hold any gzapp REQUEST to ux-designer-01, or send it knowing that. j228 stays blocked on the presence read-back.
- 2026-10-28 13:48Z: G8 CLI shape ruled for rust-ui-dev-02 (REPLY 01a182c2-97ff to QUESTION 01a182c1-b0c6): a new verb `interweave-human peers state [--json]` (`{peer, connected, last_seen_ms}`, null plus `"last_seen": "unreported"` below IPC 2.12), `peers --json`'s address rows untouched; agent-fabric origin/main has no reader of `peers --json` (grepped runtime/, communication/, tools/, bin/, policies/). Their branch: develop-qzapp/rust-ui-dev-02/feat/j76-peers-state-and-listen-filters off 2c9a045e; PR number is the acknowledgement.

*Observed 2026-10-19 (architect-cto)*
