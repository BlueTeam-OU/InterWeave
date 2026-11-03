---
role: "rust-services-dev"
class: workflow
topic: "spike-facts-cite-committed-runs"
description: "Writing a SPIKE-001 measurement — every fact must rest on a committed run's recorded field or be marked; design runs to isolate each claim"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 8ce47e516b8076fa
---

## Writing a SPIKE-001 measurement — every fact must rest on a committed run's recorded field or be marked; design runs to isolate each claim

On PR #420 (S23/S24, 2026-10-24) the review class returned findings for six rounds, nearly all of one class: a README/SPIKES.md/INSTALL.md sentence claiming more than the committed evidence. Instances: a cause stated as measured when the run did not reproduce the original conditions; a claim resting on a run that also had a second variable set (needed an isolating run); a value seen only in raw output (first attempts, raw transcripts, screen) not committed; a table cell describing configuration rather than what was sent; an INSTALL command differing from the measured one.

**Why:** the review gate holds a spike record to "supported by committed evidence, or marked read-from-binary / not measured", and each overclaim costs a fix round.

**How to apply:** before writing facts, (1) give each claim a run that varies only that thing, with its control; (2) record in the driver's run.json every value a fact will cite (permission mode, write times, exit status), since extract.py keeps little; (3) re-run rather than cite a replaced attempt; (4) document exactly the measured command; (5) say "inferred" for any cause the runs only fit. Then grep the prose for each number against the evidence before opening the PR. Related: [[review-range-with-main-merge-is-partial]].

*References: review-range-with-main-merge-is-partial*

*Anchors: `spikes/spike-001/extract.py::def stream_json_evidence`, `spikes/spike-001/s24_print.py::permission_modes`*

*Observed 2026-10-24 (rust-services-dev)*
