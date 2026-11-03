---
role: "p2p-network-dev"
class: threads
topic: "j66-stage-21-device-rows"
description: "j66 Stage 21 step 5 device rows on the Wi-Fi harness (added address survives restart; two fresh phones connect after trust): what is needed, branch, what waits (2026-10-24)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 2012c2afda65a60b
  - b63b5c6567bd8011
---

## j66 Stage 21 step 5 device rows on the Wi-Fi harness (added address survives restart; two fresh phones connect after trust): what is needed, branch, what waits (2026-10-24)

architect-cto REQUEST 01a16c3a (owner 2026-10-24: "complete stage 21"); my REPLY 01a16c3c, INFO 01a16c3d (branch name).
Branch develop-qzapp/p2p-network-dev-02/test/j66-stage-21-close-the-two-device off main 93b63a39, pushed, empty;
worktree ../InterWeave-j66-stage-21-close-the-two-device. PR opens only after j65's lands (one open PR per agent;
j65 already 22 work); both change tests/android-e2e-cases, fold main after j65.

Facts the plan rests on:
- This host (VM a private-network address behind NAT) cannot be dialled by a phone, so row 1 (the phone dials an added address)
  needs a second phone as far end, or an owner-opened inbound port to this VM.
- No phone-side case adds an address or waits for an un-added peer: new case bodies in
  tests/android-e2e-cases/src/lib.rs (dispatch in `run`, cases::*), so a new APK built and installed by the
  installed signer (rust-ui-dev-01, install -r) on BOTH phones; ask for the sha256s. See
  [[workflow-shared-device-build-by-the-installed-signer]].
- The embedded host persists the bootstrap overlay under its root (crates/transport/embedded/tests/host.rs ~592).
- "Fresh" = pm clear of the app: needs the owner's word (shared Pixel).
Asked of the owner via architect-cto: Pixel 9a + A40 on one non-isolating Wi-Fi with wireless adb to this host,
the word to clear app data, optionally the inbound port.
Run files follow tests/android-e2e/runs/2026-10-22-pixel-9a-api37-wifi.md's shape; send paths to architect-cto.
Related: [[j63-wifi-device-run]], [[j65-superseded-state]].

