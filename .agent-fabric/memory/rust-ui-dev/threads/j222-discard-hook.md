---
role: "rust-ui-dev"
class: threads
topic: "j222-discard-hook"
description: "j222 debug-only adb store-open refusal (A16 discard): pushed bbdfad4a8 on feat/j222 over j220; INFO 01a1b5d6 to ux; no device run"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 709a3355cf7a2dda
---

## j222 debug-only adb store-open refusal (A16 discard): pushed bbdfad4a8 on feat/j222 over j220; INFO 01a1b5d6 to ux; no device run

2026-11-07: j222 (ux REQUEST 01a1b5aa item 3) is pushed. It is commits f8278f236 + bbdfad4a8 on develop-qzapp/rust-ui-dev-01/feat/j222-a16-discard-debug-only-store-open, on top of j220 0e1d4d175 and j218 c01eb6a70. That makes 4 work commits for one PR, with no PR yet.
- The hook is `am broadcast -a org.interweave.human.debug.REFUSE_STORE -p org.interweave.human`. It goes through DebugHooks.kt (debug) and the JNI entry `Java_org_interweave_human_DebugHooks_refuseNextStoreOpen`, which only exists with dev-stand-ins, to `ServiceHost::refuse_next_store_open`.
- The refusal is one-shot: an AtomicBool swap in `ServiceHost::open_store`. Both starts now use that helper. The files are left untouched, so the plan and the discard are real.
- The flag is per host, not a global static, so parallel tests do not share it. Test: tests/service.rs `the_store_hook_refuses_one_start_as_recovery_and_leaves_the_store`, mutation-checked.
- Featureless clippy needed `cfg_attr(not(feature), expect(unused_self))` ([[feature-gated-body-clippy]]).
- `verifyReleaseNative` forbids "Java_org_interweave_human_DebugHooks". Checked: the release .so has 0 matches and the debug .so has the entry.
- APK: A host scratch path, with a copy in a scratch path (sha 9d3a60aa…e60). It is the same key as 0e1d4d175, so install -r works.
- The Pixel runs 0e1d4d175 (j221). I offered to install j222 only if ux asks.

Related: [[j220-restart-hook]] [[j221-pixel-install-hook-build]] [[j218-one-unlock-prompt]]
- ux REPLY 01a1b5f9: ux-designer-01 installed bbdfad4a8 on the Pixel themselves (install -r, device sha matches). j98 reviews that build and names both hooks. The Pixel is under their lease, so do not install there.

*References: feature-gated-body-clippy, j218-one-unlock-prompt, j220-restart-hook, j221-pixel-install-hook-build*

*Observed 2026-11-07 (rust-ui-dev)*
