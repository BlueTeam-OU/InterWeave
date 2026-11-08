---
role: "rust-ui-dev"
class: threads
topic: "j90-a40-open-stall"
description: "j90 - A40 open stall was the debug library (fix 90dfbdb7c, opt-level 3); A40 run DONE; FOLDED INTO #565 (owner 2026-11-05) at 623c26939, lands with j95"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2760347eefb8c3df
---

## j90 - A40 open stall was the debug library (fix 90dfbdb7c, opt-level 3); A40 run DONE; FOLDED INTO #565 (owner 2026-11-05) at 623c26939, lands with j95

j90 (2026-11-04): on the A40, the conversation open that stalled `android_main` 7.7-12.3 s was a body of the chat
case's compressible text, ~900 Markdown list lines (`chat_text`). It is not the held-row count. With the debug APK's
native library built `--release`, the same open took 1.41 s Enter-to-handled, against a 1.22 s one-row control (the
`input` tool takes ~1 s on that phone). Recorded in `apps/human-android/device-runs/2026-11-04-samsung-a40.md`
"The pane on an optimised build"; it landed on main with #563.

Fix (2026-11-05): commit 90dfbdb7c on develop-qzapp/rust-ui-dev-02/feat/j90-android-opening-a-conversation-stalls-androi
(pushed, no PR, 1 work commit). `cargoNdkDebug` sets `CARGO_PROFILE_DEV_OPT_LEVEL=3`, which keeps debug assertions,
overflow checks and the debug features. The APK built in 8m55s, and its library is 52 MB uncompressed.

Owed: an A40 run of that APK, opening two unread conversations from the chat cases, with the result added to a device
record. Then the batch needs a PR (it is under the floor) and a note to devex-tooling about the CI `android apk` build
time. Longer bodies are bounded by j92's cut.

On 2026-11-05 the A40 at a private-network address:41460 refused connections, and mdns found nothing. I let p2p-network-dev-02 take
it first for their j17 re-run (REPLY 01a1aa8f) and asked them for its new address. The wrapper is the scratch
`a40-run.sh`: its own nodaemon adb server on port 15065, under `fabric-lease samsung-a40`. See [[a40-device-run-traps]].

The A40 order, agreed 2026-11-05: p2p-network-dev-02's j17 re-run goes first, and they send me the port. Then my j90
run. Then p2p-network-dev-01's j141, which uninstalls the app and leaves release v0.2.0 on the phone. When I release
the lease I send p2p-network-dev-01 an INFO (my REPLY 01a1aaa1) saying what the phone carries.

Order changed 2026-11-05 14:05: I told p2p-network-dev-01 (REPLY 01a1ac08) that their j141 goes BEFORE my j90 and may
wipe the A40, since my run reinstalls its own debug APK. They send me an INFO with the phone's state and port when
they release it. A fresh release install there is also a j96 check once #569 lands.

BEFORE THE j90 A40 RUN (2026-11-05 18:00Z): the A40 carries my test-signed RELEASE build of #569 (signer 8f75414f),
holding the OWNER's new identity, created by hand for j96. A debug APK cannot install over it, so it must be uninstalled
first, and that wipes the owner's identity. Ask the owner first, and check whether p2p-network-dev-01 has used the phone
since (INFO 01a1ac7a told them what it carries).

Owner's word 2026-11-05 ~18:10Z (asked in session): "Yes, wipe it" - uninstall the #569 release build and the owner's
j96 identity on the A40 for the j90 run. The A40 adb port was a private-network address:39991 still; the first `adb connect` over the
~400 ms link failed, so connect in a retry loop. Wrapper: scratch copy of `.agent-fabric/bin/phone-run` with lease
samsung-a40 and port 15065.

A40 RUN DONE 2026-11-05 18:26-18:35Z: debug of 05e7ab70f (app 66af1d18, signer 26d1eb18), chat cases passed, opens
1.49 and 1.68 s against a 1.00-1.06 s TAB control, no ANR. Recorded in 2026-11-05-samsung-a40.md (commit 7a4e7b58f,
pushed). INFO 01a1acff to p2p-network-dev-01: phone free, what it carries. Owed next: fold origin/main, open the PR
(2 work commits, below the floor: arming word is architect-cto's or the owner's), the review, and the devex-tooling note
on the CI `android apk` build time (gradle debug build took 16m36s here, its cargo step 6m54s).

2026-11-05 ~19:00Z, owner's choice in session: j90 rides on #565 (one open PR per agent), not a PR of its own. Folded from
the j90 working copy (the job hook refuses commits in another job's copy): detached at #565's head cb101cdd3, merged
origin/main, merged the j90 branch -> 623c26939, pushed as a fast-forward to the j95 branch. #565 now 14 work, 2 fix,
5 merge; title and body carry a j90 section. The j95 worktree had the merged j96 branch checked out; I switched it to the
j95 branch at cb101cdd3, so it must `git pull --ff-only` before j95 resumes. Full workspace suite run on 623c26939 in
the j90 copy (detached). Still owed on #565 (j95's): the review covering e4587d155..623c26939, F1/F2 answers, arming.
The devex-tooling note on the debug APK build time is still j90's to send.

*References: a40-device-run-traps*

*Observed 2026-11-05 (rust-ui-dev)*
