# Review card — InterWeave (agent-fabric ADR-020; the only project text a reviewer gets)

## Run, from what is already fetched (no new deps)
- `cargo check --offline`, `cargo test --offline -p <crate>` (unit beside source; crate surface in the crate's `tests/`; cross-crate in root `tests/<suite>/`; desktop in `tests/desktop-e2e/`).
- `cargo xtask checks` runs the tree checks; three need the network (`check_dependencies.sh`, `check_spike_locks.sh`, `check_workflows_lint.sh`), and their exit 2 means "not evaluated", not a pass.
- Tool self-tests: `cargo xtask selftests` (one: `bash tools/checks/run_suite.sh tools/checks/test_<name>.sh`); fixture vectors: `tools/checks/verify_fixture_vectors.py` (a drifted vector under `fixtures/` is a protocol break).

## Contracts (shape and behaviour)
- The schemas in the family directories under `architecture/contracts/schemas/` are normative for shape, scoped by `x-contract.status` (`approved` is a target, never a claim); `tools/checks/validate_contracts.py` checks schema, manifest and provenance. The prose under `architecture/contracts/`, `transport/`, `discovery/`, `clients/` is normative for behaviour (ADR-0049).
- ADRs under `architecture/adr/` with `ADR-DIGEST.md` and the README row in the same range (`tools/checks/validate_adr_index.sh`).
- `third_party/` is vendored: a file not in `tools/checks/license_exempt.txt` is an unreviewed import; `spikes/` never vouches for first-party code.

## Invariants every review holds
- A lease or a lock is released on every path, timeout and signal included; waiters are bounded, and their answers follow ADR-0019's scope.
- No mock stands in for a real-network, process or platform requirement; `tests/support` is never a production dependency.
- A comment that claims an invariant owes a test; a stage's `Met.` block claims only what its tests show.
- A key replaced by a seal is retired after the new record is written, and a failed retire is said (`OldKeyRemains`, ADR-0042); relay use is never described as anonymity.
