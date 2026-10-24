---
role: "architect-cto"
class: threads
topic: "adr-0043-recovery-warning-no-site-2026-10-21"
description: "ADR-0043's 'Recovery UX warns when importing an identity expected to still be active elsewhere' has no implementation site (transportctl, desktop, Android); found by Stage 18 TM-48 (IW #337); the warning is owed, detection is the limit"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 28c7c3e9e96a0603
---

## ADR-0043's 'Recovery UX warns when importing an identity expected to still be active elsewhere' has no implementation site (transportctl, desktop, Android); found by Stage 18 TM-48 (IW #337); the warning is owed, detection is the limit

2026-10-21: p2p-network-dev-02's Stage 18 S11 work (IW #337, TM-48 row)
found that nothing enforces ADR-0043's one-phrase-one-active-node rule
and called "whether restore should warn" a product decision. It is
decided: ADR-0043 Operational implications line 27 requires the
recovery UX to warn. Grep on origin/main 1a4fa590: `apps/transportctl`
(README, cli.rs) prints no warning; human-client-ui.md and the Android
spec state none. Ruling (REQUEST 01a15eaf to p2p-network-dev-02):
transportctl `identity restore --new|--replace` prints the warning
before the phrase prompt, no flag, no refusal (the owner's CLI
direction); a test reads it. Mine (j224, after #337 lands, same file
as its matrix edits): TM-48 row = rule + owed warning + detection limit;
C-02 row = decided (3)'s residual; human-client-ui.md recovery-screen
sentence, then REQUEST to rust-ui-dev. Shape lesson: a decision whose
only sentence is in an ADR's implications and never in a client spec
is a decision with no site — the Stage 18 matrix is where those show.

*Observed 2026-10-21 (architect-cto)*
