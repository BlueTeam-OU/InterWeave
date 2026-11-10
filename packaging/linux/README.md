# linux

Linux desktop/server packaging, service integration and permissions.

## systemd user units

Two templates, each instantiated on a profile name (plan §28 step 4,
ADR-0055 decision 6):

| unit | runs | after a failure |
|---|---|---|
| `interweave-daemon@<profile>.service` | `~/.local/bin/transport-daemon --profile <profile>` | restarted 5 s later; exit 2 (a usage error) is not restarted; exit 75 (a stop asked by SIGHUP) is restarted 5 s later and logged as a success |
| `interweave-client@<profile>.service` | `~/.local/libexec/interweave/client --profile <profile>` | restarted 5 s later |

Both are **user** units: they run as the account that installs them, from
that account's own binaries, with nothing installed system-wide. A host
with many profiles under many accounts runs one daemon unit per account.

The daemon is stopped with `SIGTERM`, which it answers by stopping within
its own shutdown grace. `SIGHUP` is a restart, not a reload: the daemon
stops exactly as on `SIGTERM` and its unit starts it again `RestartSec=`
(5 s) later. Nothing survives a restart -- every client's endpoint lease
ends and the new daemon issues fresh epochs -- and there is no
`ExecReload=`: a changed `config.yaml` is applied by `systemctl --user
restart interweave-daemon@<profile>`. A daemon whose listen port another
account holds fails its start and its unit retries it. Give each profile
its own port: two daemons of one account on one port both start, because
the TCP transport listens with `SO_REUSEPORT` (ADR-0055 decision 5).

The client unit starts after its daemon and is stopped whenever its daemon
stops (`After=`, `BindsTo=`). Enabling it also makes it start whenever its
daemon starts, its daemon's own restarts included (`WantedBy=` the daemon
instance). **InterWeave ships no automated client.** The program is the
deployment's: install it at `~/.local/libexec/interweave/client`, or keep it
elsewhere and override `ExecStart=` in a drop-in
(`systemctl --user edit interweave-client@<profile>`). What the transport
promises such a client, and what the client owes itself, is
`architecture/clients/automated/`.

## Installing and uninstalling

`install.sh` and `uninstall.sh` are run as the account that will run the
profile, need no root and change nothing system-wide; they write only that
account's own files: the four programs
(`transport-daemon`, `transportctl`, `human-desktop` and the command-line
human client `interweave-human`) into `~/.local/bin`,
the two templates into `${XDG_CONFIG_HOME:-~/.config}/systemd/user`
(outside the home when `XDG_CONFIG_HOME` is), and on uninstall the
removal of InterWeave's own unit links, persistent under that directory
and runtime under the account's `$XDG_RUNTIME_DIR/systemd/user` (normally
`/run/user/<uid>/systemd/user`).
From the release archive, whose `bin/` and `systemd/` sit beside the
scripts:

```sh
./install.sh                                          # or ./install.sh <dir holding the four programs>
transportctl identity init --profile <profile>        # once: mints the key, prints the PeerId
# write the profile's config.yaml (see below), then, to run it at login:
systemctl --user enable --now interweave-daemon@<profile>
systemctl --user enable --now interweave-client@<profile>   # if this profile has an automated client
```

**Installing enables nothing.** A profile's daemon starting with the
account's user manager -- at login, or at boot with lingering -- is the
person's explicit choice, made by `enable` above and undone by `disable`.
An upgrade is the same `install.sh` over the old one: each file is
replaced by a rename, so a running daemon keeps the binary it started
with until it is restarted.

`./uninstall.sh` stops and disables every `interweave-daemon@` and
`interweave-client@` instance of the account, then removes the two
templates and the four programs. Run it from the account's own
session: from a shell without the session's variables, or with another
account's (`sudo -u`, `su -c`, `sudo -E`, cron), it finds the manager
under the account's own `/run/user/<uid>`, and if a running manager
still cannot be reached it removes nothing and fails, since it could not
stop what runs. **It never removes a profile**:
every profile's `config.yaml`, identity key, state, human store, logs
and peer cache stay where they are, and a reinstall serves the same PeerId from the kept
key. A removed key is a lost PeerId, recoverable only from its recovery
mnemonic, so deleting a profile is your own act, by hand. A profile lives
in four directories, each `interweave/profiles/<profile>` under its XDG
root: the configuration under `$XDG_CONFIG_HOME` (`~/.config`), the key
under `$XDG_DATA_HOME` (`~/.local/share`), the state and the human
client's store under `$XDG_STATE_HOME` (`~/.local/state`), and the peer
cache under `$XDG_CACHE_HOME` (`~/.cache`).

