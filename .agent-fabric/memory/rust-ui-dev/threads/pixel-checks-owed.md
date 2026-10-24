---
role: "rust-ui-dev"
class: threads
topic: "pixel-checks-owed"
description: "Device checks owed on the Pixel once the owner is back (InterWeave, from 2026-10-21)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3348ec498df5fb9d
---

## Device checks owed on the Pixel once the owner is back (InterWeave, from 2026-10-21)

The Pixel left with the owner on 2026-10-21 (fabric-coordinator broadcast
01a15d23). Owed on it, on a build with each change:
- #313 d9b98c92 (#304 F9 step 5): landscape, keyboard up, the newest
  message's end at the composer (host-verified only); ux-designer-01 judges.
- #313 round-13 risks: a refused send still announced by TalkBack now the
  line starts at zero height; `am start --activity-multiple-task -f
  0x00080000` keeps one MainActivity under singleTask.
- #329 (p2p-network-dev-02) once landed: the 1 s relay_circuit redial is
  gone; logcat shows "reconnect refused" class=RouteHeld with
  held_until_ms, once per hold.
Related: [[android-relay-front-instance]], [[empty-line-takes-no-room]].

Ran 2026-10-21 on main 4c916aa2 (APK 5ec3da27, install -r 16:51 UTC):
forced-new-task launch PASS (one task, one instance, window live);
F9 step 5 PARTLY (status+time in view, text above it: fix owed, j131);
TalkBack refused-send NOT RUNNABLE (no AccessKit bridge on Android main;
carried with the release gate's TalkBack, architect-cto INFO 01a15f4c).
Still owed (j131): notification arrival, network rebind (owner's hand),
human_chat on device. Evidence stills: A scratch path.
Network rebind RAN 2026-10-21 (Wi-Fi off 19:34:55Z, on 19:37:10Z; new net 164 default
19:37:13Z; runtime told the networks 19:37:13-44 in pid 8265; owner read "Online,
limited" throughout). onLost logs nothing, so the drop half rests on the owner's read.
A Wi-Fi toggle moves wireless adb to a new IP:port (was .47:45405, now .48:34159).
Evidence a scratch path, rb0/rb2.png; record it in device-runs/ (j131).

2026-10-22 (j131, #358 merged fbca15e6): recorded in device-runs/2026-10-21-pixel-9a.md
(store reopen, forced-new-task, rebind) and 2026-10-22-samsung-a40.md (app_lifecycle notice +
process death, human_chat/paths 6/6 at API 30). Still owed on the Pixel: ux-designer-01's walk
of F9 step 5, F17, F10, acceptance 23, F4 Details at 200 %/light on afb4466b (installed 03:50 UTC).
API 37 runs of app_lifecycle need a fresh-profile phone (POST_NOTIFICATIONS grant) — not the Pixel.

*References: android-relay-front-instance, empty-line-takes-no-room*

*Observed 2026-10-21 (rust-ui-dev)*
