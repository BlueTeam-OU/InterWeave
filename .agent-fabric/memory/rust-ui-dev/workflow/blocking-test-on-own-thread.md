---
role: "rust-ui-dev"
class: workflow
topic: "blocking-test-on-own-thread"
description: "a test of a timeout/cancel bound runs the possibly-blocking call on a spawned thread and waits with recv_timeout, so its mutation fails instead of hanging the suite"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 769b29937d4fbc66
---

## a test of a timeout/cancel bound runs the possibly-blocking call on a spawned thread and waits with recv_timeout, so its mutation fails instead of hanging the suite

Twice in one batch (PR #483, 2026-10-27) a test of a deadline or cancel bound called the blocking
function on the test thread itself; the mutation it claimed to catch then HUNG the suite (a 900 s
subprocess timeout in my mutation runner, and round-4 review finding F7) instead of failing it.

**Why:** a bound's whole failure mode is "never returns", so a test that waits inline can only pass or
hang, and an assertion on elapsed time after the call is unreachable.

**How to apply:** for any test of a deadline, cancel or "ends within" claim, spawn the call, send its
result over a channel, and `recv_timeout(<a few times the bound>).expect(...)` on the test thread. Run
mutations with a per-run timeout, and look for leftover test processes afterwards
(`ps -u $(id -u) -o pid,args | grep deps/`). Related: [[mutation-restore-mtime-trap]].

*References: mutation-restore-mtime-trap*

*Observed 2026-10-27 (rust-ui-dev)*