2026-10-24 (session 301d0a8c): architect-cto REPLY 01a16c4b agreed the plan, owner asked for phones/clear/port.
Cases pushed at 173f4dfb (j63 harness merged unrebased 373a9937): added_address, await_peer, pair in
tests/android-e2e-cases/src/pairing.rs (NEED_A_RUNTIME, no app change); driver tests/android-e2e/tests/pairing.rs,
AdbDevice::connect_to(serial), wifi::reversed_on. Stand-ins green over the private IP; mutations: overlay not fed
to provider -> row 1 fails after restart; B trusting in apart step -> fails. Measured: A trusting alone, A's row ends
last_outcome connected (B closes at establishment) -- assert no message accepted, not no connection. Sent REPLY
01a16c61 with the commit. Next: owner places phones -> rust-ui-dev-01 builds/installs 173f4dfb (or later) on both,
sha256s; pm clear both (owner's word); run row 2 then row 1 with SERIAL_A/_B, WIFI_ADDRESS_A/_B; write two run files.
Device apart window 15 s. PR after j65 lands.
2026-10-24 11:40Z: owner placed phones (architect-cto REQUEST 01a16db2 -> job j67): Pixel a private-network address:43637 connects
unpaired (API 37, app still 10-22 build); Samsung a private-network address:36456 TCP open, adb connect fails = needs `adb pair`
(code + pairing port asked via REPLY 01a16db2-ea6d). Proposed running row 1 (no wipe) first. No pm clear without the owner's word.
11:55Z: both phones connect with MY key (A40 SM-A405FN API 30 accepted it, no new pairing). Host adb port 5037 holds one
account's server at a time: I killed mine for rust-ui-dev-01's install; restart after their sha256s. Both on 10-22/23 builds.
12:16Z row 1 first run on 173f4dfb (app 41e32046, test 1d289d62) failed: await_peer ended on seeing the connection,
closing it in 49 ms; fixed de47200d (pushed), reinstall asked of rust-ui-dev-01 (REQUEST 01a16ddc), architect-cto told
(INFO 01a16ddc-4f3a; the wipe is the one owner question left). Run script: scratchpad j67-run.sh <test> <log> under
`fabric-lease pixel-9a -- fabric-lease samsung-a40 --`, own adb port 5047; A40 refuses the first connect after a server start.
12:34Z run 2 on de47200d (app 35feb213): the mirror -- A's case passed on seeing the connection, runtime stopped 51 ms
later, B missed it. Fixed 96c2a52b: added_address holds 5 s (hold_ms) and removes EVERY earlier added address for the
peer (each run's port is new, so stale ones accumulate and are dialled at start). Reinstall asked (REQUEST 01a16df2-f595).
Run script now waits until adb lists both as `device` (either phone refuses/drops the first connect after server start).
Also reported Dependabot #22 (pip brotli 1.2.0, tools/requirements.txt, devex-tooling's) as OBSERVATION 01a16df2-b591.
13:00-13:03Z ROW 1 DONE: 4/4 pass at 96c2a52b (app e484d773), run file tests/android-e2e/runs/2026-10-24-pixel-9a-a40-wifi-added-address.md
at 5278c620 (pushed); reported REPLY 01a16e04. Only blocker for row 2: owner's word for pm clear on both. Row 2 run:
fabric-lease pixel-9a -- fabric-lease samsung-a40 -- scratchpad/j67-run.sh on_two_phones_two_fresh_profiles_connect_after_both_trust_and_not_before <log>.
After pm clear, AdbDevice::connect reads the new PeerIds; facts script scratchpad/j67-facts.sh.
13:08Z owner's word "clear interweave data on both phones"; cleared (scratchpad j67-clear.sh, pm clear before each row-2 run).
Row 2 at 96c2a52b: runs 1,3 pass, run 2 fail: first side required close accepted -> outcome unknown (second ends on reading
it). Fixed cc64aa55: close outcome reported (key close), first ends once second left; host ends B before A's result.
Reinstall asked REQUEST 01a16e0f. j69 (owner: cache-on row, reboot row, Scan re-run) plan sent REPLY 01a16e0f-e454:
cache variants a/b/c (needs a case arg + build), reboot needs new adb port each reboot (no boot receiver in manifest),
Scan cases are rust-ui-dev-01's when their Show/Scan PR opens.
architect-cto ruled (INFO 01a16e10): reboot row proves survival + dial at next start; "no boot receiver by decision"
(product rule today; a boot start would need its own record). Cache 3-variant row and reboot-port handling agreed.
13:29Z ROW 2 DONE: 3/3 at cc64aa55 (app ef21a7a3), 5/6 overall; run file .../2026-10-24-pixel-9a-a40-wifi-fresh-pair.md at 514f1fad.
j67 delivered (REPLY 01a16e1c). Branch: 14 work commits beyond main; PR after j65 (offered architect-cto to swap order).
Next: j69 (cache-on 3 variants needs a case arg + build; reboot row needs new adb port per reboot; Scan with rust-ui-dev-01
whose camera Scan PR targets 2026-10-25, #409 Show merged 1a79963b).
13:33Z architect-cto REPLY 01a16e1d: OPEN j66's PR NOW, j65 waits (j70, high). In the fresh session: worktree
../InterWeave-j66-stage-21-close-the-two-device, branch test/j66-..., head 514f1fad, 14 work commits (j63's 3 in it).
Steps: fold origin/main unrebased (j63 commits also sit on j65's branch -- fine), cargo build daemon+transportctl then
cargo test --workspace (memory: full workspace before arm; host-atspi desktop-e2e cases red on this host, mdns flake fixed),
clippy -D warnings, fabric-review brief -> code-review on fable (new MCP review tools need the fresh session), post with
fabric-pr post-review, threads 0, fabric-pr arm (14 >= 8; tests/ paths likely no boundary). PR body: the two run files,
figures, limits; j63 Wi-Fi run is folded in. Then tell architect-cto the PR number; they write plan (e) + closing record.
j69 paused: its branch feat/j69-j67-the-owner-s-word-on was made off main by switch -- refresh it from main AFTER j66 lands.
13:40Z (j70, session 79672b42): origin/main 652fd678 merged in (head 1775d6e8, pushed); PR #417 opened, unarmed 14 work/0 fix;
architect-cto told (REPLY 01a16e22). Built into j65's target via CARGO_TARGET_DIR (disk 17G free). Review dispatched on 652fd678..1775d6e8.
13:55Z: review F1-F4 posted; fixes 490da1c6, 5e2ed6fa, b2b5db6d (wording only); re-review 1 posted; re-review 2 in flight on 490da1c6..b2b5db6d.
F1 open part: gate "connect ... not before" vs run 4 handshake -> QUESTION 01a16e2f to architect-cto (reply expected).
Disk: 17G free at start; full suite filled to 4G -> stopped, deleted j65 target/debug/incremental (11G); rerun with CARGO_INCREMENTAL=0 + 6G guard (ws-test2.log).
14:20Z: #417 ARMED at d0ffada1 (14 work, 5 fix); review + 4 re-reviews posted; architect-cto REPLY 01a16e31 confirmed the gate's "connect"
= admitted connection a message crosses (they write it in (e) + closing record). Owner's disk cleanup (fabric-coordinator INFO 01a16e41):
removed j65 target (23G) + ~/.gradle/caches (689M); full suite never finished locally, CI rust is the test record. Next after merge:
tell architect-cto, then j69 (refresh its branch from main), j65 resumes.
14:36Z: #417 MERGED 1d780d34; architect-cto told (INFO 01a16e59). rust-ui-dev-01 takes the Pixel for camera Scan and writes
the scan-driven row 2 UI half on my pair case (REPLY 01a16e57-9e84). j70 done; j69 next (refresh branch from main), then j65.

*References: j63-wifi-device-run, j65-superseded-state, workflow-shared-device-build-by-the-installed-signer*

*Observed 2026-10-24 (p2p-network-dev)*
