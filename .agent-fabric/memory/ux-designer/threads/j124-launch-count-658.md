---
role: "ux-designer"
class: threads
topic: "j124-launch-count-658"
description: "j124 delivered — #661 merged 05d38e9d5 (add-a-peer §5.1 / §8 item 12 after #658); owed: item 12 window cases on a phone, reattach check once #662 lands"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3fd39072465032ef
---

## j124 delivered — #661 merged 05d38e9d5 (add-a-peer §5.1 / §8 item 12 after #658); owed: item 12 window cases on a phone, reattach check once #662 lands

2026-11-13, PR #661 (branch develop-qzapp/ux-designer-01/feat/j124-j268-built-on-658-launch-count, 1 work commit b1d45e9a5).
- #658 (merged 4add2e9f8) read against §5.1: Root::attach counts from Chats showing, start_asked restarts it, closed menus count. Matches; not run on a phone.
- Decided (after review F1 reversed my first reading): a window made again while the service runs is NOT a launch; it shows what the start decided (the person cannot tell it from Home+return). A view attached again (hub detach: view >256 messages behind) is the same launch, 10 s go on.
- Build gaps, both rust-ui-dev-01's j274: reattached resets owed_since (REQUEST 01a1d5f2-c4ba); recreated Activity re-opens a deferred Connect (correction REQUEST 01a1d5f8-0e36).
- Item 12's queued case: no longer blocked; evidence the_rules_decide_before_the_turn_the_reads_open_connect_in + root's call in Root::pump. New device check: "Don't keep activities".
- Owed: LC pass (REQUEST 01a1d5f2-ccda to language-culture-en), blind review rounds, owner's arm word (under the floor).
Related: [[j121-launch-rule-pr655]], [[connect-owed-since-launch]], [[lc-pass-before-arm]].

