---
role: "rust-ui-dev"
class: threads
topic: "j95-release-run-handoff"
description: "j95 DONE: #652 merged f38fd581d (A40 release-profile record, Back past first go, TM-57); held-Back fix is j107"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d75aa84ce3cec495
---

## j95 DONE: #652 merged f38fd581d (A40 release-profile record, Back past first go, TM-57); held-Back fix is j107

The owner asked for this work on 2026-11-12: "take the A40 and do the release run", then "do the 0.6 s Back run too".

**Branch.** `develop-qzapp/rust-ui-dev-02/test/j95-a40-release-run`, in working copy interweave-j95-long-messages-second-batch-the-cut.
- It is cut off main 6e52954cb, which is now about 52 commits behind origin/main. Nothing is committed on it and nothing is pushed.
- The uncommitted changes are tests/android-e2e/src/screen.rs and tests/android-e2e/tests/long_message.rs. A copy is in ~/agent-fabric-tmp/j95-review/back-case.patch.
- clippy and fmt are green on that diff. The commit was refused only because of ADR-037 rule 3 (a fresh session per job).
- The diff does three things:
  - `read_whole_then_back` stamps the phone time of each key (enter-at, back-at);
  - the helper `back_while_opening(names, after)`;
  - a new test `on_a_device_back_past_the_first_go_closes_the_reader_to_its_conversation` (0.6 s; heading autolinks, reference link, distinct headings, distinct links).
- Suggested commit message: "android-e2e: Back 0.6 s into an opening, on four shapes, with each key's phone time" (Kind: work).

**The build.** It is the debug variant (trace, case entry, stand-ins) with the native library at the [profile.release] settings, passed through `CARGO_PROFILE_DEV_*`: thin LTO, one codegen unit, debug 1, overflow checks ON, debug assertions OFF, opt-level 3.
- Script: ~/agent-fabric-tmp/j95-review/build-release-profile.sh. Output: app 48833554, test 4db8c691, signer 26d1eb18.
- Verified: rustc ran with `-C opt-level=3 -C codegen-units=1 -C debuginfo=1 -C overflow-checks=on`, and the UI crates came back Fresh under the same environment.
- A real release APK cannot be timed: it has no trace, no case entry and no stand-ins.

**The phone.** The A40 held a release 0.3.0 (signer a7f38b5e, app 3b89be89, installed 2026-11-12 16:42). It was uninstalled on the owner's word; its owner is unknown.
- The A40 now holds my probe16 app 80840ce0 and test 4db8c691.
- stay_on_while_plugged_in is back to 7 (awake-run.sh saves and restores it).

**Timed case: five complete rounds** (ab/release-complete: r1, r3, r4, r5 and the extra round r6; r2 lost to adb exit 255 on wireless). Median (min-max) in ms:

| shape | open |
|---|---|
| paragraph | 146 (140-152) |
| list | 370 (365-371) |
| code | 355 (352-358) |
| table | 358 (357-362) |
| headings | 371 (366-372) |
| images | 403 (401-407) |
| heading autolinks | 513 (510-514) |
| reference link | 752 (748-760) |
| distinct headings | 1652 (1631-1658) |
| distinct links | 1311 (1309-1325) |

End is at most 65 ms formatted and 24 ms source, in one frame. These equal the A/B candidate within 4 ms, so the release settings change nothing measurable.

**Back runs.**
- Data is in ab/back200 (list, 0.2 s, one round), ab/back600-two (two shapes, three rounds) and ab/back600 (four shapes, three rounds), with recordings. The analysis script is back-times.py.
- The `input` command takes about 0.47 s to start under screenrecord. The script estimates when the Back arrived from the Enter's own delay, worked out as drawn time minus the drawn ms.
- Heading autolinks and reference link open in 0.25 and 0.41 s when they are alone in a conversation, so their Back lands after the rows. The close then comes 82-205 ms after the Back, and no bar shows, as expected.
- List at 0.2 s: the Back arrives at 272 ms, inside the first go, and the close comes at 622 ms. That is 271 ms after the rows, which suggests the source build after the first frame. This is not verified.
- Stress shapes: the Back arrives at about 0.67 s, inside the opening. The recordings show the empty reader, then the indeterminate bar moving. The close comes only when the build ends, 0.87-0.96 s after the Back. In two of three distinct-headings runs the rows were never drawn before the close.

**FINDING.** A Back held during a long opening, localised by probes that have since been removed. In the probe runs ab/probe-runs and ab/probe16-runs, Java's onBackPressed (`back=asked`) comes only at the build's end, and from there the close takes 80-220 ms. Changing the step timer from 1 ms to 16 ms made no difference. So the Back is held before onBackPressed, in NativeActivity's input path: either the native input queue, or InputDispatcher waiting on the Enter. It is not caused by the opening's timer.
- Next step: probe the Slint android backend's process_inputs (i-slint-backend-android-activity 1.18.1) or android-activity 0.6.1's native input queue, or try a build that yields to the looper in a different way.
- It still meets ux's "close within 1 s" for now, by 40-130 ms.

**Status 2026-11-13 (session 6368d59c):** steps 1-5 done on PR #652 (head dad016cc4, 4 work 7 fix).
- Blind review + rr2 + rr3 posted (rr3 clean). F2 stays "partly" only for the plan line 7477-7479, which is architect-cto's: defer it at arming with `--defer "dad016cc:F2" "<plan line is architect-cto's dated record; handed over in REPLY 01a1d20f>"`.
- Below floor: asked architect-cto-01 for AUTHORIZES (REPLY 01a1d20f, seq 1068499). Arm with `--authorization <their msg id>`.
- ux-designer-01 ruled the held Back a miss of §5.4 (REPLY 01a1d1bd): their acceptance is close within 0.15 s, double Back not lost, A5 rows still pass, A40 release profile, 3 rounds, recorded. That is job j107, queued.
- The changed Back case (F5 focus equality, F6 stamp order) has not run on a phone.

Related: [[j95-plan]], [[j95-a40-ab-rig]].

*References: j95-a40-ab-rig, j95-plan*

*Observed 2026-11-12 (rust-ui-dev)*
