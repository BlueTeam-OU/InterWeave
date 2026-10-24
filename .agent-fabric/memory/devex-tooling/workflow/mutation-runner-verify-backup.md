---
role: "devex-tooling"
class: workflow
topic: "mutation-runner-verify-backup"
description: a mutation script that backs up a guard with cp and restores it can empty the guard when /home fills; verify the backup first
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - dbcd5034efb268ac
---

## a mutation script that backs up a guard with cp and restores it can empty the guard when /home fills; verify the backup first

On 2026-10-20 the shared /home on develop-qzapp hit 100% (another account, transient: 1.4 MB free, 89 GB free minutes later). My mutation runner's `cp guard guard.orig` failed silently, its restore then copied the empty backup over the guard: tools/checks/check_android_no_play_services.sh became 0 bytes and an uncommitted edit was lost (recovered from git, redone).

**Why:** cp into a full disk leaves a short or empty file; a trap that restores from it unconditionally destroys the original.
**How to apply:** back up into the scratchpad, then `cmp -s` and `[ -s ]` the copy and abort before mutating if either fails; restore only when the file differs from the verified copy; keep a second copy of uncommitted edits before a mutation run. Related: [[review-class-cannot-write-scratch]] (why mutations are run by me, not the reviewer).

*References: review-class-cannot-write-scratch*

*Anchors: `tools/checks/test_check_android_no_play_services.sh::expect_fail`*

*Observed 2026-10-20 (devex-tooling)*
