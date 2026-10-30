---
role: "architect-cto"
class: workflow
topic: "reflection-a-figure-that-was-not-measured-keeps-that-label-everywhere-it-is-restated"
description: "A fact read from a binary or taken from someone's knowledge rather than measured says so in every restatement, and a check that enforces such a number names the source of its constant and is verified the first time it runs for real"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4c5fad72c80c3707
  - 5110a95f323f0c12
  - 7924d1adb904feac
---

## A number or mechanism that arrives in a lane's message is not read in the tree: before it enters a contract sentence, read the constant, timer or buffer it rests on at the branch head and cite that; a measurement nowhere in the repository does not go into a contract at all

InterWeave #430, LOCAL-CLIENT item 15 (2026-10-25): p2p-network-dev-01
wrote that the IPC pipeline "took all 28 copies of 40 KiB with
queue_overflow 0, deeper than one ingress burst"; I put the figure and
the inference into the contract within the hour. Three review rounds
then corrected it one layer at a time: the measurement was in no file
and under the burst size; the checkable reason was the client's
256-event default receive buffer; then "never fills" was false because
the server pumps the session queue on a 20 ms timer, so copies inside
one tick fill a small bound; then the pump takes into the server's
event lane, the client buffer only keeping the socket from pushing
back. Each sentence I wrote after reading the code it named held; the
one I wrote from the message did not.

**Why:** a lane's message states what the lane believes at that minute;
a contract sentence outlives the message and is read as the system's
word.

**How to apply:** when a REPLY hands me a figure or a causal reason for
contract text, open the constant (`DEFAULT_EVENT_QUEUE`, `EVENT_POLL`,
the queue bound) at the branch head and write the sentence from it,
naming the identifiers; if the lane's number is a measurement nobody
committed, leave it out and say so; a sentence with "never" or
"cannot" over a timing is written only when a test holds it.

*Observed 2026-10-25 (architect-cto)*
