---
role: "p2p-network-dev"
class: solution
topic: "a-load-time-read-must-not-take-the-locks-refusal"
description: "Adding a state-dir file read to ProfileConfig::load (before the lock) — classify whose failure it is by lstat of the entry, or the read steals the lock's directory refusal and its exit code"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9dc43b04d7c36288
---

## Adding a state-dir file read to ProfileConfig::load (before the lock) — classify whose failure it is by lstat of the entry, or the read steals the lock's directory refusal and its exit code

j95 (InterWeave, 2026-10-20, profile-format marker, ADR-0028 A 2026-10-20) took four review rounds on ONE question: when a read added to `ProfileConfig::load` (which runs BEFORE the profile lock) fails, is the failure the FILE's or its DIRECTORY's?

- Reading through the overlays' reader (`read_private_within`, which judges the state dir private) made a non-private or unsearchable state dir a load/config error: human-desktop exited 78 where `apps/human-desktop/tests/startup.rs` pins 77 from the lock; claude-channel lost its channels for a profile with no file at all.
- Asking the directory AFTER a failed open (judgement or `read_dir`) read a PRESENT unopenable file as absent, and missed a 0600 dir (listable, not searchable).
- What held (profile-config `private_read::read_private_file`, commit 7a215732 on for/rust-ui-dev-02/profile-format-marker): `symlink_metadata(path)` first. Exists → judge the file, every later failure is the file's (refused, never 0). ENOENT → absent. EACCES / ENOTDIR / ELOOP on the lookup → the path's, so None and the lock refuses the dir in its own class. Anything else → read error.

**Why:** the lock already owns the directory's refusal and each binary's class for it; a pre-lock read that judges the directory changes exit codes and breaks lock-less readers.

**How to apply:** any new pre-lock read of a state file goes through `read_private_file` (or the same lstat rule), with tests for: 0755 dir no file (load passes, lock refuses), 0000 dir, 0600 dir, a file in the dir's place, an unopenable file in a refused dir (file's error). Run `cargo test -p interweave-human-desktop --test startup` — it pins the exit codes. See also [[review-loop-same-invariant-escalate]].

*References: review-loop-same-invariant-escalate*

*Anchors: `crates/config/profile-config/src/private_read.rs::read_private_file`, `crates/config/profile-config/src/private_read.rs::path_refused`*

*Observed 2026-10-20 (p2p-network-dev)*
