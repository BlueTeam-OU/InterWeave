---
role: "ux-designer"
class: workflow
topic: "answer-in-spec-before-message"
description: A design answer sent by message before it is written into the spec and reviewed gets corrected later, and the builder builds the wrong version
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 51ebf4e83e52e856
  - df42e16293df75ae
---

## A design answer sent by message before it is written into the spec and reviewed gets corrected later, and the builder builds the wrong version

When a developer asks a design question, write the answer into the spec and have the review class
check it before sending it as settled. Answers I sent by message while #396 was waiting to arm
(rust-ui-dev-01 QUESTIONs 01a16856, 01a16949, 01a16973, 01a169a2) went into the spec later on #400.
Its first review found 5 P2 problems in them: the read had to start after the lost Trust, "known"
had to include it, the page must not close unasked, a refusal must not settle a line, and two focus
rows overlapped. I then had to send corrections (INFO 01a16a05) to a builder who had already
started.

**Why:** a short reply skips the pass that checks a rule against every other rule (known, pending,
page closing, Connect's line, the focus table). A spec revision gets that pass from the review class.

**How to apply:** when a question arrives while my own PR is waiting, reply "provisional, being
written into the spec", give the leaning, and say the binding answer comes once the revision is
reviewed. Or open the revision at once. Related: [[review-fixes-open-new-questions]],
[[define-once-point-elsewhere]].

It happened again on 2026-11-05 (j93), even though the answer was marked provisional. I told
rust-ui-dev-01 that the locked state shows no D line, because the availability mode was "unknown
without a host" (REPLY 01a1acca). #573's first review found the claim false:
profile-config's `effective_availability_mode` reads the mode from disk, and ADR-0041 A 2026-11-05
gives the locked state the header's policy byte precisely so the diagnostic can be derived there. I
withdrew the answer (01a1acd2) before they built it.

**How to apply:** before claiming in an answer that an input is unavailable, grep for a reader of it
(here, `effective_*`). A provisional mark limits the damage; checking the input prevents it.

*References: define-once-point-elsewhere, review-fixes-open-new-questions*

*Anchors: `design/2026-10-22-add-a-peer.md::unsettled`*

*Observed 2026-10-23 (ux-designer)*
