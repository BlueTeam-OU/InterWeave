---
role: "architect-cto"
class: threads
topic: "j458-start-listing-prose-2026-10-31"
description: "j458 (rust-ui-dev-02 QUESTION 01a18cfc): the prose after #520 removed list_all — IW #523 (4 work, 2 fix; under the minimum, the owner's word), head 4bb7ba34 (j472 read-walk sentence, re-review dispatched 2026-10-31); REPLY 01a191eb sent…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2a7f6dbfd42e9521
---

## j458 (rust-ui-dev-02 QUESTION 01a18cfc): the prose after #520 removed list_all — IW #523 (4 work, 2 fix; under the minimum, the owner's word), head 4bb7ba34 (j472 read-walk sentence, re-review dispatched 2026-10-31); REPLY 01a191eb sent; what the reviews taught about the start's reads

IW #523 (opened 2026-10-31, docs only, not a boundary): SPIKES.md 631/672
past tense, SPIKE-011 bullet "The baseline's read, restated on the window
(A 2026-10-31)" with #521's figures placed, plan §29 P2-1 closed on #515,
#517, #519, #520. Under the minimum: my own PR, so the owner's word arms
it. rust-ui-dev-02 told in REPLY 01a191eb, with two of their spike-011
files still naming list_all as the current read (harness main.rs 264-266,
README 34-37) — theirs to date.

**What the reviews corrected in my first text** (F1-F5, all real):
- The start still walks the PENDING rows whole, twice (`list_pending`,
  `own_cards_pending`), reads the contact directory, and reclassifies
  every inbound row on a stale kind version; only the unread/kept walk
  went with the window.
- The harness's read_walk never called list_all: it calls the three
  page reads directly, which still exist. "Only in the harness, at its
  pin" was wrong.
- A page is served by the v10 per-conversation indexes, not the walk's
  `(received_at, row_id)` ones; kind indexes are v10's AND v11's.
- The CLI's two-pass walk reads twice only a row the ceiling let go; a
  held row once. "About two of the row's figure" was an unmeasured
  estimate and is gone.
- A device record's "shown by the next screenshot" is an upper bound,
  stated as one, with its unit and "one try".

**How to apply:** before restating what a client reads "now", read the
facade's start function and the harness's own read, not the PR body of
the removal; a removal PR says what left, not what stayed.

**j472 (2026-10-31):** 4bb7ba34 adds beside the row that it is the rows' read and never the whole start; at the pin the three walks were all of list_all, from #359 (95674b26) to #520 list_all also built card proposals and contact views, untimed. rust-ui-dev-02 told in REPLY 01a191fe (their #524 README needs nothing).


2026-10-31: owner "merge 523 525". #525 had merged on my AUTHORIZES (0a0e0d0e); #523 armed by me at 4bb7ba34 (2 work, 2 fix, owner's word in the basis); closes when on main.
2026-10-31: #523 MERGED 65546db6; j458 and j472 closed by sweep.

*Observed 2026-10-31 (architect-cto)*
