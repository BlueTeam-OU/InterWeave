---
role: "rust-ui-dev"
class: solution
topic: "j109-a5-back-runs-keysender"
description: "A5 Back-while-opening device cases (PR #669): KeySender on the uptime clock, back_run judge, what is unproved until a device run"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5f6e41ad2bd19f38
---

## A5 Back-while-opening device cases (PR #669): KeySender on the uptime clock, back_run judge, what is unproved until a device run

**What changed.** The A5 Back runs in tests/android-e2e no longer use `input keyevent` or `date` stamps (PR #669, branch develop-qzapp/rust-ui-dev-02/feat/j109-long-message-a5-back-runs-changed, head 923d6e4e on 2026-11-14).

**The sender.** `KeySender` (Java, in androidTest, so it needs no Kotlin runtime) runs as `CLASSPATH=$(pm path org.interweave.human.test) app_process /system/bin ...KeySender ENTER:0 BACK:600 BACK:30`.
- It injects each press asynchronously, through InputManager or InputManagerGlobal by reflection.
- For each press it prints either `sent KEY <uptime ms>` or `unsent KEY <why>`.

**The judge.** The trace is read again with `logcat -v threadtime -v monotonic`. `back_run::judge` then gives each round pass, fail or void by A5.
- A void round fails the case under its own heading (`case_failure`).
- A harness failure is a "the harness:" fail and is never judged.

**Unproved until a device run (2026-11-14).**
- Whether app_process may inject keys on API 30.
- Whether uptimeMillis and logcat's monotonic stamps are one clock. To settle it, have KeySender `Log.i` its own uptime and compare.
- logcat's line shape on the A40: exactly 3 ms digits, and the ` interweave: ` tag.
- Whether `adb shell` passes the sender's exit status through.

**Tooling trap.** Each run of `fabric-review next` writes a new rrN.json, so run it once with output to a file and delete any duplicate. `fabric-review save <pr> <round> < report` must come before `next`.

**State.** DONE: #669 MERGED 178405120 on 2026-11-14 (3 work, 4 fix). Four review rounds were posted, the last clean, and it was armed below the floor on architect-cto-01's word 01a1d9a4 (fabric-pr arm --authorization <id>). The A5 figures wait for a device run, and a phone is assigned by architect-cto. The desktop Escape case is j110.

Related: [[j95-a5-device-case]], [[j107-handoff]], [[phones-order-by-architect-cto]].

*References: j107-handoff, j95-a5-device-case, phones-order-by-architect-cto*

*Anchors: `apps/human-android/android/app/src/androidTest/java/org/interweave/human/test/KeySender.java`, `tests/android-e2e/src/back_run.rs::judge`, `tests/android-e2e/src/screen.rs::read_whole_then_back`*

*Observed 2026-11-14 (rust-ui-dev)*
