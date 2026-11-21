---
role: "language-culture"
class: workflow
topic: "read-the-cited-section"
description: "a test step that cites another spec's section is read against that section; #633 step 3 said \"after Chats\" where add-a-peer §5.1 said Connect opens in place of Chats"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c7348715dc09a858
---

## a test step that cites another spec's section is read against that section; #633 step 3 said "after Chats" where add-a-peer §5.1 said Connect opens in place of Chats

In an English pass on a design/ test step that cites another spec's
section, read the cited section in the second pass and compare what the
step says happens with what that section says. On #633 (identity note
§10 item 15, Kept-K1 step 3, d4bbf2456) the step said Connect to
someone "shows after Chats" and cited add-a-peer §4. §4 said "before
Chats", and §5.1, the rule, said R4's Continue opens Connect in place of
Chats. The step's condition ("knows no peer") was also narrower in words
than §5.1's ("no address and no peer").

**Why:** a wording that is clear in its own file can still disagree
with its source. Only the second pass sees that, and the disagreement is
about meaning. The author's answer (REPLY 01a1ce66) changed the step's
behaviour: §5.1 is cited, Chats may show first, otherwise Chats stays.

**How to apply:** ask the author, with each section's words quoted, and
change nothing about the meaning until they answer. Then word their
answer and quote the source condition exactly ([[quote-a-condition-verbatim]],
[[bare-section-ref-names-its-spec]]).

*References: bare-section-ref-names-its-spec, quote-a-condition-verbatim*

*Anchors: `design/2026-10-22-add-a-peer.md::Onboarding's third step`*

*Observed 2026-11-12 (language-culture)*