Without `loginctl enable-linger <account>`, the account's units stop when
its last login session ends; a profile that must stay up enables lingering
(whether an account may do this for itself is the host's polkit policy).

**Never run one profile on two hosts at once**, as a home shared over NFS
would allow. The daemon's profile lock is `flock(2)`, which Linux carries
over NFS as a byte-range lock granted by the server's lock manager, so it
excludes a second host only when the server grants locks; a home mounted
`nolock` or `local_lock=flock` makes the lock host-local, and two hosts
would both serve one PeerId.

## The release archive

A release ships these files as
`interweave-<version>-x86_64-unknown-linux-gnu.tar.gz`, built by
`tools/ci/release_linux_archive.sh`: one directory holding `bin/` (the
four programs), `systemd/` (the two templates), `install.sh`,
`uninstall.sh`, this README, `LICENSE`, `THIRD-PARTY-NOTICES.md`, the
notice of every crate compiled into the programs and the Claude Code
bridge -- the licence files it publishes, or for a crate that publishes none
a notice derived as `tools/ci/notice_derived.json` lists it (ADR-0058); a
crate with neither fails the archive -- and of the C libraries a crate's
build compiles in from a build-dependency's sources (SQLCipher, OpenSSL),
by the notice files `tools/ci/notice_sources.json` names; a build-dependency
that list does not classify fails the archive too -- and `claude-plugin/`, the Claude Code plugin marketplace with the
bridge inside it. `install.sh` does not install the plugin; Claude Code adds
it, from a copy of `claude-plugin/` kept where it stays, with
`claude plugin marketplace add <that copy>` and then
`claude plugin install interweave@interweave`. Installed, the plugin
receives nothing yet: without the development flag, a Claude Code session
registers its channel only when started with
`--channels plugin:interweave@interweave` (or, non-interactively, sent the
`channel_enable` control request) on a host whose managed settings list
`{"plugin": "interweave", "marketplace": "interweave"}` in
`allowedChannelPlugins` (beside `"channelsEnabled": true`, which a Team or
Enterprise login, or one that is not claude.ai, also needs); `claude-plugin/interweave/README.md` in this
archive names that entry and the development flag, and the repository's
`packaging/claude-plugin/INSTALL.md` gives the settings and the start as
its steps 7 and 8. The archive's SHA-256 is on the
release page and in the `.sha256` file beside it, and the same sources
and `SOURCE_DATE_EPOCH` give the same bytes. The first build of it is
`evidence/2026-10-20-release-archive.md`.

The programs need glibc 2.35 or newer, on x86_64 (ADR-0065, A
2026-10-25): the release run builds them in an ubuntu:22.04 image, checks
every one against 2.35 before packing, and starts each in a clean image
of that glibc before anything is published. A local build is made
against whatever glibc the builder has.

From v0.2.0 the archive and its `.sha256` are signed: a `.minisig` beside
each, made with the archive key whose public half is
`packaging/release-keys.toml`'s `archive.minisign_public_key` on `main`,
and the release run's provenance attestation names the run that built
it. Check both before unpacking:

```sh
minisign -Vm interweave-<version>-x86_64-unknown-linux-gnu.tar.gz -P <archive.minisign_public_key>
gh attestation verify interweave-<version>-x86_64-unknown-linux-gnu.tar.gz --repo BlueTeam-OU/InterWeave
```

v0.1.0's archive has neither, and needs glibc 2.43.

A deployment of many profiles renders each profile's `config.yaml` from its
own record of who is who and installs the units with its own tooling
instead (ADR-0055 decisions 4 and 5); nothing here has to be done by hand.

## What the tests show (plan §22)

Stage 19's Validate rows for this package, each with the test on `main`
that meets it or the limit that stands in its place. The cases under a
user manager run in CI's `rust` job, a required context, in the step
"The packaged user units under the user manager (plan §22)"; the others
run with the workspace's tests.

- **service/autostart lifecycle.**
  `tests/desktop-e2e/tests/user_units.rs::the_installed_daemon_unit_starts_restarts_on_sighup_and_stops`:
  installed, the unit is disabled and inactive; started, it serves the
  profile's own PeerId; SIGHUP ends the run with the status
  `RestartForceExitStatus=` and `SuccessExitStatus=` both name, as the
  manager saw it, restarts it no sooner than `RestartSec=`, logs no
  failure, and the new daemon serves the same PeerId with a fresh lease
  epoch; the control, SIGKILL, is restarted and logged as a failure; a
  stop exits 0, is not restarted and leaves no socket; enabled, the
  daemon is wanted by `default.target` and the enabled client starts
  with its daemon and stops with it. Limit: starting with the manager is
  shown by the `default.target.wants` link, not by a login, since CI's
  manager lingers and no login is made.
- **Client autostart of the daemon (Stage 15, §18 (9), carried to this
  row).** Not met by a test: no client program starts the daemon. What
  starts it is systemd: the person's `enable` of its unit (the row
  above), or starting the client unit, whose `BindsTo=` also pulls its
  daemon in and stops the client with it; the human client, which has no
  unit, shows guidance when no daemon serves its profile. No launch
  action from the client is owed: §18 (9) shows guidance, and
  `human-client-packaging.md` lets a launcher start the daemon without
  requiring it, so a client-side launch is an option the owner may ask
  for. For Linux the carry's substance is met by the units, with the
  limit the row above names; the carry stays by name as that option
  (architect-cto's ruling of 2026-11-03, REPLY 01a1a0df).
- **The SIGHUP carry (Stage 13, §16).**
  `tests/desktop-e2e/tests/daemon.rs::sighup_stops_as_sigterm_does_and_exits_with_the_units_restart_status`:
  SIGTERM's stop sequence in order, both sockets unlinked, the lock
  released and kept, and the exit status read from the unit file, which
  must name it in both keys and not as 0 to 3; started again, the same
  PeerId and a fresh epoch. The restart through the unit is the row
  above.
- **The units' CI cover (§28 step 4, the supervision-units carry).** The
  CI step named above runs the `#[ignore]`d cases of `user_units.rs`, the
  two that need a manager, one at a time under the runner's lingering
  user manager; the others run in the `Tests` step without one.
- **uninstall semantics.**
  `user_units.rs::uninstall_stops_every_instance_removes_the_package_and_keeps_the_profile`:
  every instance stopped, disabled and no longer loaded, the templates
  and the four programs removed, no socket left, every file under the
  profile's four XDG directories kept, a link left by a partial
  uninstall removed by a second run, a run from a shell with no session
  variables and one from a shell with another account's both reaching
  this account's manager, and a reinstall serving the same PeerId from
  the kept key. Beside it:
  `user_units.rs::uninstall_with_no_manager_removes_every_link_with_the_package`,
  `user_units.rs::uninstall_refuses_a_running_manager_it_cannot_reach`
  and `user_units.rs::uninstall_refuses_a_manager_whose_query_fails`
  (removes nothing when it could not stop what runs), and for install,
  `user_units.rs::install_with_a_missing_input_writes_nothing` and
  `user_units.rs::install_over_a_directory_at_a_destination_writes_nothing`.
- **flock on NFS homes.** A limit, by reason: whether the profile lock
  excludes a second host is the NFS server's lock manager's answer, not
  this program's, and mounting NFS needs root, which no test of this
  repository has. What the daemon owns is tested:
  `tests/desktop-e2e/tests/daemon.rs::a_second_daemon_for_the_profile_fails_fast`
  and `crates/config/profile-config/tests/lock.rs::a_child_holding_the_lock_blocks_the_parent_until_it_is_killed`.
  The rule for a shared home is the paragraph "Never run one profile on
  two hosts at once" above.
