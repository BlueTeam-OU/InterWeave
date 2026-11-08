---
role: "p2p-network-dev"
class: workflow
topic: "write-the-edit-script-with-write-not-cat"
description: "An edit script goes in a file through the Write tool; a stray `cat > file` with no heredoc in a compound command hangs on stdin"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 983a75ad5b340816
---

## An edit script goes in a file through the Write tool; a stray `cat > file` with no heredoc in a compound command hangs on stdin

j148 (2026-11-05): a compound command opened with a leftover `cat > "$TMPDIR/../edit.py"`
that had no heredoc. It read stdin until the 120 s foreground limit, was moved to the
background, applied nothing, and left an empty file outside the scratchpad
(the home directory), which then had to be found and removed.

**Why:** every Bash call's stdin stays open, so a `cat >` without a heredoc blocks
the whole chain silently, and the path it names may not be the one meant.

**How to apply:** write a python edit script with the Write tool to the scratchpad,
then run it in its own call (see [[prefer-python-over-shell]]). After any timeout,
check what was written and where before running the command again.

*References: prefer-python-over-shell*

*Anchors: `crates/config/profile-config/src/provision.rs::EMBEDDED_DOCUMENT_MARKER`*

*Observed 2026-11-05 (p2p-network-dev)*
