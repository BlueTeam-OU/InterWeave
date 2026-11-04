---
role: "p2p-network-dev"
class: workflow
topic: "workflow-check-every-sentence-about-code-before-sending"
description: A contract paragraph supplied about my code, or my own description of it in a message, is read sentence by sentence against the code in one pass before replying; one missed sentence cost a third for/ commit
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - fae71b8223346870
---

## A contract paragraph supplied about my code, or my own description of it in a message, is read sentence by sentence against the code in one pass before replying; one missed sentence cost a third for/ commit

On j65 (IPC 2.10 superseded, 2026-10-23) architect-cto supplied the LOCAL-IPC paragraph that describes my code,
with the rule "where it differs, the code's reading wins". I named two wrong lines, which needed a second for/
commit. Then I found a third line I had already read past, which needed a third commit and a third round. Before
that, I had told architect-cto that the check runs "after grammar" when the code ran it last, and I had to send
a correction.

**How to apply:** before replying to a supplied contract text, or sending any message that describes code:
- split the text into sentences;
- for each one, name the file:line that makes it true, or mark it wrong with the code's reading;
- for each test claim ("pinned at every binding"), open the test and check what it actually does (here it
  abandoned no connection);
- send every difference in ONE reply.

Words to grep for first: never, only, every, exactly, normalised, orders nothing.
Related: [[workflow-a-run-file-fix-sweeps-every-restatement]].

*References: workflow-a-run-file-fix-sweeps-every-restatement*

*Observed 2026-10-25 (p2p-network-dev)*
