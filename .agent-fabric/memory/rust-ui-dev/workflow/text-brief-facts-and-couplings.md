---
role: "rust-ui-dev"
class: workflow
topic: "text-brief-facts-and-couplings"
description: "Briefing language-culture for a person-facing text (release page, refusal screen): mark each fact decided or PROPOSAL, and name every coupling between the sentence and what a placeholder or another role's file renders"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 10717460cdc50ca2
---

## Briefing language-culture for a person-facing text (release page, refusal screen): mark each fact decided or PROPOSAL, and name every coupling between the sentence and what a placeholder or another role's file renders

A sentence a person reads describes things other roles produce: "the link and its SHA-256 are
below" depends on what the filler puts in `{crate_notices}`; "under the heading Licence texts"
depends on a tool's output heading; "the archive holds three programs" on a packaging proposal.
On the release-page work (2026-10-20/22) every rework came from one of those changing under the
text, and every catch came from having named the dependency.

**How to apply** in a brief to language-culture-en (or any text supplier):
- give the reader, their state and the space, and each fact with its status — decided (cite the
  record) or PROPOSAL from <role> until <change> lands — and promise to say if it moves;
- send person-facing promises only as decided facts (what the message does and does NOT say;
  "some", not "every", when the rule is conditional);
- when handing the finished text to the integrator, list the couplings it relies on (the
  placeholder renders X then Y; that file's heading must stay "..."), so a change on their side
  comes back as a question, not a silently false sentence;
- when a record says where a value appears (e.g. "in the page's hash list") and the template
  puts it elsewhere, raise it to the filler before it lands: page and record must agree.
Who finalises the words and how they land: [[labels-english-supplier]].

*References: labels-english-supplier*

*Anchors: `apps/human-android/release/release-page.md.in::{crate_notices_sha256}`, `tools/ci/release_notices.py::Licence texts`*

*Observed 2026-10-23 (rust-ui-dev)*
