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
- `tests/identity_recovery.rs` (S11): wrong and tampered restores through
  `transportctl`, the secret's files under umask 000, a running
  profile's key, and the scan of every stream a run writes.
- `src/secret.rs` is that scan, and `src/tap.rs` records the IPC bytes
  between a client and the daemon.

The foreign-uid test is `#[ignore]`d: it needs a second account, named by
`INTERWEAVE_FOREIGN_UID_USER`, that `sudo -n -u` reaches without a
password and that can run `python3`. Run it with
`cargo test -p interweave-security-local-tests --test socket_spoofing --
--include-ignored`; with the variable unset, or `sudo` refused, it fails
by name. Its scratch directory must be reachable by that account, so
`TMPDIR` must not sit under a private home.
