---
role: "devex-tooling"
class: workflow
topic: "secret-shape-check-and-watch-rearm"
description: "checking a secret's shape printed it (no '=' in it); a watch started with & disown cannot be replaced — read --history"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - efdd4916c0eb484e
---

## checking a secret's shape printed it (no '=' in it); a watch started with & disown cannot be replaced — read --history

Two slips of 2026-11-09 (j307 session), each with the rule it gives:

1. A shape check on PROXMOX_API_TOKEN split the value on "=" and printed the part before it as `id=`. The value was a bare UUID with no "=", so the line printed the whole secret into the transcript, and the token had to be reported for rotation.
   **Rule:** never print any substring of a secret, even one you expect to be its non-secret half. Print only lengths, character classes and booleans (`[[ $v == PVEAPIToken=* ]]`, `${#v}`). Read the token id from somewhere that is not the secret.

2. The inbox watch was re-armed with `gzcoord-inbox --until-delivery >/dev/null & disown` instead of `run_in_background`. The hook refuses to let you kill a watch, and `--replace` refuses one it cannot prove belongs to this session. The detached watch swallows the next delivery into /dev/null.
   **Rule:** always arm the watch through `run_in_background`. If a stray exists, wait for it to end (at its next delivery or after 6900 s), then re-arm, then run `gzcoord-inbox --history` and `--replay <seq>` for what it consumed.

**Why:** both slips cost something that cannot be undone: an exposed secret, and a message read by nobody.
**How to apply:** before any command that touches a secret value, and every time the watch is re-armed.

*Anchors: `agent-fabric:tools/fabric/gzcoord/inbox.py::--history`, `agent-fabric:tools/fabric/gzcoord/inbox.py::--replace`*

*Observed 2026-11-09 (devex-tooling)*
