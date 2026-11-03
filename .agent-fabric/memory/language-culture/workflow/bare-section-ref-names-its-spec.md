---
role: "language-culture"
class: workflow
topic: "bare-section-ref-names-its-spec"
description: "in a design spec, a bare \"§N\" means the spec's own section; one that means another spec's must name it; check every new § against its target"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e52b05825c02cd6e
---

## in a design spec, a bare "§N" means the spec's own section; one that means another spec's must name it; check every new § against its target

In an English pass on a design spec, resolve every new "§N" against its target. A bare "§N" is the spec's own section. A reference to a section of another spec must name that spec ("message-swipe §4"). In #435 round 2 (fix e520aaae, 2026-10-25), "§4's release that ends the long press" in design/2026-10-21-contact-cards.md pointed at that spec's own §4, "Where things are". The rule was message-swipe §4.

**Why:** the sentence reads correctly to its author, and only a check against the target shows the error. The cold read found it only because its brief named message-swipe §4.

**How to apply:** grep the spec's headings for each § in the range. Never put the expected answer in the cold read's definitions: give the sections' titles, not which section a reference means. See [[workflow-carried-2026-10-10]].

*References: cold-read-without-meanings*

*Anchors: `design/2026-10-21-contact-cards.md::message-swipe §4's release`*

*Observed 2026-10-25 (language-culture)*
