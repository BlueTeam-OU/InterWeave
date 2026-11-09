---
role: "p2p-network-dev"
class: workflow
topic: "release-signing-is-the-owners-hand"
description: "How an InterWeave release is cut (ADR-0066): the owner signs the tag and approves the release environment; the agent prepares, gives the owner a guarded script, and verifies with one traced script"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1dcccfd2304c42fc
---

## How an InterWeave release is cut (ADR-0066): the owner signs the tag and approves the release environment; the agent prepares, gives the owner a guarded script, and verifies with one traced script

Learned cutting v0.2.0 (2026-10-27/28, j134; #487, #499 on BlueTeam-OU/InterWeave).
- The tag is signed by the owner's tag key in the owner's account, never an agent's (ADR-0066 decision 1). Refused when asked to "do it yourself": the key is absent, and an agent-signed tag fails the pin.
- A `!` command at the prompt runs as the AGENT's account. Give the owner a script for their own terminal, guarded against agent logins (`case $(id -un) in *-dev-0*) exit 1`), and keep its inputs out of shared /tmp. An agent-owned file there can be read in place of the owner's.
- The owner's git signs through `gpg.program=/usr/local/bin/ots-git-gpg-wrapper.sh` (OpenTimestamps), which hangs. Sign with `git -c gpg.program=gpg -c gpg.format=openpgp tag -s`. A pinentry also needs `GPG_TTY=$(tty)`.
- The owner has no Wayland on this host, so wl-copy fails. Set environment secrets with `gh secret set NAME -e release -R <repo> < file`.
- The release run pauses at `release apk signing` for the `release` environment's required reviewer (the owner's account). Ask; never self-approve, even though can_approve=true.
- Before tagging, bump Android versionCode (apps/human-android/android/app/build.gradle.kts, rust-ui-dev's lane) above the last published APK's. #487's review caught the reuse of 1.
- Verify with ONE script run from the top under `set -x`, `PS4='$ '`, and paste its trace as the record. A hand-assembled transcript drew four review rounds.
- Cite only the tag's harness for a run made at the tag. Code added after the tag guards later runs only.

*Observed 2026-10-28 (p2p-network-dev)*