Round 1 review 5895396040 (F1 P2, F2 P3) fixed in a84f1c782; LC pass for/ branch j310-661-english c07153cd4 based on b1d45e9a5, to fold after round 2, then a new LC pass of the a84 text.
Lesson: an Android implementation detail the person cannot see (Activity recreation) must not change what they see.
Round 2 (review 5895452802): F3 P3 fixed in b417dd906 (decided once per service start; launch narrower than unlock note's L3). LC j310 folded c60473b49 keeping new meaning; 2nd LC pass asked (REQUEST 01a1d5fc-826b). rust-ui-dev-01 took j274+j275 (REPLY 01a1d5f8-73b9). Round 3 re-review running on b417dd906.
Round 3 (review 5895529878): F4/F5 P3 + notification risk fixed in 0a0e13407; LC told head moved (INFO 01a1d600). Next: fold j311, one re-review over b417dd906..final, then owner's arm word.
j311 folded e39547280; 'fail' reworded e76cd35e8 (LC check asked 01a1d605); round 4 re-review b417dd906..e76cd35e8 running.
LC edit list for e76 sentence: REPLY 01a1d606 (apply after round 4).
Round 4 clean (5895653112); LC sentence applied 4d2bb4d0d; carried risks comment 6860699531; round 5 running.
Round 5 clean (5895676900) on 4d2bb4d0d; arm gate passes except count (3 work < 8): waits on owner's 'arm 661'.
rust-ui-dev-01 built j274+j275 on #662 (7e38b60ce..947069371), REPLY 01a1d63e; owed: device check of item 12 window cases on #662's build once a phone address arrives.
Delivered 2026-11-13: #661 merged 05d38e9d5 on the owner's word "arm 661" (3 work, 5 fix; review rounds 4-5 clean). CI's rust job failed once on an unrelated flake (tests/android-e2e/tests/pairing.rs:219, on_stand_ins_with_the_cache_on_and_neither_source_kept_nothing_dials); re-run passed.
Owed: rust-ui-dev-01's #662 (j274/j275) builds both gaps; when it lands, item 12's reattach check moves from blocked to passed (a_reattach_past_the_bound_gives_connect_up). The three "Don't keep activities" window cases need a phone (owner's Wireless debugging address).
2026-11-14 Pixel run (50107ae97, rust-ui-dev-01's pair, restored test phrase abandon x23 + art, keyboard only; owner's uninstall word via architect-cto 01a1d700):
- Case 2 (control) end state passes: Connect again after Home+reopen, focus connect-title.
- BUT item 12's method is wrong: with "Don't keep activities", Home destroys the window, NetworkService is bound-only, so it stops ("stop answered true"); the reopen is a NEW start (start answered 0), Chats shows ~3.6 s with connect=owed before Connect. Cases 1/3 not run (would only test a new start).
- Asked rust-ui-dev-01 when a window attaches while the same start runs, and how to make one (QUESTION 01a1d888-f0b2). Spec fix to item 12 is mine (new job). Pixel left: Connect open, settings restored (always_finish 0, DND off).
Lesson: before writing a device check that relies on a platform option, confirm on the build what that option does to the process/service lifetime.
2026-11-15 00:06Z: rust-ui-dev-01 answered QUESTION 01a1d888 (REPLY 01a1dc86): reattach to a running start happens (1) Stay reachable on (foreground service outlives binding; Home with Don't keep activities, reopen = new window over same start: no 'stopped:' at Home, no new 'start answered' at reopen) and (2) a locale/layoutDirection config change (not in configChanges) recreates with isChangingConfigurations true: adb shell cmd locale set-app-locales org.interweave.human --locales de (untried). Foreground-only + Don't keep activities: Home = service stop, reopen = new start. j127 unblocked.
2026-11-15 (j127): item 12 rewritten, PR #676 (99b563e0a, 1 work): cases 1-3 on Stay reachable + Don't keep activities, case 4 foreground-only via app-locale change (untried cmd); each case needs logcat both halves (views start; no stopped:, no 2nd start answered) else void. §5.1: which windows the build remakes; #662 replaced the two differences (host tests named). Found: foreground-only Home while C4/R4 held ends the start (code read) -> QUESTION 01a1dc8f-aba4 to architect-cto; Home-held check runs on Stay reachable, blocked on foreground-only. LC pass asked REQUEST 01a1dc8f-a557. Next: fold LC, blind review, owner's 'arm 676'.
2026-11-15 00:20Z: architect-cto ruled (a) (REPLY 01a1dc91): foreground-only Home while C4/R4 held ends the start, return = new launch; written c5e70a994 (Interruptions, item 12 runs both modes). LC j322 told to pass c5e70a994 (REPLY 01a1dc93). #676 2 work, 0 fix.
00:30Z: LC j322 b09b16202 folded (3f5a271ae); answers to its Q1 (neither -> 'run on a phone still owed') and Q2 (case 4: no stopped: at Home or at the language change) in 486947b3b; LC check asked REPLY 01a1dc9a. #676 4 work, 0 fix. Next: blind review.
00:35Z: LC edit 'still to be done' + rewrap d213f390e (5 work). Review r1 dispatched on a5c4d1905..d213f390e (~/review-reports/pr676.yaml, r1 brief). Post it while the job is active.
00:45Z: r1 posted (review 5908185510): F1 P2 kept window on foreground-only Home keeps held C4 (build: held->Connect <10s, Chats deferred >=10s; spec says new launch); F2 P2 Settings fallback for case 4 stops Activity -> void; F3 P3 Stay-reachable cases need force-stop + start answered after am start; F4 P3 item 12 restates §5.1 rule; F5 P3 line ~326 "(Interruptions)" now Stay-reachable only. Judgement dispatched by hand (fabric-review next --adjudicate needs a later head). Fix plan F1: state build difference, kept-window run expected to fail; destroyed-window variant passes.
00:50Z: judgement confirmed F1-F5. Fixes e771f4f7f F5, 391f477cc F4, 5a1d5ed6a F3, c2bfce910 F2, 106acdca4 F1 (build difference in §5.1: kept window holds C4 and earlier 10 s -> rust-ui-dev's to change; tell rust-ui-dev-01 after merge or now). Re-review r2 running (rr2.json); LC pass of fix range asked REQUEST 01a1dcac.
01:00Z: r2 clean, posted (5908267914). Fixed r2's two pre-existing P3s anyway (my PR made the first): 1b unlock-note comparison Stay-reachable only; f464f6e63 cases 3/4 void unless new window's android_main precedes connect=deferred. LC told (INFO 01a1dcb0). Next: fold LC, re-review r3 via fabric-review next --pr 676 --head <new>, owner's arm word.
01:00Z: LC j323 bc7f2eeff folded (fbc49d03b); Q1 reading A + Q2 each run twice in dc7362cff; LC confirmed both read right (REPLY 01a1dcb6-df4c). Re-review r3 106acdca4..dc7362cff running (rr3.json; next round chains from rr3). If clean: post, carried risks comment, ask owner 'arm 676'. After merge: OBSERVATION to rust-ui-dev-01 on the held-screen difference (§5.1).
01:05Z: r3 posted (5908325556): F6 P2 — my round-2 ordering void rule couldn't work (no window -> no trace of the deferral). Fixed 68c367ccd with a logcat timestamp rule (<10 s after first start=running). Lesson repeated: an extra fix taken from a review's pre-existing list cost a round. r4 running (rr4.json); LC check REQUEST 01a1dcbb.
01:06Z: LC edit list for F6 sentence (REPLY 01a1dcbb-caf7): 'that line is written later'; 'The order of the android_main line and the trace's connect=deferred line'; 'when it is first drawn'. Apply after r4.
01:07Z: r4 clean posted (5908338424). LC edits applied 5490f3cf1 (8 work). r5 re-review running (rr5.json). If clean: post, carried-risks comment (split-screen timing; ~9.9 s return margin; cmd locale untried), ask owner 'arm 676'.
01:10Z: r5 clean posted (5908347099). Carried risks comment 6887077722. ARMED #676 at 5490f3cf1 (8 work, 8 fix). After merge: OBSERVATION to rust-ui-dev-01 (held-screen difference §5.1), deliver j127, new job for item 12 phone run.
2026-11-15 ~01:20Z: #676 MERGED 5ed967157 (8 work, 8 fix; rounds 1-5, last two clean). j127 done. OBSERVATION 01a1dce4 to rust-ui-dev-01 on the held-screen difference (reply expected). Phone run queued as j129.
2026-11-15 ~01:55Z (j129): rust-ui-dev-01 took the held-screen change as j280 (REPLY 01a1dce5; activity.rs, reset in stopped() or the new start's attach not decided). The foreground-only option-off Home-held run waits on j280. Asked architect-cto-01 for the Pixel address and a slot (QUESTION 01a1dce6); j129 blocked on it. Build needed: debug main >= #662 e4a962472 (launch extras); read the installed build first.
04:36Z: architect-cto REPLY 01a1dd7d: Pixel slot is mine now, first; address follows from the owner. Finish in 90 min, set back what I changed, then INFO p2p-network-dev-02 (j110 uninstalls both apps with data after me; Pixel then unplugged ~6 h). The later foreground-only run after j280 is a new slot on a fresh install: ask architect-cto then. Scripts in session scratch run/ (lib.sh, step0.sh, mode.sh, case.sh). Build 50107ae97 (last run's) contains #662.
04:46Z: owner's address a private-network address:44361 refused adb connect (ping ok, TCP open, "failed to connect" x6); asked architect-cto for the Wireless debugging page port + paired list (REPLY 01a1dd88; they asked the owner, 01a1dd89). Slot not started.
05:08Z: rust-ui-dev-01 REPLY 01a1dd9a: j280 branch develop-qzapp/rust-ui-dev-01/feat/j280-add-a-peer-s5-1-foreground (unpushed): a held C4/R4 whose start ran then stopped without refusal ends at once into Chats, Connect stays owed, no deferral; the return's start is a launch. Option-on foreground-only run should pass on main except a stale defer_next if the old window outlived 10 s stopped; j280 removes that too.
2026-11-15 05:16-05:32Z Pixel run (j129, build installed 2026-11-14 09:21 = 50107ae97, contains #662; owner re-paired, connect :38703): cases 1, 2, 3 PASS on Stay reachable (overlay file written then removed) + Don't keep activities. `settings put global always_finish_activities` does NOT apply: the owner had to toggle it in Developer options (off/on). Case 2: the new window writes page=conversations ~57 ms before page=connect. Case 4 waits on the owner toggling the option off. Logs: session scratch run/out/.
05:57Z rust-ui-dev-01 INFO 01a1ddc7: #678 (278feda79) builds j280; §5.1 build paragraph and item 12 "expected to fail" line are mine to revise after it merges. Unrun limits: Home before the service exists (holdStartMs), hub queue full, restart across two pumps.
06:36Z: released the Pixel without case 4 (owner did not get to the DKA off toggle inside the slot). INFO 01a1ddea-c22f to p2p-network-dev-02: phone theirs; DKA reads 0 but is still applied ON until the owner toggles on/off. #678 merged a6a6fedf6 (INFO 01a1ddea-47a5); spec revision queued as j130. j129 blocked on the next slot: case 4 + both foreground-only Home-held runs on a fresh install >= a6a6fedf6.
Logs of the 2026-11-15 run (no screenshots): ~/j129-pixel-run/run/out/{case1-*,case2-052635,case3-052821} (steps.txt, c.log). Remove once j130 lands.

*References: connect-owed-since-launch, j121-launch-rule-pr655, lc-pass-before-arm*

*Observed 2026-11-15 (ux-designer)*
