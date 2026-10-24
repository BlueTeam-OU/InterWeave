---
role: "devex-tooling"
class: workflow
topic: "shell-reader-allow-list-not-parser"
description: a guard reading what a shell script runs or what a workflow YAML calls — read a declared subset and refuse the rest, never chase forms with regexes
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0ecd4d761cf03917
---

## a guard reading what a shell script runs or what a workflow YAML calls — read a declared subset and refuse the rest, never chase forms with regexes

When a check must read which scripts a shell script runs (agent-fabric
guards_wired.py rule 1's one hop, j186, 2026-10-20/21), a permissive
hand-written segmenter that excludes constructs one by one does not
converge: three blind-review rounds each found new ways printed text
became a command (a ; inside a string, heredoc in "$( )", the & of >&2,
a binder in a case arm, let/eval, nested (( ))).

**Why:** bash's grammar has more corners than a review cycle has
rounds; every exclusion list is incomplete, and the failure is a silent
false "wired".

**How to apply:** read only a subset: refuse the whole script (and say
so on stderr when it matters) on a heredoc, an unquoted backtick, or a
line shlex cannot read; tokenize with
shlex.shlex(posix=True, punctuation_chars=True, whitespace_split=True);
count only an interpreter's first operand or a path command word;
resolve a variable only when named nowhere but its own assignment.
Mutate every mechanism and judge each run by the suite's LAST line,
not a FAIL count: a mutation that hangs crashes the suite with no FAIL
lines. Never edit a worktree while tests/run.sh runs in it. Related:
[[mutation-runner-verify-backup]], [[grep-q-pipefail-silent-pass]].

**How it converged (2026-10-21, 21 review rounds, delivered at agent-fabric
7c4d7f30):** every fix that *cut* or *closed* something early (a comment
cut, a case closer) produced the next false wire; every fix that
*refused the script* (with the reason on stderr) held. When a reading
is ambiguous, refuse; never cut. Count a case closer on raw text
(what follows the last ;; is exactly esac), not on any lexer's tokens.
A continuation is an odd trailing backslash run after strip_comment,
joined with nothing between.

**The same in YAML (gzapp #1157, 2026-10-21):** check_job_scripts_trigger_ci
read ci.yml `uses:` lines and job blocks with line regexes; five blind
rounds each found a new form dropped silently (comment-only name, folded
run:, quoted uses:, a comment naming another filter, remote @ref call).
What converged: ONE reader shared by every item, a lenient by-meaning
counter (`calls_by_name`: value, quotes and @ref dropped, last segment)
that the strict reader must equal or exit 2, and the forms neither reads
named in the docstring's PINNED LIMITS up front. A fix that replaces a
cross-check must keep what the cross-check caught (round 3 F5).

*References: grep-q-pipefail-silent-pass, mutation-runner-verify-backup*

*Anchors: `agent-fabric:tools/fabric/github/guards_wired.py::_run_path`, `gzapp:tools/checks/check_job_scripts_trigger_ci.py::calls_by_name`*

*Observed 2026-10-20 (devex-tooling)*
