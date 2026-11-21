---
role: "p2p-network-dev"
class: workflow
topic: "supplied-wording-only-when-is-a-claim"
description: "Wording another role supplies for a doc of mine is checked clause by clause against the source it cites, \"only when\" above all"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 95c0072251928806
---

## Wording another role supplies for a doc of mine is checked clause by clause against the source it cites, "only when" above all

Wording another role supplies for a file of mine is checked against the source it cites before it lands. Verifying the supplier's facts (which files are in the archive, which step numbers) is not enough: the sentence's quantifiers ("only when", "both", "nothing") are claims too. In #620 (merged 2026-11-10) the supplied channel sentence said "only when started with --channels". INSTALL.md step 7 also names the development flag and channel_enable. Each fix round then produced the next defect: the F1 fix made channelsEnabled necessary for every login, while INSTALL.md:138 says only some logins need it (F4). Three review rounds for one sentence.

**Why:** a sentence summarising a conditioned source drops conditions silently, and the reviewer's suggested fix wording can be wrong in the same way.

**How to apply:** before committing supplied or review-suggested prose, list the source's conditions (who, which flag, which login kind, measured vs read). Then check each "only", "every" and "needs" against that list.

*Anchors: `packaging/linux/README.md::allowedChannelPlugins`, `packaging/claude-plugin/INSTALL.md::Keep both keys`*

*Observed 2026-11-10 (p2p-network-dev)*
