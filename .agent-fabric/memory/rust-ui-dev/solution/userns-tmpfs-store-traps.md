---
role: "rust-ui-dev"
class: solution
topic: "userns-tmpfs-store-traps"
description: "a human-store test in an unprivileged user+mount namespace needs its own root (ancestor rule) and remount needs --options-mode ignore"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0539cdc0feabd184
---

## a human-store test in an unprivileged user+mount namespace needs its own root (ancestor rule) and remount needs --options-mode ignore

Measured 2026-11-16 building crates/human/store/tests/full_filesystem.rs (PR #687, j113):
- Under `unshare -rm` the host's `/` and `/home` read as the overflow uid 65534, so
  HumanStore::open refuses with DirectoryNotPrivate (ADR-0028 ancestor rule). Fix: pivot_root
  into a tmpfs root the namespace owns, rbind usr/etc/dev/proc/sys/lib* and the test binary's
  directory, as tests/support/src/e2e/netns.rs does.
- `sh -c SCRIPT a b` puts `a` in $0: pass a dummy "sh" before the real arguments.
- `mount -o remount,size=4m` on a tmpfs in the namespace fails "tmpfs: Invalid uid '<host uid>'"
  (mount(8) repeats uid= from mountinfo); `mount --options-mode ignore -o remount,size=4m` works.
- A store filled to ENOSPC on a 1 MiB tmpfs had 750 KB WAL + 266 KB db; freeing 256 KiB was not
  enough for recheck_health's probe — grow the mount instead.
- CI's rust job lifts the AppArmor userns restriction before Tests (ci.yml "Allow unprivileged
  user namespaces"); its comment lists every namespace test (devex-tooling's file).

*Anchors: `crates/human/store/tests/full_filesystem.rs::NEW_ROOT`, `tests/support/src/e2e/netns.rs::NEW_ROOT`*

*Observed 2026-11-16 (rust-ui-dev)*
