# Review card — InterWeave (agent-fabric ADR-020; the only project text a reviewer gets)

## Run, offline, from what is already fetched (no network, no new deps)
- `cargo check`, `cargo test -p <crate>` (unit beside source; crate surface in `<crate>/tests/`; cross-crate in root `tests/<suite>/`; desktop in `tests/desktop-e2e/`); `cargo xtask checks` runs the tree checks; `cargo deny check licenses` for a dependency change.
- tool self-tests: `tools/checks/run_suite.sh`; fixture vectors: `tools/checks/verify_fixture_vectors.py` (a drifted vector under `fixtures/` is a protocol break).

## Contracts (shape and behaviour)
- `architecture/contracts/schemas/*.json` are normative for shape, scoped by `x-contract.status` (`approved` is a target, never a claim); the prose under `architecture/contracts/`, `architecture/transport/`, `architecture/discovery/`, `architecture/clients/` is normative for behaviour (ADR-0049). A status flip without its manifest entry is a finding.
- ADRs under `architecture/adr/` with `ADR-DIGEST.md` and the README row propagated in the same range (`tools/checks/validate_adr_index.sh`).
- `third_party/` is vendored and compiled in: an unlisted file in `tools/checks/license_exempt.txt` is an unreviewed import; `spikes/` never vouches for first-party code.

## Invariants every review holds
- A waiter is answered, never dropped; a lease or a lock is released on every path, timeout and signal included.
- No mock stands in for a real-network, process or platform requirement; `tests/support` is never a production dependency.
- A comment that claims an invariant owes a test; a stage's `Met.` block claims only what its tests show.
- Privacy regimes and custody: a sealed or retired key is never still usable; nothing stripped is called anonymous.
