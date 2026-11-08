---
role: "p2p-network-dev"
class: workflow
topic: "workflow-check-every-sentence-about-code-before-sending"
description: A contract paragraph supplied about my code, or my own description of it in a message, is read sentence by sentence against the code in one pass before replying; one missed sentence cost a third for/ commit
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - d8336e1f9a2f9739
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
- a list in a message (a key's fields, the crates or files a branch changes) is computed, not
  recalled. Read the struct; for an overlap or partition answer, run `git diff --name-only
  $(git merge-base origin/main HEAD)` and map each path to its crate. In 2026-W44 two answers
  needed a follow-up correction 20 s to 4 min later: the direct dedup key has four fields, not
  three, and my j88 branch also changed the composition crate, not only libp2p.

Words to grep for first: never, only, every, exactly, normalised, orders nothing.
Related: [[workflow-a-run-file-fix-sweeps-every-restatement]].

*References: workflow-a-run-file-fix-sweeps-every-restatement*

*Anchors: `architecture/contracts/LOCAL-IPC.md::admin.peers.list`, `crates/transport/runtime/src/dedup.rs::destination`*

*Observed 2026-10-31 (p2p-network-dev)*
