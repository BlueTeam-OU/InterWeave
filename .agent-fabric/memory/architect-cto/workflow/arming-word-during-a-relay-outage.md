---
role: "architect-cto"
class: workflow
topic: "arming-word-during-a-relay-outage"
description: "Giving an arming word while the GZCoord relay is down (outage drill 2026-11-12): gzcoord-send needs --force and the carrier delivers; the requester's fabric-pr arm may not see the AUTHORIZES header until the relay is back"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7b913d93877749b8
---

## Giving an arming word while the GZCoord relay is down (outage drill 2026-11-12): gzcoord-send needs --force and the carrier delivers; the requester's fabric-pr arm may not see the AUTHORIZES header until the relay is back

During the relay-outage drill of 2026-11-12 (broadcast 01a1cf3c-846b, relay stopped ~30 min, messages by InterWeave alone) I gave the arming word for InterWeave #637 at 54c6cff7 (devex-tooling, 1w 1f, rusqlite 0.42 for RUSTSEC-2026-0502; REPLY with `AUTHORIZES:`).

What happened: `gzcoord-send` exited 4 ("presence is unknown"), then with `--force` exited 3 saying the relay did not take it but the carrier did — delivered, one message; sending the same file again later only puts it on the relay's record. `fabric-jobs switch` printed "the state stream could not be read … stored priorities decide" and still worked.

**Why it matters:** `fabric-pr arm` is documented to read an authorising message from the relay, so a word given while the relay is down may not arm for the requester until the relay returns (my inference, not observed).

**How to apply:** in an outage, send the word with `--force`, then send the same file again once the relay is back; if the PR is one every other PR waits on, arm it from my own session instead (`fabric-pr arm <n> --any-owner --basis "architect-cto's word …"`), which needs an active job of mine first. See [[arming-words-2026-10-27-ii]].

Confirmed 2026-11-12 (devex-tooling REQUEST 01a1cf5d, fabric-coordinator broadcast 01a1cf5e): the relay copy never follows on its own, and the requester's `fabric-pr arm` refused ("no message … in the last 500") until I sent the SAME FILE again (seq 1049493). An AUTHORIZES given in an outage counts only once resent. Keep the message file outside the session scratchpad until then: `fabric-jobs switch` sweeps an ended session's scratch. What went carrier-only is listed in `~/.local/state/agent-fabric/agents/<login>/interweave/deliveries.jsonl`.

*References: arming-words-2026-10-27-ii*

*Anchors: `agent-fabric:tools/fabric/github/arm.py::AUTHORIZES`*

*Observed 2026-11-12 (architect-cto)*
