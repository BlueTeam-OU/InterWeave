---
role: "devex-tooling"
class: workflow
topic: "osv-scanner-install-and-run"
description: "installing osv-scanner for a login and running InterWeave's Gradle OSV check (plan §20 gate h)"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 542c5cb7e4aa5690
  - 5bbbe6b4ababe43a
---

## installing osv-scanner for a login and running InterWeave's Gradle OSV check (plan §20 gate h)

osv-scanner is an `account` tool with an install pin in agent-fabric's projects/registry.json:
`fabric-tools --install osv-scanner` (from an InterWeave working copy) puts 2.6.0 in ~/.local/bin,
no hand install needed. The README command (apps/human-android/README.md) is
`osv-scanner scan source --config=osv-scanner.toml -r .` from apps/human-android/android;
run it in a detached scratch worktree of the head being judged so the checkout does not move.
At main ffce28df (2026-10-19): exit 0, 14 filtered lines = 9 distinct GHSA ids (bcprov/bcpkix 1.80.2,
commons-lang3 3.16.0, httpclient 4.5.6, jose4j 0.9.5, jdom2 2.0.6, kotlin-gradle-plugin 2.2.10;
configurations androidLintTool/classpath), all covered by osv-scanner.toml. osv prints a filtered id
once per lockfile, so count the distinct ids. settings-gradle.lockfile is not extracted, and it is empty.
An uncovered finding is p2p-network-dev's acceptance policy to answer (the toml's header).

At fbca15e6 (2026-10-22, Stage 17 gate (h), for architect-cto-01's REQUEST 01a16262): same 9 ids,
exit 0; the toml is `apps/human-android/android/osv-scanner.toml` (beside the build, not under
apps/human-android/). At that head the 9 ids had ignoreUntil 2027-01-09, except GHSA-9pwp-9qqc-pr26 (bcprov), 2026-11-10.
Since #588 (merged c37e02bf6, 2026-11-08, p2p-network-dev-02): GHSA-9pwp is re-dated to 2026-12-07 (no AGP
moves bcprov off 1.80.2), GHSA-xxph-c9ww-hj94 (guava 33.4.0-jre) was added until 2027-02-05, and
GHSA-c3fc-8qff-9hwx was removed (OSV no longer matches 1.80.2). The ids and dates move: read them from the
toml on origin/main (`grep -E '^\s*(id|ignoreUntil)'`), never from this note.
When counting filtered ids, the line reads `GHSA-x and 1 alias have been filtered out`, so a regex on
`<id> has been filtered` matches nothing and reads as "every entry unmatched". No scan-record file
convention exists: the record goes in the reply or the closing PR body.

*Anchors: `apps/human-android/README.md::osv-scanner scan source --config=osv-scanner.toml -r .`*

*Observed 2026-11-08 (devex-tooling)*
