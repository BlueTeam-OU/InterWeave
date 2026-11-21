---
role: "language-culture"
class: workflow
topic: "superseded-supply-branch-deleted"
description: "a for/ branch that was never folded but whose content main replaced is deleted only on the owner's yes, after a check of main by content"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 32bf70b1a1ae0e00
---

## a for/ branch that was never folded but whose content main replaced is deleted only on the owner's yes, after a check of main by content

A `for/` branch that was never folded can still be superseded: main can hold a later rewrite of the same text, or a change that removed the text. `fabric-branches --sweep` does not delete such a branch, because its commits are not on main. Before you ask the owner to delete it, check main by content, not by topic. Run `git show origin/main:<file>` for the sections the branch wrote. Run `git grep <key> origin/main` for a key the branch named, and expect no match when main removed the key. Also confirm that no PR heads the branch (`gh pr list --head <branch> --state all`). Delete it with `git push origin --delete` only after the owner says yes. Then check with `git ls-remote`.

Seen in j304 (2026-11-13): `for/rust-ui-dev-02/j94-release-page-all-platforms` (76c196886) was superseded by later rewrites of `release-page.md.in`. `for/ux-designer-01/add-a-peer-contact-names` (e1392ddf4) was superseded by 83f52cfeb, which removed PeerContactNames. Related: [[folded-supply-branch-kept]].

*References: folded-supply-branch-kept*

*Anchors: `agent-fabric:tools/fabric/branches.py::pull request: {pr or 'none'}`*

*Observed 2026-11-13 (language-culture)*
