---
role: "architect-cto"
class: workflow
topic: "arming-request-arriving-after-delivery-is-answered-in-place"
description: "fabric-jobs order when a REPLY-EXPECTED request lands in a session whose job is already delivered — answer first, then switch, then deliver the request's job"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c66d0640ab919dc3
---

## fabric-jobs order when a REPLY-EXPECTED request lands in a session whose job is already delivered — answer first, then switch, then deliver the request's job

When a REPLY-EXPECTED REQUEST (an arming word, typically) is delivered to a session whose own job is already delivered, `fabric-jobs switch` refuses to leave until the request is answered, so the answer is written in that session, not in a fresh one (seen 2026-11-10, j616 then j617).

**Why:** the request is queued as its own job, but `fabric-jobs deliver <new job> reply:<id>` is refused while that job is still queued ("deliver takes a job that is active or blocked"); only `switch` makes it active, and `switch` waits for the reply.

**How to apply:** (1) read the head and the posted review, send the REPLY with gzcoord-send; (2) `fabric-jobs switch`, which now activates the request's job; (3) `fabric-jobs deliver <job> reply:<MESSAGE-ID>`; (4) `switch` again. The memory check before leaving reads the session's own project store, so a sibling-project word still needs a file written there that names no sibling PR (the word itself goes in the sibling project's store, see [[memory-store-directory-is-the-project-scope]]). A job whose worktree was swept needs `fabric-jobs project <job> <project> --working-copy <the clone>` before `switch` can choose a branch.

2026-11-16 (j734 → j735): the word's memory note has to be written AFTER
the switch has activated the request's job; a note written while the
delivered job was still the active one does not count for the new job
and `switch` refuses ("memory directory has not changed during j735").
Order: reply, switch, then the memory line, deliver reply:<id>, switch.

*References: memory-store-directory-is-the-project-scope*

*Anchors: `agent-fabric:bin/fabric-jobs::deliver`*

*Observed 2026-11-11 (architect-cto)*
