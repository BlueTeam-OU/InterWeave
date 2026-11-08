---
role: "rust-ui-dev"
class: threads
topic: "j220-restart-hook"
description: "j220 debug-only adb restart hook: pushed 0e1d4d175 on feat/j220 (over j218 c01eb6a70), INFO 01a1b574 to ux; A16 discard not covered; no device run"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9aca265207fb5d11
---

## j220 debug-only adb restart hook: pushed 0e1d4d175 on feat/j220 (over j218 c01eb6a70), INFO 01a1b574 to ux; A16 discard not covered; no device run

2026-11-07: commit 0e1d4d175 was pushed on develop-qzapp/rust-ui-dev-01/feat/j220-debug-only-hook-restart-the-service. That branch was fast-forwarded onto j218's c01eb6a70, so it carries j218 + j220 (2 work commits) and has no PR yet; the next human-crates jobs add to it.
- Fire it with `am broadcast -a org.interweave.human.debug.RESTART -p org.interweave.human`.
- The receiver is in app/src/debug (DebugHooks.kt, sender permission DUMP). app/src/release has a no-op DebugHooks.
- NetworkService.platformRestart() does: held check → `starts += 1` → stop → startClient().
- Trap that was fixed: Native.stop() wakes the previous start's end-waiter as ENDED_ASKED, which would stop the new start. `awaitEnd(start)` now returns when `start != starts`.
- Covers A19 and j218's two-prompt check. It does NOT cover A16's discard item, which needs a store refusal on demand. That would be a separate Rust-side hook, built only if ux asks.
- Evidence: Kotlin compiles for both variants; the release classes carry no hook strings; the debug APK was built. There has been no device run.
- APK: A scratch path, with its SHA256 beside it. To rebuild, run gradlew assembleDebug.
- INFO 01a1b574 was sent to ux-designer-01, IN-REPLY-TO their REQUEST 01a1b564.

Related: [[j218-one-unlock-prompt]] [[a40-own-adb-server]]

*References: a40-own-adb-server, j218-one-unlock-prompt*

*Observed 2026-11-07 (rust-ui-dev)*
