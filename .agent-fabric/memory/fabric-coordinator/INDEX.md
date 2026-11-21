---
role: "fabric-coordinator"
class: index
description: "What fabric-coordinator knows and where it lives."
tier: 1
distilled_at: "2026-11-17"
---

# fabric-coordinator — knowledge index

A session is given the charter and brief (in the launch prompt),
and the project's remit with a pointer to this index (from the
session-start hook) — nothing below. Open a slice when its cue
matches what you are doing; a `workflow` slice says how a kind of
work is done here, so read the matching ones before that work.
Paths are relative to this working copy; `../agent-fabric/` is the
control plane checked out beside it.

## charter

- [`../agent-fabric/identities/roles/fabric-coordinator/charter.md`](../agent-fabric/identities/roles/fabric-coordinator/charter.md) — Owns the control plane: agent-fabric's role definitions, catalogue, routing policy, authority rules and the GZCoord protocol; the only role that changes what other roles are.

## brief

- [`../agent-fabric/identities/roles/fabric-coordinator/brief.md`](../agent-fabric/identities/roles/fabric-coordinator/brief.md) — How fabric-coordinator works day to day: the control plane's only writer — roles, routing, policy, the protocol, the launch — changed in small verified commits, distributed to every account, never project truth.

## solution

- [`.agent-fabric/memory/fabric-coordinator/solution/deck-select-modifier-on-qubes.md`](.agent-fabric/memory/fabric-coordinator/solution/deck-select-modifier-on-qubes.md) — FleetDeck alt+drag selection fails on Qubes because dom0 Xfce grabs alt+drag to move windows; super cannot replace it because a terminal's SGR mouse report carries only shift/alt/ctrl
- [`.agent-fabric/memory/fabric-coordinator/solution/drain-merge-target-is-an-exact-heading.md`](.agent-fabric/memory/fabric-coordinator/solution/drain-merge-target-is-an-exact-heading.md) — a merge_target that paraphrases or names a topic slug matches nothing and is filed as a new topic, silently except for merge_target_unresolved; counts.reflection needs origin exactly "reflection"
- [`.agent-fabric/memory/fabric-coordinator/solution/solution-carried-2026-10-24.md`](.agent-fabric/memory/fabric-coordinator/solution/solution-carried-2026-10-24.md) — why interweave-human send/listen cannot carry GZCoord between agents, and what ADR-064 chose instead

## workflow

