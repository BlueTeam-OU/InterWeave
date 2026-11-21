---
role: "rust-ui-dev"
class: workflow
topic: "j95-a40-ab-rig"
description: "How to run an interleaved A/B of APK builds on the A40 with long_message.rs, a Back-during-opening run with screen recording, and the Pixel with the owner's profile kept"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 519d507b0fb6970a
---

## How to run an interleaved A/B of APK builds on the A40 with long_message.rs, a Back-during-opening run with screen recording, and the Pixel with the owner's profile kept

The scripts are in ~/agent-fabric-tmp/j95-review/. Built in 2026-11-12 for #643.

**Arms.** `build-arms.sh` builds each arm from one source with its
constants patched. They are debug APKs with
`CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=false` and
`CARGO_PROFILE_DEV_OVERFLOW_CHECKS=false`: the native library at the
release opt-level, with the ui-trace still in. Each lands as `ab/<arm>/app-debug.apk`.

**Running them.** `fabric-lease samsung-a40 -- ab-run.sh <out> <rounds> <arm>...`.
- It uses its own adb server on port 15065. Its connect step retries, because the first `adb connect` is sometimes refused while the port is open.
- Between arms it does an `install -r`, checked by sha. It never uninstalls.
- `FILTER=<test>` names one case of the test, and `SHOTS=1` saves its recording.
- Prefix the shots variable with `env`: a `${X:+VAR=..}` in prefix position is not taken as an assignment.
- A run takes about 12 minutes, so five rounds of three arms take about 3 hours. Run it in the background with a timeout over 10,000,000 ms.
- `ab-summary.py <runs>` prints the median and min-max per arm and shape.

**Back during opening.** `Screens::read_whole_then_back` sends ENTER, then `sleep 0.2`, then BACK, all in one adb shell.
- Start `screenrecord` from its `on_row` hook, or the recording misses the keys.
- Record to /data/local/tmp, pull the file, remove it from the phone, and tile its frames with ffmpeg.

**Pixel.** The owner's profile stays.
- rust-ui-dev-01 installs the build with `install -r` and their key; see [[pixel-debug-key-not-mine]].
- `pixel-j95.sh` saves the profile's config.yaml, which the provision step replaces, before the run and restores it after. Check its sha.

**Before a run:** the case starts `target/debug/transport-daemon` and does not build it. After a `target/` clean, run `cargo build -p interweave-transport-daemon` first. If you don't, every round fails in about 4 s, and round 1 also spends about 8 minutes compiling the test, all inside the lease (2026-11-12).

*References: pixel-debug-key-not-mine*

*Anchors: `tests/android-e2e/tests/long_message.rs::on_a_device_back_while_the_reader_opens_closes_it_to_its_conversation`, `tests/android-e2e/src/screen.rs::read_whole_then_back`*

*Observed 2026-11-12 (rust-ui-dev)*
