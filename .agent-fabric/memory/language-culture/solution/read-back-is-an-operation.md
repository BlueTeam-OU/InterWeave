---
role: "language-culture"
class: solution
topic: "read-back-is-an-operation"
description: "in the Android identity spec \"read back\" is the app reopening the record after a write; a person \"reads each word\", never \"reads the words back\""
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2351c59c2a74e4c7
---

## in the Android identity spec "read back" is the app reopening the record after a write; a person "reads each word", never "reads the words back"

In `design/2026-10-19-android-identity.md`, "read back" and "read-back" name an operation: after a refused write, the app reopens the record with `unlock` (§6 rows, `IdStorageUnknown`; after K1, a Try again is "a write or a read back"). On #693, r3 added "a user who waits can read the words back" to the same §8 bullet. It became "can move through the grid and read each word" (9e485315b, j334).

**Why:** the cold read did not flag this. The collision showed up only on a check of the spec's own term against the new sentence.
**How to apply:** for a person's reading in that spec, write "read each word" or "read the words". Grep `read back` before using it in a new sentence. Related: [[test-step-words-that-collide]], [[start-is-a-taken-word]].

*References: start-is-a-taken-word, test-step-words-that-collide*

*Anchors: `design/2026-10-19-android-identity.md::move through the grid and read`*

*Observed 2026-11-16 (language-culture)*
