# security-local

Stage 18's local adversarial suite (plan section 21, steps S9-S11): the
daemon's data and admin sockets, transportctl and the stores, attacked
from the same host. It covers admin/data socket spoofing, a real foreign
uid, storage failure, and identity recovery failure and tamper.

The gate is [`../SECURITY-MATRIX.md`](../SECURITY-MATRIX.md). The
network half is [`../security`](../security).

- `tests/socket_spoofing.rs` (S9): a data-socket hello claiming
  `transportctl`, the daemon's modes under umask 000, an open run
  directory, and the real foreign uid of decided (4).
- `tests/source_and_deputy.rs` (S9, two daemons): a caller naming its
  own source endpoint, and admin requests arriving as messages.
- `tests/profile_state.rs` (S10, profile state): an other-writable
  ancestor or a link on the state directory's path, the trust and endpoint
  overlays tampered with or deleted, and an admin set whose overlay
  cannot be written, each through the shipped daemon.
- `tests/identity_recovery.rs` (S11): wrong and tampered restores through
  `transportctl`, the secret's files under umask 000, a running
  profile's key, and the scan of every stream a run writes.
- `src/secret.rs` is that scan, and `src/tap.rs` records the IPC bytes
  between a client and the daemon.

Two tests are `#[ignore]`d because they need a second account, named by
`INTERWEAVE_FOREIGN_UID_USER`, that `sudo -n -u` reaches without a
password; with the variable unset, or `sudo` refused, each fails by name.
CI runs each as `nobody`, in a run of its own.

- S9's foreign-uid test, which also needs the account to run `python3`:
  `cargo test -p interweave-security-local-tests --test socket_spoofing --
  --include-ignored`. Its scratch directory must be reachable by that
  account, so `TMPDIR` must not sit under a private home.
- TM-49's second-uid case, where that account makes a link in `/tmp`:
  `cargo test -p interweave-security-local-tests --test profile_state --
  --include-ignored`.
