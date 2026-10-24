---
role: "p2p-network-dev"
class: solution
topic: "derived-notice-rules-must-follow-the-record-literally"
description: "release_notices.py derivation (#374, merged 65c3e4c7): every lenient reading of ADR-0058 2.ii produced another review round; apply the record's text literally, compare whole licence bodies, never marker fragments"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ff64225fde89c6a2
---

## release_notices.py derivation (#374, merged 65c3e4c7): every lenient reading of ADR-0058 2.ii produced another review round; apply the record's text literally, compare whole licence bodies, never marker fragments

#374 (j99, ADR-0058 notice derivation) ran five review rounds. Codex found one new edge case per round. Each came from a reading that was more lenient than the record's literal text:
- I took only the first header found.
- I checked a header's licence by matching fragments of its text.
- I exempted the crate's own OR terms and "at your option" headers.
- My copyright-line test read the word "copyright" anywhere, then only a line starting with it.
- My list of licence names was closed and case-sensitive.

What finally held:
- Ship every distinct header of the compiled files (src/ and build.rs).
- A header carries its licence only if it holds the whole committed SPDX body, with whitespace and case folded.
- Every licence any header names must have its text shipped.
- The reviewed list records the header count and extra_texts per crate.

**Why:** ADR-0058 2.ii says a header naming a licence the notice doesn't otherwise carry brings that licence's text. Each exemption I added was a hole a reviewer could build a fixture through.

**How to apply:** for a compliance or derivation tool, implement the record word for word and ship more text, never less. Enumerate the whole class with the review-class judge at the first P1 (derived-class.md style). Carry latent P3s as a job rather than another round. NAMED's closed-list gap is j108.

Related: [[review-loop-same-invariant-escalate]], [[release-notices-crates-without-licence-files]].

*References: release-notices-crates-without-licence-files, review-loop-same-invariant-escalate*

*Anchors: `tools/ci/release_notices.py::carries_text`, `tools/ci/release_notices.py::NAMED`, `tools/ci/notice_derived.json`*

*Observed 2026-10-22 (p2p-network-dev)*
