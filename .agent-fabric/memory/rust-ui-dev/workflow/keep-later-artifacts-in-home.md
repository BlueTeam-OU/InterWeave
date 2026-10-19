---
role: "rust-ui-dev"
class: workflow
topic: "keep-later-artifacts-in-home"
description: "an artifact a later run or another session needs (an APK waiting for the phone, a patched crate copy) goes under ~/scratch/<job> with its sha and rebuild step, never /var/tmp; removed when its job closes"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b6763f1b89531bf4
---

## an artifact a later run or another session needs (an APK waiting for the phone, a patched crate copy) goes under ~/scratch/<job> with its sha and rebuild step, never /var/tmp; removed when its job closes

On develop-qzapp, /var/tmp (each login's TMPDIR and the session scratchpad) sits on the root
volume, about 21 GB for the whole fleet. It was wiped on 2026-10-17 at 18:06, after the disks
filled and the fleet went down twice. That took the j97 prototype APKs and the patched backend
copy, and main's APK waiting for the phone. Each had to be rebuilt (about 10 minutes) once the
phone was finally back. The owner's instruction of the same day: point build scratch at your
home, not /var/tmp.

- The session scratchpad is for this session's throwaway files only.
- Anything a later run waits for goes under `~/scratch/<job>/`, with a note beside it: the
  source commit, the sha256, and the command that rebuilds it. A device step waits on a phone
  that can be away for a day.
- Remove it when the job closes, and say so in the job's notes. Home is shared too: this login
  was the largest home on the host (135 GB, 2026-10-17), mostly target/ directories of merged
  worktrees.

**How to apply:** when a job blocks on a device or on another agent with a built artifact in
hand, move the artifact home before you switch away. Related: [[release-disk-after-tests]]
[[release-phone-after-testing]]

*References: release-disk-after-tests, release-phone-after-testing*

*Anchors: `apps/human-android/README.md::assembleDebug`*

*Observed 2026-10-18 (rust-ui-dev)*
