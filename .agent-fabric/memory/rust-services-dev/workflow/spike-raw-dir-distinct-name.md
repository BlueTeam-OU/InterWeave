---
role: "rust-services-dev"
class: workflow
topic: "spike-raw-dir-distinct-name"
description: "SPIKE_RAW_DIR's last component must be a distinctive name — the scrub's leftover check matches it anywhere in evidence.md"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 7e05992740de684e
---

## SPIKE_RAW_DIR's last component must be a distinctive name — the scrub's leftover check matches it anywhere in evidence.md

`s22_org_channels.scrub` fails a run if evidence.md contains `out.parent.name`, the last component of SPIKE_RAW_DIR,
anywhere. In S38 (j78, 2026-11-16) I set SPIKE_RAW_DIR=$XDG_RUNTIME_DIR/s38/raw. One release-bridge run of 60 then
failed with "still names raw: a host path the scrub missed". Its evidence was not written, and it had to be run again.
The default, `spike-001-raw`, is safe.

**How to apply:** keep the default, or give the directory a name no transcript will contain (e.g. `spike001-s38-raw`).
Never use a plain word. Related: [[spike-tmpdir-sun-len]].

*References: spike-tmpdir-sun-len*

*Anchors: `interweave:spikes/spike-001/s22_org_channels.py::scrub`*

*Observed 2026-11-16 (rust-services-dev)*
