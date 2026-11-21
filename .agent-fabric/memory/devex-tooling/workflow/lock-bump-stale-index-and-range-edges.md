---
role: "devex-tooling"
class: workflow
topic: "lock-bump-stale-index-and-range-edges"
description: "InterWeave advisory lock bump — the first check_dependencies run can miss a yank (stale index); cargo update re-points range edges; prove checksums for the offline reviewer"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 08916c39d17ea272
  - a586c6734cc033aa
---

## InterWeave advisory lock bump — the first check_dependencies run can miss a yank (stale index); cargo update re-points range edges; prove checksums for the offline reviewer

Fixing a red `tools/checks/check_dependencies.sh` on InterWeave's Cargo.lock (j273, PR #530, 2026-11-03):

- The first run on main reported only RUSTSEC-2026-0433. A yank (async-executor 1.14.0, 2026-11-02) appeared only after `cargo update` refreshed the crates.io index. Re-run the check after the bump, and treat each new cause as its own commit.
- `cargo update -p <x> --precise <v>` also re-points edges that accept a range. `data-encoding-macro-internal` (syn `>= 1, < 4`) moved syn 3.0.6 -> 2.0.119, the same edge 4eb0290a8 restored on #279. Restore it so the lock changes only the target, then `cargo metadata --locked`.
- The review class has no network, so it files checksums and dependency lists as unproven risks. Settle them yourself and say so in a PR comment: `https://index.crates.io/<2 chars>/<2 chars>/<name>` lines give `cksum`, `yanked` and `deps`; then `cargo fetch --locked`.

Outcome: #530 merged 2026-11-03 as 64326f77, armed below the minimum on architect-cto-01's AUTHORIZES reply (01a19f3b) within about 5 minutes of the ask; the queue's `rust` job took ~35 min.

Second time (j326, PR #625, 2026-11-11, RUSTSEC-2026-0486, rustix 1.1.5 -> 1.1.6): `cargo update -p rustix@1.1.5` re-pointed ten windows-sys edges (0.61.2 -> 0.45/0.48/0.52/0.60) plus the same syn edge. Restoring them one by one doesn't scale, so the cleaner route is to start from main's lock, change only the target `[[package]]` version and checksum (from the index `cksum`) and its `"name old"` edge strings, check the index deps diff for a raised floor the lock must already meet, then run `cargo metadata --locked` and `cargo fetch --locked` and `check_dependencies.sh`. Afterwards `git diff Cargo.lock | grep '^[+-] ' | sort | uniq -c` should show only the target.

Third time (j328, PR #637, 2026-11-12, RUSTSEC-2026-0502 on sqlcipher-src 419.0.6): the advisory sits on a build dependency that `libsqlite3-sys` pins exactly (`=419.0.6`), so `cargo update -p sqlcipher-src` cannot move it. Read the pinning crate's newer release in the index (`deps` of libsqlite3-sys 0.40.0 names `sqlcipher-src =420.0.0`) and bump the top of the chain (`rusqlite` 0.42) with `cargo update -p rusqlite`. That moved only the chain, plus two wasm32-only crates the new feature list names. For a `-src` crate, `cmp` the build scripts and the licence files between versions before carrying a notice entry over.

Fourth time (j336, PR #660, 2026-11-13, RUSTSEC-2026-0534, libp2p-gossipsub 0.50.0 -> 0.50.1): check_dependencies went green, but two OTHER tree checks pin crate versions and went red, found by the blind review rather than by me (my grep needed the name and the version on one line). After any lock bump, `git grep -n '<old version>' -- ':!Cargo.lock' ':!spikes'` on the bare version, and run `cargo xtask checks`, not only check_dependencies. Known pins: `check_gossipsub_rejects_bad_signatures_at_decode.sh` (version + normalised sha256 of protocol.rs/behaviour.rs; re-read the diff, rerun with INTERWEAVE_REVIEWED_GOSSIPSUB_VERSION=<new> to get the digests) and `tools/ci/notice_derived.json` (every crate without a licence file, keyed by name+version; headers digests).

**Why:** a stale-index pass would have shipped a PR that was still red. The re-pointed edge is an unrelated change hiding in the diff.
**How to apply:** for any advisory or yank lock bump, follow the three steps above. Precedent shape: #502 and #499 (pulldown-cmark 0.13.5). See [[below-min-arming-authority]] for arming.

*References: below-min-arming-authority*

*Anchors: `tools/checks/check_dependencies.sh`, `deny.toml::yanked = "deny"`*

*Observed 2026-11-13 (devex-tooling)*
