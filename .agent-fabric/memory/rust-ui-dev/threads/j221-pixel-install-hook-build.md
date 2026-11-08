---
role: "rust-ui-dev"
class: threads
topic: "j221-pixel-install-hook-build"
description: "j221: install debug 0e1d4d175 on the Pixel over release v0.2.0 for ux j98 — blocked on re-pair + the session guard refusing the uninstall; guarded script ready"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 580a71822d1cb20a
---

## j221: install debug 0e1d4d175 on the Pixel over release v0.2.0 for ux j98 — blocked on re-pair + the session guard refusing the uninstall; guarded script ready

2026-11-07: ux-designer-01 sent REQUEST 01a1b5aa. It relays the owner's yes to uninstall release v0.2.0 from the Pixel 9a (no identity on it) and to install the debug APK of 0e1d4d175 (j220 over j218). j98 runs once, on that build.
- The Pixel was at a private-network address:38897 (the owner's word). The LAN is back on 100.x.
- My adb connect fails there, through phone-run, while ping and TCP answer. That is a lost pairing: the owner must re-pair rust-ui-dev-01's key.
- The auto-mode guard refused an uninstall script ("Irreversible Deletion"). The owner's word reached me only by relay, so the owner has to allow it in my session. I did not route around it.
- The guarded order, once both are cleared:
  1. Check that the installed app is versionName 0.2.0, not DEBUGGABLE, and that there is no .test package; otherwise STOP.
  2. `adb uninstall`, then `adb install` of a scratch path (sha256 7d6ca799…bcd).
  3. `sha256sum` of `pm path` on the device. Launch nothing.
  4. REPLY to ux with the commit, the device sha, and "lease free" after a pgrep check.
- REPLY 01a1b5ac told ux all of this. It also told them the policy value is `user-presence` (kebab-case), not `user_presence`. Item 3 of the REQUEST (A16 discard hook) is queued as j222.

Related: [[j220-restart-hook]] [[pixel-no-destructive]] [[phones-lan-2026-10-30]] [[release-phone-after-testing]]

2026-11-07 (later session): the owner said in my session "yes, uninstall v0.2.0" — that half is cleared. The re-pair is still owed: connect at .62:38897 still fails (TCP open). The guarded order is scripted as a scratch path (run through phone-run once paired).
DONE 2026-11-07: the owner re-paired (pair port 46569; connect stays 38897). The guarded script found v0.2.0 non-debuggable with no test package, uninstalled it, and installed 0e1d4d175. The device sha is 7d6ca799…bcd. Launched nothing; the lease is free. Sent REPLY 01a1b5c6 to ux. The next debug build goes on with install -r over it (same key).

*References: j220-restart-hook, phones-lan-2026-10-30, pixel-no-destructive, release-phone-after-testing*

*Observed 2026-11-07 (rust-ui-dev)*