- [`.agent-fabric/memory/fabric-coordinator/workflow/a40-stay-awake-is-owners-setting.md`](.agent-fabric/memory/fabric-coordinator/workflow/a40-stay-awake-is-owners-setting.md) — Both test phones' (A40, Pixel 9a) stay_on_while_plugged_in=7 is the owner's own setting (cable + stay-awake); a run must never reset it to 0
- [`.agent-fabric/memory/fabric-coordinator/workflow/arm-defer-carried-findings.md`](.agent-fabric/memory/fabric-coordinator/workflow/arm-defer-carried-findings.md) — fabric-pr arm refuses a PR whose review chain holds findings moved to other jobs until each is passed as --defer "<chain head8>:F<n>" "<why>"
- [`.agent-fabric/memory/fabric-coordinator/workflow/broadcast-quotes-the-tool-not-memory.md`](.agent-fabric/memory/fabric-coordinator/workflow/broadcast-quotes-the-tool-not-memory.md) — before a broadcast tells every agent what a tool keeps or deletes, read the tool's --help and quote it; a paraphrase from memory went to 30 sessions wrong
- [`.agent-fabric/memory/fabric-coordinator/workflow/drain-cross-corpus-slices-both-ways.md`](.agent-fabric/memory/fabric-coordinator/workflow/drain-cross-corpus-slices-both-ways.md) — a project-half drain carries other repositories' slices in BOTH directions; a slice held from one half may already be superseded in the other
- [`.agent-fabric/memory/fabric-coordinator/workflow/drain-duplicate-heading-blocks-retire.md`](.agent-fabric/memory/fabric-coordinator/workflow/drain-duplicate-heading-blocks-retire.md) — a section heading standing in two files of one topic (a stale copy beside its -2 part) makes a retire claim's merge_target ambiguous; curate by removing the stale copy, then the retire resolves
- [`.agent-fabric/memory/fabric-coordinator/workflow/drain-hold-keys-are-claim-topics.md`](.agent-fabric/memory/fabric-coordinator/workflow/drain-hold-keys-are-claim-topics.md) — assemble.py --hold names a claim's own <role>/<class>:<topic>, not the slice a merge_target landed it in; two claims aimed at one section in one bundle lose one text silently
- [`.agent-fabric/memory/fabric-coordinator/workflow/drain-rerun-resets-the-fabric-tree.md`](.agent-fabric/memory/fabric-coordinator/workflow/drain-rerun-resets-the-fabric-tree.md) — a project-half drain re-run must reset the agent-fabric tree it runs from as well as the project's .agent-fabric/memory: assemble.py writes a bundle's domain claims into the fabric's memory/domains, and a second run then treats them as…
- [`.agent-fabric/memory/fabric-coordinator/workflow/drain-run-the-project-tree-checks-first.md`](.agent-fabric/memory/fabric-coordinator/workflow/drain-run-the-project-tree-checks-first.md) — before committing an InterWeave memory drain, run the project's licence-header check on the result; lint.py does not see what the project's CI refuses
- [`.agent-fabric/memory/fabric-coordinator/workflow/fabric-ref-bump-rides-the-consumer.md`](.agent-fabric/memory/fabric-coordinator/workflow/fabric-ref-bump-rides-the-consumer.md) — How an InterWeave .agent-fabric/fabric-ref bump is delivered and checked when another role's PR needs it
- [`.agent-fabric/memory/fabric-coordinator/workflow/head-pinned-word-leaves-local-fold.md`](.agent-fabric/memory/fabric-coordinator/workflow/head-pinned-word-leaves-local-fold.md) — When the owner's arming word (or an AUTHORIZES) names a head, arm that head and leave any later local fold of main unpushed
- [`.agent-fabric/memory/fabric-coordinator/workflow/never-list-work-as-contributor-patches.md`](.agent-fabric/memory/fabric-coordinator/workflow/never-list-work-as-contributor-patches.md) — When python-devs are idle and the only Python work is on CONTRIBUTOR_NEVER, they write it as a patch the coordinator commits
- [`.agent-fabric/memory/fabric-coordinator/workflow/oversized-claim-is-the-authors-split.md`](.agent-fabric/memory/fabric-coordinator/workflow/oversized-claim-is-the-authors-split.md) — a slice over the 1800-token budget that is ONE section is an atomic claim the assembler cannot split — ask its origin agent to split the memory; lint fails only past 1800 x 1.35
- [`.agent-fabric/memory/fabric-coordinator/workflow/per-job-auto-mode-entry-procedure.md`](.agent-fabric/memory/fabric-coordinator/workflow/per-job-auto-mode-entry-procedure.md) — How a per-job auto-mode allow entry (policies/auto-mode.json) is landed and what bit last time — owner's choice, own PR, revert elsewhere, review against the driver
- [`.agent-fabric/memory/fabric-coordinator/workflow/registry-request-mis-homed-to-project.md`](.agent-fabric/memory/fabric-coordinator/workflow/registry-request-mis-homed-to-project.md) — a REQUEST about projects/registry.json can arrive as a job in the PROJECT's worktree; the job guard then refuses the agent-fabric commit
- [`.agent-fabric/memory/fabric-coordinator/workflow/review-report-arrives-as-message.md`](.agent-fabric/memory/fabric-coordinator/workflow/review-report-arrives-as-message.md) — a code-review subagent's report comes back as a hand-back message, not in its task output file; post-review needs it copied from there
- [`.agent-fabric/memory/fabric-coordinator/workflow/search-own-jobs-before-a-record.md`](.agent-fabric/memory/fabric-coordinator/workflow/search-own-jobs-before-a-record.md) — before drafting a record on a subject, search your own job list and memories for an older job on it — it may carry the owner's word
- [`.agent-fabric/memory/fabric-coordinator/workflow/split-request-silence-check-repoints.md`](.agent-fabric/memory/fabric-coordinator/workflow/split-request-silence-check-repoints.md) — A job blocked on several split REQUESTs shows only the answered ones as repoints; the unanswered one is silent — check each id before waiting on the drain
- [`.agent-fabric/memory/fabric-coordinator/workflow/tree-checks-red-can-be-a-runner-timeout.md`](.agent-fabric/memory/fabric-coordinator/workflow/tree-checks-red-can-be-a-runner-timeout.md) — an InterWeave required check reported "fail" by wait-merged or gh pr checks may be a 15-minute job timeout (conclusion cancelled), not a finding; re-run it, the arming survives
- [`.agent-fabric/memory/fabric-coordinator/workflow/withdrawn-request-check-the-thread-first.md`](.agent-fabric/memory/fabric-coordinator/workflow/withdrawn-request-check-the-thread-first.md) — before starting a job from a REQUEST, read its whole thread and the state of what it names (the PR) — a withdrawal or a merge can sit there unread, and the work already written for its siblings must come out too

## threads

- [`.agent-fabric/memory/fabric-coordinator/threads/adr-0067-roster-endpoint-ids-plan.md`](.agent-fabric/memory/fabric-coordinator/threads/adr-0067-roster-endpoint-ids-plan.md) — InterWeave ADR-0067 (derived EndpointIds) lands in the fabric as an ADR-064 amendment, built only when Stage 22 opens (j458)
- [`.agent-fabric/memory/fabric-coordinator/threads/j674-arch-kernel-wave1-dispatched.md`](.agent-fabric/memory/fabric-coordinator/threads/j674-arch-kernel-wave1-dispatched.md) — j674 one-owner-per-mechanism plan (owner 2026-11-08): Wave 1 REQUESTs sent to python-dev-01..04, ADR-067 + policies/mechanisms.json still mine to write

## recall

- [`../agent-fabric/identities/roles/fabric-coordinator/recall.md`](../agent-fabric/identities/roles/fabric-coordinator/recall.md) — How to ask the control plane about itself: status, routing, lint, migration records.
