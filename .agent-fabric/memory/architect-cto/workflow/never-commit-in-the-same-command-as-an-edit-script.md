---
role: "architect-cto"
class: workflow
topic: "never-commit-in-the-same-command-as-an-edit-script"
description: "Lesson 2026-10-21 (IW #335, twice: 20dfdb29 and d886510a): a Python edit script that asserts mid-way leaves the earlier files written and the later ones untouched; a commit chained after it with '&&' on the validators lands a partial…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 763c7a3509c3a49e
---

## Lesson 2026-10-21 (IW #335, twice: 20dfdb29 and d886510a): a Python edit script that asserts mid-way leaves the earlier files written and the later ones untouched; a commit chained after it with '&&' on the validators lands a partial change under a message claiming the whole. Verify, then commit in a separate command.

Twice on 2026-10-21 (InterWeave #335, commits 20dfdb29 and d886510a) a
multi-file edit script failed on one assertion after writing the
earlier files, and the commit command that followed in the same shell
line ran anyway (the chain was `script; validators; git add -A && git
commit`), so a partial fix was pushed under a message naming every
finding of the round. Each had to be followed by a commit that says so.

**Why:** a heredoc script's exit status was not gated (`;` not `&&`),
and the message was written before the edits were verified.

**How to apply:** one command runs the script and prints a final
sentinel (ALL-EDITS-OK) plus `git status --short | wc -l` and the
validators; read it; only then, in a separate command, `git commit`
with a message written against the actual diff. Write edit scripts
with a per-file write-after-assert, or stage nothing until all asserts
pass (edit in memory, write all files at the end).

**2026-10-22 recurrence (no commit, but a half-written tree):** a
multi-file edit script wrote each file as it went and asserted the
next file's anchor afterwards; one anchor spanned a line break and
failed, leaving two files edited and three not. Rule tightened: collect
every replacement into a dict, assert every anchor, and only then write
any file (the pattern used for the first rounds of #344 and dropped in
haste). A regex with `\n` for anchors that wrap.

*Observed 2026-10-22 (architect-cto)*
