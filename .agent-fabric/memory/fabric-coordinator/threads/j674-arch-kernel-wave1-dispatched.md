---
role: "fabric-coordinator"
class: threads
topic: "j674-arch-kernel-wave1-dispatched"
description: "j674 one-owner-per-mechanism plan (owner 2026-11-08): Wave 1 REQUESTs sent to python-dev-01..04, ADR-067 + policies/mechanisms.json still mine to write"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 76a31180454a5396
---

## j674 one-owner-per-mechanism plan (owner 2026-11-08): Wave 1 REQUESTs sent to python-dev-01..04, ADR-067 + policies/mechanisms.json still mine to write

The owner approved (2026-11-08) a plan from an external architecture review: each cross-cutting mechanism gets one owning module plus a checker on `tools/fabric/state_writes.py`'s pattern. Plan file: `~/.claude/plans/i-think-you-should-swift-frost.md`.

Verified at agent-fabric origin/main 27af8227: the review's "15 state-writer gaps" is stale (j577/j576 closed them; only j642 gateway.log and send.py's unlocked trim remain). Real: 14 modules with own killpg/start_new_session, four 250-commit cap constants, query.py:210 reads binding.json.

Wave 1 REQUESTs sent 2026-11-08 (REPLY-EXPECTED yes):
- W1-A python-dev-02 01a1bce2-0d6a: tools/fabric/process.py from provisioning/bounded.py, 14 sites, process_spawns.py (adopts #477's j602 siblings)
- W1-B python-dev-03 01a1bce2-7dbe: github/evidence.py PrEvidence, caps once in common.py
- W1-C python-dev-01 01a1bce2-7e70: typed context in runtime/identity.py (mine: their for/ supply), context_reads.py, j642 + send.py trim -> zero gaps. My j642 drops when their REPLY names a job.
- W1-D python-dev-04 01a1bce2-7fa7: characterisation tests of the work lifecycle; I owe the ADR-037 transition-table amendment and must send them its branch.

**Why:** the owner wants mechanisms, not only ADRs, to hold each invariant once.
**How to apply:** my next agent-fabric PR is ADR-067 + policies/mechanisms.json (entries "pending") + the ADR-037 amendment; as each Wave 1 PR merges, wire its checker into lint/pre-commit/CI/suite and mark the entry "enforced". Wave 2 (WorkItem, TransportMux) waits on W1-D's findings and W1-A. Related: [[never-list-work-as-contributor-patches]].

Replies 2026-11-08 20:40Z: W1-C taken as python-dev-01 j212 (supply for/user/w1c-context; j642 dropped to it), W1-D python-dev-04 j100 (test/j100-work-lifecycle, WAITS on my ADR-037 table branch), W1-B python-dev-03 j119. W1-A python-dev-02: no reply yet — chase at next session start. j674 raised to blocking, j655 drain paused behind it.

*References: never-list-work-as-contributor-patches*

*Observed 2026-11-08 (fabric-coordinator)*
