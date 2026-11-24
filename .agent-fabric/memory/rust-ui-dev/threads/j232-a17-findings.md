---
role: "rust-ui-dev"
class: threads
topic: "j232-a17-findings"
description: "j232 (ux A17 on #578, OBSERVATION 01a1bb5a + correction 01a1bb85): kept-model window re-open, locked page read, lift reads re-opening, Slint Shift limit; IME item open"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6fbb8d1d637fdeb9
---

## j232 (ux A17 on #578, OBSERVATION 01a1bb5a + correction 01a1bb85): kept-model window re-open, locked page read, lift reads re-opening, Slint Shift limit; IME item open

2026-11-08: the j232 branch develop-qzapp/rust-ui-dev-01/feat/j232-j98-a17-on-the-578-build is stacked on j226 (4a2d391ef) and pushed at 2df8d27ec. It has 5 work commits; the PR for j226+j232 is not opened yet. REPLY 01a1bb96 went to ux-designer-01.
- Finding 4 (load error in place of the messages while locked): the Android `model_made` hook makes the surface forget its window reports, so the kept model got `Opened` again and reopened the window from the locked store. Fixed: an `Opened` for the window already open keeps it (7f5e7548b). Edge reads with no service: `UiModel::page_not_read`, routed from app-core on `Failure::ServiceStopped` (eb9ae8f4d, 550fbdc4c).
- Finding 3 (one frame of notice=no-daemon after a grant): `stopped()` sets daemon absent, and the grant clears authentication before the start runs. Now `ViewSide` reads re-opening (daemon_seen None) on a true→false lift while not running, until a non-Opened Store answer (2df8d27ec).
- Finding 1 (Shift dropped): Slint 1.18.1 android-activity `map_key_code` ignores meta state; fixed upstream in 3a7e700487, unreleased. architect-cto ruled (b) (01a1bb8d): wait for the release, README limit (3f2bc455e); bump the pins when a release carries it.
- Finding 2 (IME up, focus on notice after cancel): NOT diagnosed. Hypothesis: the Slint Disable/hide is sent while BiometricPrompt holds window focus, so IMM ignores it and the keyboard is restored on return. Next step: read dumpsys input_method at the cancel on the Pixel once the phone order frees it, then fix.
Related: [[j226-try-again-focus-notice-inset]] [[android-ime-dumpsys]]
- 2026-11-08 17:37Z (ux 01a1bc3c, 3b on main 3e3c6b902): the kept message vanished while locked, which is finding 4 above (fix not on main). Finding 2 (IME) did not reproduce on main, which lacks j226/j232; check it on the j232 build. New 3b findings are j235, stacked on j232.
- 2026-11-10 04:20Z: the Pixel was freed (p2p-network-dev-02 INFO 01a1c3af). Both phones run d1be8eb89, which contains #599, so finding 2 can be measured without an install. fabric-jobs cannot clear the blocker job:rust-ui-dev-01/j229: the query is refused because j229 came from a request by user, and switch, start and block --on clock:<past> all refuse. OBSERVATION 01a1c3b0 went to fabric-coordinator (develop-qzapp/user).
- 2026-11-10 04:40Z, a read-only Pixel probe (a private-network address:38897, SDK 37, app 0.3.0 installed 11-09 12:10): config.yaml has key_unlock_policy background-compatible and availability_mode stay-reachable, and deviceLocked=1. A17 needs user-presence authored by run-as, which is a profile write, so the owner's word is needed ([[pixel-no-destructive]]). j232 is re-blocked on it.

*References: android-ime-dumpsys, j226-try-again-focus-notice-inset, pixel-no-destructive*

*Observed 2026-11-08 (rust-ui-dev)*
