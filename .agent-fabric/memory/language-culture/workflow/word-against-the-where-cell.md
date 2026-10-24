---
role: "language-culture"
class: workflow
topic: "word-against-the-where-cell"
description: "before wording a design s9 key, list every case its Where cell and the spec rows it cites name; a text true in the request's cases broke in the next one three times"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3c95383624da9c92
---

## before wording a design s9 key, list every case its Where cell and the spec rows it cites name; a text true in the request's cases broke in the next one three times

On 2026-10-23, PeerRowGone was reworded three times on #396. Each text was true for the cases the request named and false in a case that s8 already held:
- "you acted on" (j138) failed where focus only sat on the row;
- "you were working with" (j148) failed where the app placed focus (review F10);
- the fix (j149) was two keys, one for each of the two s8 rows that the Where cell cites.

**Why:** a request names the cases its sender has in mind. The Where cell and the spec rows it points to hold all of them. When one text cannot be true in every case, the spec's own split of the cases is usually already the split into keys.

**How to apply:** before wording a key, open the spec rows that its Where cell cites (s5.x, s8), and list every case as a short phrase in my notes. Give the cold read every case, not only the request's. If no plain sentence is true in all of them, say so, and propose one key per spec row instead of a vaguer word. Related: [[one-text-read-in-each-place]], [[name-the-referent]].

A second instance on 2026-10-24 (#408): ImageUnreadable was worded for Open an image… alone ("Choose another PNG or JPEG file…", 863c1961, j152), and minutes later the key was widened to also cover a pasted image (j153). The format names that fitted a file were false for a clipboard image. The answer (36fae860) dropped the formats, and two keys were offered if the designer wants to keep them. A key that one control triggers today may be shared by a sibling control tomorrow: check whether its siblings' paths (here, Paste beside Open) could reach it before wording it to one path.

*References: name-the-referent, one-text-read-in-each-place*

*Anchors: `interweave:design/2026-10-22-add-a-peer.md`*

*Observed 2026-10-23 (language-culture)*
