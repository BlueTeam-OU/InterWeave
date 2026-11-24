---
role: "ux-designer"
class: threads
topic: "a5-back-held-stress-shapes"
description: "Long-message reader: Back during opening on stress shapes held to build end (~0.9 s) on A40; fix asked of rust-ui-dev-02; A5 run list to gain the stress shapes"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7b4ded4e8530df24
---

## Long-message reader: Back during opening on stress shapes held to build end (~0.9 s) on A40; fix asked of rust-ui-dev-02; A5 run list to gain the stress shapes

2026-11-12. rust-ui-dev-02's INFO 01a1d1bb (A40 release-profile run, branch
develop-qzapp/rust-ui-dev-02/test/j95-a40-release-run): on distinct-headings and
distinct-links shapes a Back at ~0.6 s closes only when the build ends, 0.87-0.96 s
later (input held before the activity sees it). Passes A5's 1 s, misses §5.4 "closes
at the next gap between turns". Replied REPLY 01a1d1bd: not acceptable; fix asked
with acceptance (close within 0.15 s, double-Back not lost or merged, A5 rows still pass).
Taken by rust-ui-dev-02 as their job j107 (REPLY 01a1d1be), after j95's PR lands; j95 record and TM-57 to say the stress shapes miss §5.4.
Owed by me: add the stress shapes to A5's run list once the fix's figures arrive.
Related: [[j118-reader-opening-back]].

2026-11-13: the figures have arrived. REPLY 01a1d356 says the fix is on PR #656 (head aacf451d9). On the A40, acceptance 1-3 pass:
- A Back on the stress shapes closes in 20-118 ms.
- When Back is pressed twice, both presses register, neither lost nor merged.
- Opening a message takes 1.02 s for distinct links and 1.41 s for distinct headings; every other shape opens in under 0.31 s.

Not yet covered: gesture Back and the app bar's Back, TalkBack, the Pixel, the release APK, and one first Back that was lost on an earlier build.

Still owed by me:
- a hands-on review on the phone of #656;
- adding the stress shapes to A5's run list as a separate job after #655.

