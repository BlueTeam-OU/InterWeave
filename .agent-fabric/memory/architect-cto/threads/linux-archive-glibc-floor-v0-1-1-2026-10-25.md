---
role: "architect-cto"
class: threads
topic: "linux-archive-glibc-floor-v0-1-1-2026-10-25"
description: "j369 2026-10-25 — Linux archive floor glibc 2.35 x86_64 ruled; ADR-0065 A 2026-10-25 (second) on IW #444; v0.1.1 PATCH owed; what I still owe the owner and p2p-network-dev-01"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - eda5f09a273e2e8b
---

## j369 2026-10-25 — Linux archive floor glibc 2.35 x86_64 ruled; ADR-0065 A 2026-10-25 (second) on IW #444; v0.1.1 PATCH owed; what I still owe the owner and p2p-network-dev-01

**Ruled 2026-10-25** (REPLY 01a1736d-9ab5 to p2p-network-dev-01's QUESTION 01a17366-34c8, from the owner's external review of v0.1.0): the Linux archive's floor is glibc 2.35 on x86_64 (Ubuntu 22.04+, Debian 12+, Fedora 36+). v0.1.0's `human-desktop` needs GLIBC_2.43 (built on `ubuntu-latest`); the daemon binaries need 2.34, so the fleet's pin on v0.1.0 holds (INFO 01a1736e-6767 to the owner). The rule is ADR-0065 A 2026-10-25 "The Linux archive's minimum platform is public API: raising it is breaking, lowering it a patch", plus plan §22's paragraph with the figures: IW #444 (branch develop-qzapp/architect-cto-01/feat/j369-linux-archive-minimum-platform-glibc-2, 1 work commit, arms on the owner's word).

**Who builds what:** devex-tooling's j231 branch `feat/j231-release-signing-and-glibc-floor` (b4b5911e) carries the ubuntu:22.04 build and p2p-network-dev-01's `check_glibc_floor.py` (folded from their for/ branch) in one PR with release signing; the release page sentence is rust-ui-dev's with language-culture-en's English; `packaging/linux/DEPLOYING.md`'s requirements line is p2p-network-dev's.

**Still owed by me:**
- the release-signing ADR supply onto that same j231 branch (job j366, DECISION 01a17326-c5db), unrebased over b4b5911e; REPLY to devex-tooling 01a17381-f9b4 promised the sha;
- when devex-tooling's PR is on main: ask the owner to tag v0.1.1 and to add one line to v0.1.0's release page (GUI there needs glibc 2.43; v0.1.1 states the floor); then tell the owner the tag exists so the fleet's release.json pin can move;
- a 2.34 image (RHEL 9) is a measurement after the job builds in the floor image, never on v0.1.1's path; if it builds, the next PATCH lowers the floor;
- the fleet's archive-run conformance rows (j125's REQUEST 01a1722a to p2p-network-dev-01) run from v0.1.1's archive.

**Why:** a floor is what was built and checked; the host a program runs on is part of what a previous install relies on, so the ADR had to say it or a raise reads as a dependency bump.
**How to apply:** any release whose archive needs more than its page states is a PATCH that states the floor, never a replaced tarball. Cite ADR-0065 A 2026-10-25 only once #444 is on main ([[reflection-cite-an-amendment-only-once-its-pr-is-on-main]]).

*References: reflection-cite-an-amendment-only-once-its-pr-is-on-main*

*Observed 2026-10-25 (architect-cto)*
