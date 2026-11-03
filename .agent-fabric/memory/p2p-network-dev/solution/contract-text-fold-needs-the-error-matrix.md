---
role: "p2p-network-dev"
class: solution
topic: "contract-text-fold-needs-the-error-matrix"
description: "Folding contract prose that names an error code into a PR turns `rust` red: tests/transport-contract error_contract_matrix demands a row or PROSE entry per code occurrence in contracts/ENDPOINTS.md and transport/libp2p/DIRECT.md"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d690e726878017a0
---

## Folding contract prose that names an error code into a PR turns `rust` red: tests/transport-contract error_contract_matrix demands a row or PROSE entry per code occurrence in contracts/ENDPOINTS.md and transport/libp2p/DIRECT.md

On #460 (2026-10-26), architect-cto supplied contract text (a2a905b1, e3ba33cb) saying a zero-byte response stream is `PeerUnreachable`. The supplier's own checks passed (docs integrity, validate_contracts), and so did the review rounds. GitHub's `rust` job still failed: `the_contracts_name_no_error_this_matrix_has_missed` in `tests/transport-contract/tests/error_contract_matrix.rs` counts every occurrence of an error code, per line and per code, in `architecture/contracts/ENDPOINTS.md` and `architecture/transport/libp2p/DIRECT.md`.

Each positive mapping needs a `Clause` row: a verbatim quote, the code, and `Proof::Test(<a #[test] whose body names the code>)`. Each mention of a code that a condition is NOT ("never `ProtocolViolation`", "rather than its own `Timeout`") needs a `PROSE` entry. Fixed in 8e38f17a.

**Why:** the matrix runs only in the Rust test suite, not in the tree checks, so neither a docs-only supplier nor a prose review sees it.

**How to apply:** after folding or writing any line in those two documents that names an error code, run `cargo test -p interweave-transport-contract-tests --test error_contract_matrix` before pushing. See also [[supply-commits-to-the-callers-change]].

*References: supply-commits-to-the-callers-change*

*Anchors: `tests/transport-contract/tests/error_contract_matrix.rs::the_contracts_name_no_error_this_matrix_has_missed`*

*Observed 2026-10-26 (p2p-network-dev)*
