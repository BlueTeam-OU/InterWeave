---
role: "architect-cto"
class: workflow
topic: "reflection-a-second-amendment-of-a-record-re-reads-the-first-s-limit-sentences"
description: "W44 - two amendments of ADR-0055 in one day: the second (the port rule measured) made the first's limit sentence ('decision 5's port rule stays unmeasured') false, a parenthesis cited a measurement the kept test did not render, and 5b's…"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d016ecaa9c3db086
---

## W44 - two amendments of ADR-0055 in one day: the second (the port rule measured) made the first's limit sentence ('decision 5's port rule stays unmeasured') false, a parenthesis cited a measurement the kept test did not render, and 5b's 'rule 9's door admits the literal whatever its class' named one half of a two-half door; before pushing an amendment grep the record for every limit, Revisit and 'measured' sentence, name a kept test for each cited measurement, and read both halves of a door in code

InterWeave #475 (2026-10-27): decision 5b (two hosts over an overlay,
with its limits) landed beside decision 5's "port rule measured"
note. The blind review found F1 (5b still said the port rule stays
unmeasured; decision 5's last sentence and the Revisit condition
"a second host" now sat against 5b) and F2 (5b's parenthesis
"measured 2026-10-27 in the case below" named a measurement
two_hosts.rs did not render: the kept case ran the wildcard only).
#475 landed on the owner's word, so the fix was a further dated note
(ii) on the follow-up branch, citing the third case p2p-network-dev-01
then added by name. The same day 5b's "rule 9's door admits the
literal whatever its class" proved half true: the configuration entry
admits a 100.64/10 literal, the run-time add (special_use_host and its
unit test) refuses it, so an overlay peer is a config entry by design
(a8fe9ea1, DEPLOYING.md's fact cited).

**Why:** a record's limit, Revisit and "measured" sentences are
cross-references to its own other sentences and to tests in the tree;
an amendment that changes one side leaves the other stale, and a
landed amendment is never edited, so the miss costs a further note.

**How to apply:** before pushing an amendment commit, grep the record
(body, history note, digest line) for `unmeasured|not measured|stays|
Revisit|measured 2026` and read each hit against the new text. Every
"measured" clause names the test in the tree that renders it; a
measurement no kept case renders is written as "measured once on
<date>, no kept case". When a rule says a door admits something, read
both halves of the door (the authored entry and the run-time add) in
the code before writing "whatever its class". Related:
[[a-review-fix-reaches-every-derived-record]],
[[history-notes-are-append-only]].

*References: a-review-fix-reaches-every-derived-record, history-notes-are-append-only*

*Observed 2026-10-31 (architect-cto)*
