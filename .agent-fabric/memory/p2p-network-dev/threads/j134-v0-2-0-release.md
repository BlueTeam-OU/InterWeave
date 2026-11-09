---
role: "p2p-network-dev"
class: threads
topic: "j134-v0-2-0-release"
description: "j134 (3 of 3) — the v0.2.0 release: #487 key pins, owner-signed tag, release run verified, acceptance on the archive (#499), RUSTSEC-2026-0414 lock fix; j134 delivered 2026-10-28"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b62d34268f97c7ad
---

## j134 (3 of 3) — the v0.2.0 release: #487 key pins, owner-signed tag, release run verified, acceptance on the archive (#499), RUSTSEC-2026-0414 lock fix; j134 delivered 2026-10-28

Part 3 of j134 (REQUEST 01a17c02, architect-cto); parts 1-2 are j134-control-plane-over-interweave and j134-two-host-and-case-1-built.

- Release prep PR #487 (feat/j134-release-v0-2-0), opened 2026-10-27: version 0.2.0 + INTERWEAVE_PACKAGE_BIN seam in tests/support (daemon, transportctl, netns mount) + control_plane CLI. release-keys.toml is EMPTY on main: every tag fails closed until the owner pins the tag key (gpg) and the archive key (minisign, secret in the release environment). Steps: packaging/android/README.md "The release keys". Next: the owner hands over the public halves, I commit them to #487, review, the owner arms, the owner signs v0.2.0 on the merge commit, the release run, then I verify the minisign sig + hash and run control_plane/two_hosts with INTERWEAVE_PACKAGE_BIN=<extracted>/bin, recorded under packaging/linux/evidence.
- Tag key PINNED on #487 (739594ca): fingerprint 220728A8B44731A93CFF8969FA651DF333C31A0C, Ed25519, no expiry. Its uid is the placeholder "<you@example.org>"; the owner was told quick-add-uid. Still owed by the owner: the minisign .pub second line (RW...) + the release env secrets. Pitfall: `public_key = ""` also matches `minisign_public_key`; anchor the match to the line.
- Archive key PINNED (b1b65cb9) RWThBWgf...iJs4; both MINISIGN secrets set by the owner 16:43/16:44Z. #487 is a security boundary (Cargo.toml, release-keys.toml); blind review dispatched on 3b6db4e7..b1b65cb9. The owner has no Wayland on this host: use `gh secret set -e release` instead of wl-copy.
- #487 at f97e5e07: folded versionCode 2 (rust-ui-dev-01 b1b65cb9..1287c77b) + the human-cli README (rust-ui-dev-02 ..8af94348). Round 2 found no P1/P2. CARRY: F2 the tag uid placeholder is the owner s; F5 the release check must set INTERWEAVE_PACKAGE_BIN for tests/automated-client alone, so state the exact command in the v0.2.0 verification record and make workspace_binary refuse while it is set in the next seam change. Lesson: `cargo test --lib` with several -p limits ALL packages to lib targets.
- #487 MERGED 32ade966 2026-10-27 (main tip, version 0.2.0, both pins). Notes posted as #487 last comment (issuecomment-6449793963). The OWNER signs the tag (ADR-0066 d1; an agent never holds the tag key; refused when the owner asked me to). Next: watch release.yml for v0.2.0, download, minisign -V + sha256, run INTERWEAVE_PACKAGE_BIN=<abs extracted bin> on automated-client (control_plane, two_hosts) alone, record in packaging/linux/evidence, then deliver j134 + tell architect-cto.
- v0.2.0 TAG PUSHED by the owner 2026-10-27 19:21Z (annotated, on 32ade966, signed by 220728A8...). Release run 40568434157. Lesson: the owner's git has gpg.program=/usr/local/bin/ots-git-gpg-wrapper.sh (OpenTimestamps), which hung the signing; sign with `git -c gpg.program=gpg -c gpg.format=openpgp tag -s`. A `!` command at the prompt runs as the agent account, so give the owner a script for their own terminal and guard it against agent accounts.
- v0.2.0 PUBLISHED 2026-10-28 07:32Z (run 40568434157; the owner approved the release environment, its reviewer the CEO's account; never self-approve). Verified: minisign -V on the archive and its .sha256 with the pin (a tamper control fails), sha256 -c OK, the tag verifies against the pinned key in a fresh keyring, all programs report 0.2.0+32ade966cff8. Acceptance running at the tag, the j134 worktree detached at v0.2.0, INTERWEAVE_PACKAGE_BIN=<scratch>/rel020/.../bin, automated-client control_plane + two_hosts. Then the evidence record, a PR, deliver j134.
- ACCEPTANCE MET 2026-10-28: v0.2.0 archive passes control_plane 1 + two_hosts 3 with INTERWEAVE_PACKAGE_BIN. Record + F5 (workspace_binary refuses while set; claude_channel 15 cases stop on it) on #499 (feat/j134-v0-2-0-release-check), review dispatched; architect-cto told (REPLY 01a181c3). The release env was approved under the CEO's account (the owner account). j134 delivers when #499 lands. user_units bin_dir only runs in #[ignore] manager cases.
- #499 round 3 (aa5121a8): the record is now one script plus its set -x trace (PS4="$ "), run once; the home path written ~. Lesson: an evidence record assembled from separate runs keeps drawing "states more than it shows" rounds; write the check as one script and paste its trace.
- 2026-10-28 RUSTSEC-2026-0414 (pulldown-cmark 0.13.4, via i-slint-common 1.18.1 + chat-protocol markdown) turned every PR tree-checks red; fixed on #499 by 281a8707 (lock only). `cargo update -p X --precise` also re-picked an unrelated edge (data-encoding-macro-internal syn 3->2): restore it and confirm with `cargo metadata --locked` (not --offline after a registry cache wipe). check_dependencies needs PATH=$HOME/.cargo/bin for cargo-deny.
- j134 DELIVERED 2026-10-28: #475, #478, #487, #499 (04dcace5) all merged; v0.2.0 published; architect-cto told (REPLY 01a18219). Carried: the tag key uid placeholder (owner), the APK untested, no second machine, no empty-account install.

*Observed 2026-11-03 (p2p-network-dev)*
