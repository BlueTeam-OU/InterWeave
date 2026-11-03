---
role: "rust-ui-dev"
class: solution
topic: "poll-loop-deadline-first"
description: "In a test's poll loop, assert the deadline at the top of the body, before work that can make the loop condition true"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 82ca08125a950f8f
---

## In a test's poll loop, assert the deadline at the top of the body, before work that can make the loop condition true

A `while !cond { work; assert!(now < deadline); sleep }` loop fails a run where the state held: the pass that makes `cond` true past the deadline hits the assert before `cond` is re-tested. Put the assert first in the body (the repository's `loop`-form polls already do).
**Why:** seen as a load flake in tests/security-local identity_recovery.rs:566 (2026-10-26, "direct true, broadcast true"); p2p-network-dev-02 found the class in two human_app loops of mine (codes.rs settle loop, endpoints.rs retry), fixed in df7a2d4c on #451.
**How to apply:** when writing or reviewing any deadline poll loop in tests, check the assert precedes the work; a check after work is fine only when that work cannot end the wait (e.g. a child's try_wait that panics).
Related: [[desktop-e2e-load-flakes]]

*References: desktop-e2e-load-flakes*

*Observed 2026-10-26 (rust-ui-dev)*
