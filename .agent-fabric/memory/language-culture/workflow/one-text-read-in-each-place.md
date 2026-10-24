---
role: "language-culture"
class: workflow
topic: "one-text-read-in-each-place"
description: "a text one tool writes into several artifacts must be read as the reader of each one; deictic words (\"this release\", \"below\") break first"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 22cce445c0159171
---

## a text one tool writes into several artifacts must be read as the reader of each one; deictic words ("this release", "below") break first

When one generator writes the same text into several artifacts, I read it once in each place, as that place's reader. In InterWeave #374, `tools/ci/release_notices.py` writes one notices page in two places: THIRD-PARTY-NOTICES.md in the Linux archive, and THIRD-PARTY-NOTICES-android.md attached beside the APK. The draft said "What this release ships also contains the following crates". That was false in each file: a release holds both builds, and each file lists only its own build's crates. The final text says "The InterWeave software that comes with this file" (REPLY 01a16419-62a9, 2026-10-22).

**Why:** words that point (this release, below, here, the file) take their meaning from the place where they are read. A text written while thinking of one place carries that place into the others, and a read of the source does not show it.

A second case on 2026-10-23, in a UI string rather than a file: IdR1NewKeys shows on R1 and on K1. I cut its positive half ("your words restore your PeerId") because IdR1Settings says that on R1. On K1 the line stands alone, so #396's review (F2) caught the missing half. The fix is one text with both halves, which costs one repeated clause on R1.

**How to apply:** for a template or generated text, first ask the caller or the code where it ships. In the first pass, read it once per place. Give the cold read every place too: it caught "this release" only because the brief named both places. Related: [[cold-read-without-meanings]].

*References: cold-read-without-meanings*

*Anchors: `interweave:tools/ci/release_notices.py::render`*

*Observed 2026-10-23 (language-culture)*
