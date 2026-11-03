---
role: "rust-ui-dev"
class: workflow
topic: "release-disk-after-tests"
description: "Test-run disk rule (owner, 2026-10-05) — team.md's form: clean target/ when LARGE or after a dep-graph change, not after every run; measure du before/after and report"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3b9c7c494a579d87
---

## Test-run disk rule (owner, 2026-10-05) — team.md's form: clean target/ when LARGE or after a dep-graph change, not after every run; measure du before/after and report

The rule is agent-fabric `identities/prompt/team.md` ("A test run leaves behind nothing it did not find"), as the owner ruled via architect-cto (relay seq 12550, 2026-10-05), superseding fabric-coordinator's broader broadcast of the same day (seq 12546, "remove target/ after every run"):
- containers/volumes a run started are gone when it ends; scratch under the session scratchpad, never the tree;
- a build that changed the dependency graph (bump, feature-set switch) cleans its target;
- an incremental cache is removed when LARGE (tens of GB = large; a few GB of warm cache the next run reuses stays) — not after every run;
- measure (`du -sh`) before cleaning, report what was removed and how much; own home only.

**Why:** /home on develop-qzapp hit 77%; my interweave target/ was 71G (removed 2026-10-05).

**How to apply:** check `du -sh target` at the end of a verification pass; clean if tens of GB or the dep graph changed. See [[desktop-e2e-stale-binary]] for the e2e rebuild caveat.
Since 2026-10-18 (reflection; MESSAGE-IDs 01a150cc-5240, 01a15af5-0573, 01a15b83-577b): one InterWeave worktree's target/ is 60-130G, the Android Gradle build/ a further 2-3G, and each worktree builds its own, so "large" is every worktree's target. /home filled twice on 2026-10-20 with writes failing for others. Clean a worktree's target when its job ends or the suite run that needed it ends, stop Gradle and Kotlin daemons first, remove a worktree whose branch merged with its review heads, and measure with du (fabric-ctl disk is host-operator only).

*References: desktop-e2e-stale-binary*

*Observed 2026-10-05 (rust-ui-dev)*
