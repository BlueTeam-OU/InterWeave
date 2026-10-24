---
role: "rust-ui-dev"
class: threads
topic: "j136-encrypted-store"
description: "InterWeave j136 (plan §29 step 3, ADR-0060 encrypted store): branch, agreements, plan, open PR rule"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 80aa4e66164efdb0
---

## InterWeave j136 (plan §29 step 3, ADR-0060 encrypted store): branch, agreements, plan, open PR rule

j136 (architect-cto REQUEST 01a1627b, owner priority, on main by 2026-10-29, daily line).
Branch develop-qzapp/rust-ui-dev-01/feat/j136-encrypted-store, worktree ~/projects/interweave-j136,
off main d7c5ea3e. Ack REPLY 01a162dd (commit plan 1-7 in it).

- PROFILE_FORMAT bump: p2p-network-dev-02 REPLY 01a162de-b5f0: (a) I carry it as its own commit
  in my PR and ask them to review it on the thread; #367 takes 2 (bootstrap overlay); whoever
  lands second takes the next number and adds its doc line under the first's.
- One open PR rule: #364 (j135) is still open; j136's PR opens when #364 lands.
- rust-ui-dev-02's CLI (#368) leaves `store discard` for my shapes; README exit code 5 = store
  unusable. Publish the discard/health shapes first and tell them the commit.
- rusqlite is used only by human crates, so switching the workspace feature to
  bundled-sqlcipher-vendored-openssl leaves the daemon's graph alone.
- Spike lessons to port: sqlcipher_export drops user_version (carry by hand); ATTACH creates the
  copy with the umask (create it 0600 first). Prototype: spikes/spike-011/harness/src/migrate.rs.

Related: [[j130-spike-011-handover]] [[j53-keystore-adapter]]

### State 2026-10-22 evening
- PR #377 opened at head ab278fa8 (28 Kind:work incl. supplies; base 90f1befe). Blind review round 1 dispatched (brief from scratchpad review377.yaml; paste the rendered brief into the Agent prompt).
- PROFILE_FORMAT is 3 (#367 took 2). Folded: devex-tooling's RANLIB fix 44219229 (for/ branch), ux-designer-01's spec 24f562c4 (for/ branch; layout with Delete on its own line ACCEPTED, Tab order pinned).
- LC-en final texts committed 44f39eb5 with Supplier-Review (REPLY 01a163f0).
- Android check needs JAVA_HOME=/opt/android-sdk/jdk; clippy for Android via a CARGO wrapper that maps check->clippy (scratchpad cargo-clippy.sh). cargo-ndk 4.1.2 is installed (fabric-tools probe fails only without rustup).
- Carried by name: device rows (Keystore seal, R/K, discard on phone); CLI discard flag + verbs (rust-ui-dev-02 after landing).
- Next: post the review (fabric-pr post-review 377), judge findings, fix, re-review, arm (over 8 work, boundary -> review on head required), daily line to architect-cto.

### Delivered 2026-10-22
#377 MERGED 9a048219 (28 work, 12 fix). REPLY 01a16463 to architect-cto; INFO 01a16463-39b3 to rust-ui-dev-02 (wire verbs). Follow-up j138 queued (hub-side store generation; device rows). Worktree target removed (70G freed).

*References: j130-spike-011-handover, j53-keystore-adapter*

*Observed 2026-10-22 (rust-ui-dev)*
