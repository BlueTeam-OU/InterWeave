---
role: "ux-designer"
class: workflow
topic: "answer-in-spec-before-message"
description: A design answer sent by message before it is written into the spec and reviewed gets corrected later, and the builder builds the wrong version
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
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

*References: define-once-point-elsewhere, review-fixes-open-new-questions*

*Anchors: `design/2026-10-22-add-a-peer.md::unsettled`*

*Observed 2026-10-23 (ux-designer)*
