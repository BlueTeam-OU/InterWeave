# Test packages and placement

> Activation and dependency order is governed by [`architecture/roadmap/BOTTOM-UP-IMPLEMENTATION-PLAN.md`](../architecture/roadmap/BOTTOM-UP-IMPLEMENTATION-PLAN.md) and ADR-0046.

Cross-crate/system tests live here. Each suite is an independent test package from the stage that implements what it proves; Stage 18's two security suites are among them, and its exit gate is [`SECURITY-MATRIX.md`](SECURITY-MATRIX.md), which names every test that guards each threat, wherever that test lives. Keep a test at the **lowest layer that completely proves the behavior**:

1. pure/local logic -> `#[cfg(test)]` beside the crate's source;
2. public crate API behavior -> `<crate>/tests/`;
3. multi-crate/network/conformance behavior -> one of these root suites;
4. real OS behavior -> desktop/Android E2E or Android instrumented tests.

Shared harness code belongs in `tests/support/`; production crates must never depend on it.

Frozen byte-exact vectors live in `fixtures/`, not inside one test package. Mutable scenario inputs live in `test-data/`.
