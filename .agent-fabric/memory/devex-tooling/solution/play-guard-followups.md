---
role: "devex-tooling"
class: solution
topic: "play-guard-followups"
description: "check_android_no_play_services.sh after #311: fixtures exempt, STRICT required, lock mode an allow-list; what a lockMode edit must keep"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2bcbc0814113f5c3
---

## check_android_no_play_services.sh after #311: fixtures exempt, STRICT required, lock mode an allow-list; what a lockMode edit must keep

The three follow-ups owed after #307 landed on BlueTeam-OU/InterWeave#311 (job j170, 2026-10-20):
- test fixtures are test tooling in both halves (architect-cto DECISION 01a15473; 5fd08f8d, help reworded 422a8d23);
- STRICT required in the root's buildscript and allprojects dependencyLocking (d3027aea);
- the lock mode is an ALLOW-LIST, not a deny-list of DEFAULT spellings (f6d75402, 36cd5c0e): any line naming `(^|[^A-Za-z]|get|set)[Ll]ockMode` fails except an exact `lockMode.set(LockMode.STRICT)` line of the root build.gradle.kts. A deny-list was beaten by `LockMode.valueOf(property)`, an argument on the next line and `getLockMode()` (review of #311, judged P2).

**Why:** a subproject's dependencyLocking overrides the root's, so every spelling of a mode is a way round STRICT; the left boundary keeps `setBlockModes` (KeystoreCipher.kt shape) out.
**How to apply:** a new LOCK_MODE alternative needs a test case only it catches (the review found two alternatives with none); same CI change put lintDebug/testDebugUnitTest in the android apk job, see [[ci-command-line-is-read-by-guards]].

*References: ci-command-line-is-read-by-guards*

*Anchors: `tools/checks/check_android_no_play_services.sh::LOCK_MODE`, `tools/checks/check_android_no_play_services.sh::STRICT_LINE`*

*Observed 2026-10-20 (devex-tooling)*