2026-11-13, the owner's choice in session for j122: run the review on the Samsung A40, not the Pixel. Release v0.2.0 stays on the Pixel with its data.
Next steps:
- ask the owner for the A40's Wireless debugging IP:port;
- ask rust-ui-dev-02 which build is on the A40 (#656 head aacf451d9 or main), and for a data-kept install if needed;
- the samsung-a40 lease was free at 08:40Z.
End the run in a stated state.
2026-11-13 ~14:40Z, j122 is blocked on the owner for the A40's IP:port. Asked in session, no answer yet.
- **Ready:**
  - ~/j122-scratch/back-cmd.patch, never to be committed. It makes the Back command come from env vars J122_BACK_CMD and J122_BACK2_CMD in tests/android-e2e/src/screen.rs read_whole_then_back.
  - ~/j122-scratch/a40.sh <ip:port> <cmd>, a wrapper that takes the samsung-a40 lease and an adb server on its own port.
- **Plan:**
  - On the stress shapes, a gesture Back (`input swipe 5 <y> 500 <y> 150` on the left edge, gesture navigation on, and restore the setting afterwards) and an app-bar Back (a tap; read the coordinates from a screenshot).
  - Each about 0.6 s after Open the whole message, and double at 0.3 s.
  - Pass: closes within 0.15 s, focus on the row's Open the whole message, neither Back lost or merged.
- **Before the run:**
  - check that the A40 holds rust-ui-dev-02's clean build c3f15adf5 (app d0209344, test 4db8c691). It lacks the 300-firing cap.
  - set ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION.
- End on the chat list.
2026-11-14 ~01:30Z (phone 02:07-02:39), j122 run on the A40, rust-ui-dev-01's pair of main 41bb631b9 (app 077e4183, test 1efb4b15), no install, no wipe, samsung-a40 lease. Scratch in session scratchpad a40/runs/{control-key,navbar-back,navbar-double,appbar-back}; patch ~/j122-scratch/back-cmd.patch applied uncommitted in interweave-j122-... (detached at 41bb631b9).
Back-to-close, measured from InputDispatcher delivery to `ui-trace page=conversation`:
- key Back (control): A5 shapes 20/45 ms, stress 23/24 ms.
- nav-bar Back button (tap 840,2267; reaches the app as a key): 29/21, stress 80/118 ms.
- double nav-bar Back on stress: first 69/84 ms, second to Chats 51/65 ms; none lost or merged.
- app-bar Back (tap 104,216; from touch-down): 142/126, stress 62/88 ms.
All pass (0.15 s), focus back on read:N. Gesture Back not set up: on Android 11 it injects the same key as the nav bar.
Seen, unverified: after the 2nd Back reaches Chats, the trace reports focus=read:0 (a hidden conversation control). Recheck with the harness; no spec rule names focus on Chats after leaving a conversation (swipe note covers only the snackbar case).
j123/j116 need R2 = no record or K1: asked architect-cto for pm clear (QUESTION 01a1d711).
2026-11-14: reported to rust-ui-dev-02 (REPLY 01a1d840-cc92). Still owed: stress shapes into A5's run list (design change). R2 24th-chip focus bug found during j116 is rust-ui-dev-01's j279 (also seen on the Pixel: 28 Tabs to Check my words).
2026-11-14 later: A5 stress-shape edit written and reviewed by me, saved as ~/j122-scratch/a5-stress-shapes.patch (a diff of design/2026-11-04-long-message.md against origin/main at that time; uncommitted because the session had worked j116). It adds the Back-key, nav-bar and app-bar Back runs on distinct headings and distinct links (pass: close within 0.15 s, two Backs neither lost nor merged) and puts the runs into a list (2026-11-12 list shape; 2026-11-13 record; 2026-11-14 main 41bb631b9 62-118 ms). Next: apply it on a branch off origin/main, commit Kind: work, LC English pass, open the PR (j127's item 12 rewrite may join it). The interweave-j122 working copy is back on its branch, with the scratch patch dropped.
2026-11-14: patch applied as 235b6dbc9 on the j122 branch, PR #668 open (1 work commit, unarmed). English pass asked of language-culture-en (REQUEST 01a1d8ed). Then: fold their pass, blind review, owner's word to arm. ~/j122-scratch can be removed once #668 merges.
2026-11-14 ~07:40Z: LC pass f9af1869f folded (merge 4c663ac39); my answers to its 3 questions in bb6646f17 (pair by each way, first at 0.6 s; 0.31 s = 11-13 build; start points; 'opening finished building' restored). LC pass of bb6646f17 asked (REPLY 01a1d8f5). #668 now 3 work, 0 fix.
2026-11-14 ~07:50Z: LC 2nd pass 9be77e629 folded (0e8ebf3f5). LC OBSERVATION 01a1d8f8: 0.6 s Back on autolinks/reference comes after first screen on a passing build. Decided on #668 (abac13098): the past-first-turn Back runs on stress shapes only; a run whose first screen precedes its Back is void; 1 s close inside first turn, 0.15 s past it; desktop same by Escape. LC pass of abac13098 asked (REPLY 01a1d8fa). #668 5 work, 0 fix.
2026-11-14 ~08:00Z: LC pass done (REPLY 01a1d8ff); rewrap 0af80bc21. Review r1 posted (F1 P2 list 0.2 s Back lands after 179 ms first screen; F2 P3 runs list void runs; F3 P3 0.3 s pair can't test merge). Fixed in f4f806c44 (Answers trailer): list Back at half its first-screen time, void rule both runs, new 'two Backs pressed together' run. Re-review r2 dispatched (~/review-reports/pr668-rr2.json). Owed after merge: tell rust-ui-dev-02 the test's BACK_AFTER 200 ms and the test doc naming autolinks/reference as 0.6 s shapes are stale, and the new pressed-together run. The fix's new words also need LC's pass.
2026-11-14 ~08:10Z: #668 review rounds r1-r4 posted; fixes f4f806c44, 541786413, f794a15ac, 63634d883. Lesson: my F3 fix (new pressed-together run) widened scope and cost 3 rounds (F4-F7: arrival point, desktop has no key time and no visible second Escape). Final: two-Back runs Android-only, timed by press sent + close on phone uptime clock; desktop single Escapes only. r5 dispatched (rr5.json). LC pass owed on everything after 0af80bc21 before arming.
2026-11-14 ~08:35Z: #668 armed at 7d1ff8907 (9 work by arm's count, 6 fix; 8 review rounds, 9 findings fixed; LC passes folded). After merge: INFO/REQUEST to rust-ui-dev-02 on device-test changes (BACK_AFTER -> half first-screen time; pressed-together run; uptime-clock send stamps; stale doc naming autolinks/reference 0.6 s shapes; desktop single Escapes timed from sending tool). Then deliver j122.
2026-11-14 ~08:50Z: #668 merged f359bf797. Device-test follow-ups sent to rust-ui-dev-02 (REQUEST 01a1d954). j122 delivered; ~/j122-scratch removed. Owed: A5 runs list gains figures when rust-ui-dev-02's runs come.
2026-11-14 09:13Z: rust-ui-dev-02 took all four as their j109 (REPLY 01a1d955): new in-app sender stamping uptime per press, close from logcat -v monotonic; will come back if the sender can't run beside the open app. Branch name to follow.
09:17Z: rust-ui-dev-02 plan (REPLY 01a1d958, branch develop-qzapp/rust-ui-dev-02/feat/j109-long-message-a5-back-runs-changed): app_process sender in androidTest APK, uptime per press, logcat monotonic; list Back 90 ms. Told them plan matches; desktop reader-opening case wanted as own job after j109 (REPLY sent).
10:02Z: rust-ui-dev-02 delivered PR #669 (head 923d6e4e), not yet run on a device; desktop Escape case is their j110. Owed by me: A5 runs list gains figures when #669's device runs come.
13:38Z: desktop Escape runs (rust-ui-dev-02 REPLY 01a1da47, 533ce31df): list 9/9; stress 15/17 within 0.15 s, 2 late under load (quiet-host rerun owed), 1 void (distinct links first screen 583 ms). Decided: keep 0.6 s, void stands; if >1/3 void on quiet host, set a desktop time between 0.5 s and first screen. j128 queued+blocked for the runs-list update.
2026-11-15 06:47Z: rust-ui-dev-02 QUESTION 01a1ddf5 (quieter host, #677, apps/human-desktop/runs/2026-11-15-escape-during-opening.md): list 9/9; distinct headings 8/8 pass (15-76 ms; 1 void at 567 ms; the late closes gone); distinct links 9/9 void (rows line 391-497 ms). Answered (c) provisionally: desktop distinct links carried as not judged (no bar state exists; 0.3 s would be the inside-first-turn case; headings covers past-first-turn on desktop). Write into A5 runs list in j128; tell rust-ui-dev-02 if review changes it.

*References: j118-reader-opening-back*

*Observed 2026-11-15 (ux-designer)*
