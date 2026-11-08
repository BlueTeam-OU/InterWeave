---
role: "p2p-network-dev"
class: threads
topic: "j69-cache-and-reboot-rows"
description: "j69 (owner's limits on row 1): cache-on row in 4 variants + reboot row built at 837663a2; device runs wait on rust-ui-dev-01's APK; Scan re-run later (2026-10-24)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 7aad0e914425128e
  - d2010a7ba8ef5e0e
---

## j69 (owner's limits on row 1): cache-on row in 4 variants + reboot row built at 837663a2; device runs wait on rust-ui-dev-01's APK; Scan re-run later (2026-10-24)

Branch develop-qzapp/p2p-network-dev-02/feat/j69-j67-the-owner-s-word-on, worktree ../InterWeave-j69-j67-the-owner-s-word-on.
837663a2 (2 work commits on main 1d780d34): added_address gains remove_after / listed / apart_ms; new no-runtime case
peer_cache (reads/forgets ProfilePaths::peer_cache_file, i.e. <root>/.../cache/peers.json); driver `added_address_across`
with Sources {OverlayOnly, Both, CacheAlone, OverlayAlone, Neither} (Kademlia off: it is seeded from the cache), and the
reboot row on_two_phones_an_added_address_survives_a_reboot (adb::reboot_and_await: same ro.serialno, new boot_id,
asserts pidof app empty = no boot receiver; INTERWEAVE_ANDROID_REBOOT_PATIENCE_S default 1200; someone must
`adb connect` the new port).
Stand-ins: 6/6 green; after a stand-in kill the cache file named B every time (a phone force-stop skips shutdown's
final flush: the device run measures it).
Asked rust-ui-dev-01 for the APK at 837663a2 on both phones (REQUEST 01a16e8c); architect-cto told (INFO 01a16e8d).
Next: run the 4 cache tests then reboot (ask owner via architect-cto for the new port), write 2 run files in
tests/android-e2e/runs/ shape, send paths; then the Scan re-run of row 2 with rust-ui-dev-01 (their UI case on my
pair case), then PR. Also: j71 delivered (devex 1.99.1 supply 587ba286..19bcac02, REPLY 01a16e72).
Related: [[j66-stage-21-device-rows]], [[workflow-shared-device-build-by-the-installed-signer]].
2026-10-24 16:00-17:05Z: cache row 12/12 on phones (APK 960add40/1d289d62 by rust-ui-dev-01); reboot row run 1 timed out
(no port relayed in 20 min), run 2 PASS 16:57Z (owner read .53:45969 before, .54:44599 after: the Pixel's IP moves per boot).
Run files pushed a8db42e8 (cache-on.md, reboot.md); paths sent REPLY 01a16ee8 with question: hold for Scan re-run (#418)
or PR now on owner's word (4 work commits). architect-cto ruled one reboot pass is the record (01a16ed9).
Run script scratchpad/j69-run.sh (SERIAL_A override env; port 5047). Lesson: eprintln lands mid-line after the test
name, so watch for the text anywhere, not ^rebooted.
17:16Z architect-cto chose (b) (REPLY 01a16ee9): PR now; Scan re-run a later PR. Opened #421 at a8db42e8 (4 work, 0 fix),
unarmed. Arms on architect-cto's AUTHORIZES (below-minimum authority) after a clean blind review, 0 threads, dispatched
from a FRESH session (their ask): fabric-review brief -> code-review on fable, fabric-pr post-review 421, then send them
the head; arm with --authorization <their msg id>. j65's PR follows #421.
17:49Z blind review posted on #421 (a8db42e8): F1 P2 real -- on phones the Neither control ran with no B case, so B
was not listening (device case = process). Fixed bf114692 (await_peer `apart` window, driver runs it, asserts B
outlives A) + 29e92dda (run file says rows 4/8/12 were no control; Both = baseline, F2). Stand-ins 6/6, mutation
(keep cache under Neither) fails. Answer comment posted. Re-review a8db42e8..29e92dda dispatched. Phones owe a re-run
of the four variants at the new head (APK from rust-ui-dev-01) -- tell architect-cto; not needed to arm? ask them.
17:56Z re-review 1 (29e92dda): F1 F2 fixed, F3 P3 (run file "at that commit" named old driver) -> da6fdbe4; re-review 2
clean, posted. #421 at da6fdbe4 (4 work, 3 fix, 0 threads). Sent architect-cto REPLY 01a16f0e: (a) AUTHORIZES now,
phone re-run of four variants in the later Scan PR, or (b) hold for re-run. Waiting their answer.
17:58Z architect-cto AUTHORIZES #421@da6fdbe4 (msg 01a16f10-1066-7714-a5db-c1e2a1ba05bb): arm when checks green,
`fabric-pr arm 421 --authorization 01a16f10-... --basis ...`. Stage 21 close waits on ONE later PR of mine carrying
both: the four cache-on variants re-run on phones at bf114692+ and the Scan-driven row 2 re-run; after #418 merges and
an APK at its head exists. Name that PR to architect-cto when it opens.
2026-10-25: #418 merged (a52cb3e2). New branch ...-on-2, worktree ../InterWeave-j69-j67-the-owner-s-word-on-2 at main 8f978966
(run script copied to this session's scratchpad, cd fixed). Asked rust-ui-dev-01 (REQUEST 01a16fef): APK at 8f978966 on
both phones, data kept; split proposal: their screen-driving case bodies on a for/ branch, my driver/assertions/run file/PR.
Order sent to architect-cto (REPLY 01a16ff0): 1 cache-on re-run (no wipe), 2 Scan row 2 (owner's wipe word; camera may
need the phones facing). Add page has a text field too, so pasted-text is the fallback route.
rust-ui-dev-01 (REPLY 01a1700c): APK at 8f978966 building. A UI case cannot run under E2eCase (no Activity; Slint has no
Android a11y tree), and Show sets FLAG_SECURE, so a screencap is black. Their split: host-side screen driving via injected
keys + adb input text on for/ branch develop-qzapp/rust-ui-dev-01/for/p2p-network-dev-02/scan-path-ui; the code's text
comes from own_addresses. I answered (01a1700d): both routes, camera first then field; check TAB/ENTER focus on the A40
first. Join: a UI step in the app process, then stop the app, then my instrumentation case; trust must persist across the stop.
rust-ui-dev-01 01a17013: trust/add written to the overlay before the answer (LOCAL-IPC.md ~390, checked on main); stop the app only after the screen shows the answer; their done signal comes from the log or the store. APK still building.
23:12Z APK at 8f978966 built: app daf30998..., test 1d289d62...; not installed: rust-ui-dev-01's adb refused on both; mine connects Pixel .54:44599, A40 .6:36456 refuses mine too (told them 01a17030). Waits on owner re-pair.
01:34Z cache-on re-run 12/12 at 8f978966, neither control live; file 2026-10-25-...-cache-on-control.md committed f3663d9c (1 work), pushed, no PR yet; sent architect-cto 01a170b4. Next: Scan row 2 after rust-ui-dev-01's focus check + supply + owner's wipe word.
01:37Z rust-ui-dev-01: TAB/ENTER/ESC drive Slint on the A40; they write the screen-driving commits now (force-stop+launch per step, screencap per step as evidence).
01:40Z rust-ui-dev-01 adds a debug-only logcat UI trace (page, focus key, outcome kind; no address/PeerId/name) in ui-slint + human-android on the same for/ branch; I agreed it rides my PR (01a170b6-f80f), asked for a release-gate mutation and a no-PII test; new APK at PR head after the supply.
03:04Z rust-ui-dev-01: both phones now run their trace build (8f978966 + first trace commit), not 8f978966; their adb refused again (owner re-pair); 4 commits local on scan-path-ui, pushed after a read-only screen test passes on a phone. (Their clock times look phone-local UTC+4-ish.)
05:04Z scan-path-ui pushed 86ff7386 (8f978966..86ff7386, 4 work) - NOT a supply; don't fold until they send case names + passing tests/screens.rs on a phone. They paused for j159 until phones re-paired.
2026-10-25 16:40Z (j73, architect-cto REQUEST 01a173d9): opened #448 from ...-on-2 at the pushed f3663d9c (the local
stale merge c537a560 of old main dropped, never pushed). Review clean of P1/P2, two P3 wording (run 1's time excludes the
build; add range 1015-1529 ms, not 1.0-1.3 s) fixed af3cfcd7, re-review clean, both posted. Asked architect-cto for
AUTHORIZES #448@af3cfcd7 (REPLY 01a173e5); arm with --authorization <id> once checks green. No owner wipe word yet.
2026-10-25 ~17:05Z: architect-cto AUTHORIZES 01a173e7; #448 armed and MERGED 4ffe79c5. j73 done; told architect-cto (01a173fc). Only Scan row 2 remains of j69, on the owner's wipe word.
2026-10-31 16:55Z rust-ui-dev-01 asked (QUESTION 01a192e3-4233) to fold scan-path-ui (e3234d2b, 7 commits, their lane) or release it. Released (REPLY 01a192e3-8501): they land it in their own PR off main. Row 2, on the owner's wipe word, branches off a main that has their PR; if still open then, ask them for the range, never fold it unmerged.
2026-10-31 18:18Z: rust-ui-dev-01's #527 merged 817ac44b (scan-path-ui on main; trace adds connect=<phase>, launch fails naming Connect if onboarding's Connect is owed). Row 2 branches off main at/after it.

*References: j66-stage-21-device-rows, workflow-shared-device-build-by-the-installed-signer*

*Observed 2026-10-31 (p2p-network-dev)*
