---
role: "devex-tooling"
class: solution
topic: "apk-class-check-and-bound-mutations"
description: "release APK's Java classes are checked by tools/ci/check_apk_classes.py (android apk job + release.yml); a bad-index test must read a VALID descriptor or the bound check is untested"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4ed41e9010617e5d
---

## release APK's Java classes are checked by tools/ci/check_apk_classes.py (android apk job + release.yml); a bad-index test must read a VALID descriptor or the bound check is untested

DECISION 01a12770's JNI half landed as PR 586 (j80, 2026-11-07):
`tools/ci/check_apk_classes.py` reads class_defs of an APK's root
classes*.dex; CI's `android apk (assembleDebug)` job now also runs
`assembleRelease assembleDebugAndroidTest` (519 s with release builds
cold, job 15m14s, under rust's 19m44s) and holds the release APK to
`--holds-none org.interweave.human.test` with MainActivity and the test
APK's E2eCase as live controls; release.yml's build job runs the same
check. assembleRelease there also brings `verifyReleaseNative` to every PR.

Trap measured on it: a self-test case that points an index past a table
usually lands on garbage that a LATER check (the `L…;` descriptor test)
rejects, so deleting the bound check stays green. Declare the table one
entry shorter than written and point at that last real entry: only the
bound check can refuse it.

**How to apply:** for any parser guard, mutate each bounds check alone;
see [[mutation-runner-verify-backup]] and [[scripted-edit-assert-match]].

*References: mutation-runner-verify-backup, scripted-edit-assert-match*

*Anchors: `tools/ci/check_apk_classes.py::JAVA_NAME`, `.github/workflows/ci.yml::check_apk_classes.py`, `.github/workflows/release.yml::check_apk_classes.py`*

*Observed 2026-11-07 (devex-tooling)*
