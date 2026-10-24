---
role: "fabric-coordinator"
class: workflow
topic: "withdrawn-request-check-the-thread-first"
description: before starting a job from a REQUEST, read its whole thread and the state of what it names (the PR) — a withdrawal or a merge can sit there unread, and the work already written for its siblings must come out too
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 82e08209cbf6dad4
---

## before starting a job from a REQUEST, read its whole thread and the state of what it names (the PR) — a withdrawal or a merge can sit there unread, and the work already written for its siblings must come out too

A job queued from a REQUEST can be withdrawn before it starts: j345 (second login for InterWeave j41's install run) was withdrawn by architect-cto-01's REPLY 01a1630a at 09:56Z on 2026-10-22, and the job was still started at 14:20Z. `fabric-jobs show <id>` prints the thread's episodes; read them to the end before any work.

A withdrawal also strands what sibling requests already produced: j346's auto-mode entry (0774b0af) sat on the open agent-fabric #354 and was reverted there (8a4d0fcf), and the dependent cleanup job j347 was dropped.

A request can also be overtaken without a word: j310 (the owner's word to arm InterWeave #317, 1 work 4 fix) was restarted 2026-10-23, but #317 had grown to 3 work commits and merged 2026-10-20T17:01Z, armed by the owner. `gh pr view <n> --json state` first.

A time-bound request goes stale in minutes: j339 (rust-ui-dev-01 asking this account to free the pixel-9a lease, 2026-10-21T19:38Z) was withdrawn by their INFO 01a15ffc four minutes later, the lease having freed itself at 19:39:41Z; the job was still started 2026-10-24. Read `fabric-lease <name> --who` and the thread, close with `fabric-jobs done <id> --no-artifact "<why>"`, and send one closing REPLY since the original carried `REPLY-EXPECTED: yes`. What held the lease was unrecoverable: no account state records a lease holder's command once its pid is gone.

**Why:** a withdrawn request's artifacts otherwise land as live policy nobody needs.

**How to apply:** on taking any request job, run `fabric-jobs show` and `fabric-history --thread <id>`; on a withdrawal, drop the job and grep open PRs for what its siblings (same thread or the requester's related REQUESTs) put there. Reverting on an open PR at the sixteen-work-commit ceiling is the owner's call: 2026-10-22 they chose to push the 17th rather than force-push or defer.

*Anchors: `agent-fabric:policies/auto-mode.json::_comment`*

*Observed 2026-10-24 (fabric-coordinator)*
