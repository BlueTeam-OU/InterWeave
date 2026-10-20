<!-- SPDX-License-Identifier: Apache-2.0 -->
# Host run: the Linux package under a systemd user manager, 2026-10-20

The evidence for plan §22's Validate rows **service/autostart lifecycle**
and **uninstall semantics** on Linux, and for the SIGHUP carry's "the
unit row shows the restart". It was recorded here because CI had no
user manager then, and the row carried the `limit` "no user manager in
CI" (architect-cto's ruling of 2026-10-20) until devex-tooling's steps
in the `rust` job (4f1ca850, folded into #324) ran `user_units.rs`
under the hosted runner's lingering user manager: from then CI is the
record, and this run is the first.

- Commit: `d7f05382500642e8eeba542afd2b6e533ed9b517`
- Host: Fedora, `systemd 258 (258.11-1.fc43)`, the account's own user
  manager, lingering on
- Command: `cargo test -p interweave-desktop-e2e-tests --test user_units
  -- --ignored --test-threads 1`, with the three workspace binaries built
  beside the tests
- What the cases install: `packaging/linux/install.sh` into a scratch
  `HOME` and `XDG_CONFIG_HOME`; the installed templates linked into the
  manager's runtime unit directory, with a per-instance drop-in naming the
  installed binary and the scratch XDG tree

## Result

```text
test the_installed_daemon_unit_starts_restarts_on_sighup_and_stops ... ok
test uninstall_stops_every_instance_removes_the_package_and_keeps_the_profile ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 35.60s
```

## The manager's journal for the run's instance

Both cases share the instance `e2e-units-848328`: the lifecycle case
first, then the uninstall case. The SIGHUP stop leaves a scheduled
restart and no `Failed` line. The SIGKILL control leaves `Failed with
result 'signal'`. The `Failed to open` lines come after the uninstall
removed the template that the runtime link points to. The PIDs and the
uid are redacted.

```text
Started interweave-daemon@e2e-units-848328.service - InterWeave transport daemon, profile e2e-units-848328.
interweave-daemon@e2e-units-848328.service: Sent signal SIGHUP to main process <pid> (transport-daemo) on client request.
interweave-daemon@e2e-units-848328.service: Scheduled restart job, restart counter is at 1.
Started interweave-daemon@e2e-units-848328.service - InterWeave transport daemon, profile e2e-units-848328.
interweave-daemon@e2e-units-848328.service: Sent signal SIGKILL to main process <pid> (transport-daemo) on client request.
interweave-daemon@e2e-units-848328.service: Main process exited, code=killed, status=9/KILL
interweave-daemon@e2e-units-848328.service: Failed with result 'signal'.
interweave-daemon@e2e-units-848328.service: Scheduled restart job, restart counter is at 2.
Started interweave-daemon@e2e-units-848328.service - InterWeave transport daemon, profile e2e-units-848328.
Stopped interweave-daemon@e2e-units-848328.service - InterWeave transport daemon, profile e2e-units-848328.
Started interweave-daemon@e2e-units-848328.service - InterWeave transport daemon, profile e2e-units-848328.
Started interweave-client@e2e-units-848328.service - InterWeave automated client, profile e2e-units-848328.
Stopped interweave-client@e2e-units-848328.service - InterWeave automated client, profile e2e-units-848328.
Stopped interweave-daemon@e2e-units-848328.service - InterWeave transport daemon, profile e2e-units-848328.
Started interweave-daemon@e2e-units-848328.service - InterWeave transport daemon, profile e2e-units-848328.
Started interweave-client@e2e-units-848328.service - InterWeave automated client, profile e2e-units-848328.
Stopped interweave-client@e2e-units-848328.service - InterWeave automated client, profile e2e-units-848328.
Stopped interweave-daemon@e2e-units-848328.service - InterWeave transport daemon, profile e2e-units-848328.
interweave-daemon@e2e-units-848328.service: Failed to open /run/user/<uid>/systemd/user/interweave-daemon@.service: No such file or directory
interweave-daemon@e2e-units-848328.service: Failed to open /run/user/<uid>/systemd/user/interweave-daemon@.service: No such file or directory
interweave-client@e2e-units-848328.service: Failed to open /run/user/<uid>/systemd/user/interweave-client@.service: No such file or directory
interweave-client@e2e-units-848328.service: Failed to open /run/user/<uid>/systemd/user/interweave-client@.service: No such file or directory
Started interweave-daemon@e2e-units-848328.service - InterWeave transport daemon, profile e2e-units-848328.
Stopped interweave-daemon@e2e-units-848328.service - InterWeave transport daemon, profile e2e-units-848328.
```

## What this run does not show

- The restart at login or boot that `enable` arranges: the case checks
  the `default.target.wants` link, and does not log in again.
- A persistent `enable`: the cases enable with `--runtime`, so a link
  never lands in the account's own `~/.config`.
- Any host but this one, and any systemd but 258.
