---
role: "p2p-network-dev"
class: workflow
topic: "xtask-ci-outlives-the-background-limit"
description: cargo xtask ci takes over two hours on this host and a backgrounded Bash call is killed at 7200 s; run it detached (setsid nohup, log to a file, a ci=$? line) and wait on that line
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 37c453857da4d5f2
---

## cargo xtask ci takes over two hours on this host and a backgrounded Bash call is killed at 7200 s; run it detached (setsid nohup, log to a file, a ci=$? line) and wait on that line

On 2026-10-19 (#305) `cargo xtask ci` was started as a backgrounded Bash call with the maximum timeout (7,200,000 ms). It was killed during the last self-test (`tools/ci/test_with_display.sh`), so the run never printed a summary. The full run is longer than the tool's background limit.

What worked for the rerun:

- Start it detached, with `setsid nohup bash -c "cargo xtask ci >> LOG 2>&1; echo ci=\$? >> LOG" &`.
- Wait with a separate background loop: `until grep -q '^ci=' LOG; do sleep 60; done`.
- Put a `HEAD <sha>` line at the top of the log first, so the result names what it tested.
- Keep the log in the session scratchpad, not in `projects/`.

Under that full load `test_with_display`'s HUP case also timed out at 31 s, and it passed alone; that is the same class as [[stage6-ingress-burst-test-is-load-timed]].

*References: stage6-ingress-burst-test-is-load-timed*

*Anchors: `tools/ci/test_with_display.sh`, `xtask/src/main.rs::ci`*

*Observed 2026-10-19 (p2p-network-dev)*
