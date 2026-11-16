<!-- SPDX-License-Identifier: Apache-2.0 -->
# INSTALL.md steps the v0.3.0 run left out, run 2026-11-16

architect-cto asked (REQUEST 01a1e23e) for four steps of `packaging/claude-plugin/INSTALL.md` that
`2026-11-11-v0.3.0-install-run.md` lists under "What was not run": the upgrade path, a daemon restart during a
session, a second session on the same endpoint, and `claude plugin configure --values-stdin`. They ran on this host,
in a scratch `HOME`, the way the 2026-11-11 run was made. Nothing left the host: no plugin was published and no
marketplace entry was made. This record gives each step's command, what was seen, and what was not run and why.
Section 5 answers a second request (REQUEST 01a1e54b), step 8's interactive session, run later the same day; its
runs are named x0 to x2, i1 to i3 and ic1, and the run names r1 to r3 elsewhere are sections 1 and 2's.

## Setup

- **Archives.** `interweave-0.3.0-x86_64-unknown-linux-gnu.tar.gz` from the `v0.3.0` release page
  (`gh release download v0.3.0`), `sha256sum -c` `OK`. Its plugin's `plugin.json` says `0.3.0`, and its bridge logs
  `started, version 0.3.0`.
- **A second release, and a local build.** The request said only `v0.3.0` exists. The `v0.2.0` release's Linux
  archive also carries `claude-plugin/`, its `plugin.json` at `0.2.0` (`sha256sum -c` `OK`), so the upgrade ran
  twice: first from `v0.3.0` to a local build, then, after an uninstall, from the `v0.2.0` release to the `v0.3.0`
  release. For that build a
  detached worktree of `175d8cc9f`
  (`origin/main` on 2026-11-16) had `[workspace.package] version` changed from `0.3.0` to `0.3.1` and nothing else
  (`Cargo.lock` followed). Then `cargo build --release -p interweave-claude-channel` and
  `bash packaging/claude-plugin/build.sh <out> target/release/claude-channel` from that worktree. Its
  `plugin.json` says `0.3.1`, and its bridge logs `started, version 0.3.1`. A real 0.3.1 release would differ in its
  code, not only its number.
- **Claude Code** 2.1.322, as every session's `init` reported it. The auto-updater was off.
- **The account.** As on 2026-11-11: a scratch `HOME` (an empty directory, mode 0700) and a private
  `XDG_RUNTIME_DIR`, with the environment cleared to the same variables. `~/.local/bin/claude` was a link to the
  login's 2.1.322 binary. The login's credential reached each session through its environment only. Steps 1 to 6
  ran as INSTALL.md gives them, with the 2026-11-11 run's two stand-ins. Each daemon ran as a transient unit of the
  login's user manager (`systemd-run --user --unit=j103-interweave-daemon-<profile> -p Type=exec -p UMask=0077 …`), not
  through `systemctl --user enable`. Step 7 was presented by bubblewrap (`bwrap --dev-bind / / --ro-bind <dir>
  /etc/claude-code`, `<dir>` holding the host's `managed-settings.json` unchanged and step 7's
  `managed-settings.d/interweave.json` as INSTALL.md gives it), in the one run of sections 1 to 4 that needed a
  registered channel (run r3).
- **Profiles.** `claude` (port 4001, endpoint `claude`, `allowed_peers` the peer) as step 3 gives it, and `peer`
  (port 4002, one `human` endpoint for `human-client`, `allowed_peers` the `claude` PeerId), its address added with
  `interweave-human --profile peer peers add /ip4/<host LAN address>/tcp/4001/p2p/<claude>`, which answered `added`.
  Both daemons answered `transportctl --profile <p> status` with `health Healthy`.
- **Sessions** (sections 1 to 4). Each was `claude -p --input-format stream-json --output-format stream-json --verbose --debug-file …
  --allowedTools mcp__plugin_interweave_interweave__reply mcp__plugin_interweave_interweave__status`, plus
  `--channels plugin:interweave@interweave` in run r3. A driver wrote the user turns, ran the steps between them,
  and closed stdin once every `init` had its `result`. Each turn that asked for the bridge's state was "Call the
  InterWeave status tool once and print its full output verbatim, nothing else." The `status` values below are read
  from the tool's own result in the stream, not from the model's text. Every session reported `permissionMode`
  `default` and the server `plugin:interweave:interweave` `connected`.
- Install (steps 1, 2, 3 and 6, from the archive) answered as on 2026-11-11: `Successfully added marketplace`,
  `Successfully installed plugin`, `1 userConfig option not yet set`, then `Version: 0.3.0` and `Read from:
  ~/.local/share/interweave/claude-plugin/interweave` in `claude plugin list`.

## 1. A daemon restart during a session

| run | what | seen |
|---|---|---|
| r1 | status; `systemctl --user restart j103-interweave-daemon-claude`; 45 s; status | First status: `endpoint_lease_state` `held`, no `lease_gap`. Second: `held` under a new `endpoint_lease_epoch`, and `lease_gap` `{epoch: <new>, lost_epoch: <first>, since: 02:59:16.589Z, until: 02:59:17.414Z}`. The bridge's log: `the session ended; reconnecting`, `a session did not open: BackendUnavailable`, `a session opened, the lease held`, within 0.83 s |
| r3 | channel registered; first turn: note each `<channel>` message, do not `reply` yet; peer sends question one; restart; then: `reply` once with question one's token; then: answer each message; peer sends question two | Question one started a turn of its own; the model wrote `noted`. After the restart, `reply` with question one's token answered `InvalidArgument: an argument is invalid, or the reply token is unknown, expired or from an earlier lease` (`isError` true). `status` then showed `lease_gap` with the two epochs and `since` 03:15:14.514Z, `until` 03:15:15.272Z. Question two, sent 41 s after the restart, started a turn, and `reply` in the chat envelope answered `remote transport accepted at endpoint human`. The peer's listener printed it as `message` `u1`, text `4 + 5 = 9`, `status` `unread` |
| r3-control | as r3, without `--channels` | No `Channel notifications` line in the debug log, and question one started no turn. The bridge still took it: `status` after the restart had `reply_tokens` 1. The model said no message had arrived and called no `reply` |

What r3 also shows:

- **The peer's connection came back by itself.** Its listener printed `disconnected` at the restart and then `path`
  `direct` to the `claude` PeerId, with nothing done on either side. Question two was `accepted` by the peer's
  daemon.
- **`reply_tokens` still counted question one's token after the restart** (1 in r3's second status, and in the
  control's), though `reply` refused that token. The count includes tokens of an earlier lease.
- In r3 the debug log has `Channel notifications re-registered after reconnect` five times, at 03:14:15, 03:14:29,
  03:15:30, 03:15:51 and 03:15:56. They are not one per restart; what each is was not traced.

Not run: a message sent while the daemon was down. The gap was under a second in the two runs that recorded it (r1 and
r3), too short to aim a send into. Nor the pull mode's first `receive` after a gap: the plugin starts the bridge in push mode.

## 2. A second session on the same endpoint

Run r2: session A asked for `status`, slept 60 s, asked again, and ended. Session B was started 50 s after A, in
another working directory on the same account and profile. It asked for `status`, slept 75 s, and asked again,
40 s after A had ended.

| session | status | seen |
|---|---|---|
| A | 1st and 2nd | `held`, the same epoch both times |
| B | 1st (A running) | `endpoint_lease_state` `endpoint refused`, `endpoint_lease_epoch` null, `last_connect_error` `EndpointInUse`, `transport_health` null |
| B | 2nd (A ended) | `held` under an epoch of its own, `last_connect_error` null, no `lease_gap` |

The bridge's log, shared by both bridges as INSTALL.md says: B's `a session did not open: EndpointInUse` at
03:03:15.637Z, once; A's `ended: a signal` at 03:04:11.357Z; B's `a session opened, the lease held` at
03:04:17.652Z, 6.3 s later. So the waiting session takes the endpoint once the first one ends, at its next retry.
One run.

Not run: a second session on another endpoint of the same profile, which is outside the request's "a second session
on the same endpoint"; and two sessions on this endpoint started together, before either held the lease, which this
run did not attempt: it ran the one order INSTALL.md describes, a second session joining while the first holds it.

## 3. `claude plugin configure --values-stdin`

Each command ran in the scratch account, with no session running. After each, `claude plugin configure
interweave@interweave </dev/null` listed the options, and `pluginConfigs` in `~/.claude/settings.json` was read.

| # | stdin | answer, exit | then |
|---|---|---|---|
| — | (before) | — | `profile` and `endpoint` `set`, `log_level` `not set`; settings `{profile: claude, endpoint: claude}` |
| 1 | `{"profile": "claude", "endpoint": "claude"}`, INSTALL.md's own | `Configuration saved. Restart Claude Code to apply it.`, 0 | unchanged |
| 2 | `{"log_level": "debug"}` | the same, 0 | `log_level` `set`; settings keep `profile` and `endpoint` and add `log_level: debug`. A key left out keeps its value |
| 3 | `{"log_level": "loud"}` | the same, 0 | settings `log_level: loud`. The value was not checked |
| 4 | `{"colour": "blue"}` | `Failed to save configuration: This plugin has no option "colour". Known options: profile, endpoint, log_level.`, 1 | unchanged |
| 5 | `log_level=debug` | `The input on stdin isn't a JSON object of strings. Example: …`, 1 | — |
| 6 | `{"log_level": ""}` | `Configuration saved. …`, 0 | `configure` lists `log_level` as `not set`, while the settings hold `log_level: ""` |

The next session's `status` after each:

| after | status |
|---|---|
| 3 (`loud`), run r4-loud | `endpoint_lease_state` `bridge refused`, `refused` `{class: usage, reason: --log-level is error, warn, info or debug, not "loud"}` |
| `{"log_level": "debug"}` again, run r4-debug | `held`, `log.level` `debug` |
| 6 (`""`), run r4-empty | `bridge refused`, `reason` `--log-level is error, warn, info or debug, not ""` |

So an empty value is not the same as an unset one. `configure` shows it as `not set`, but Claude Code substitutes the
empty string into `.mcp.json`'s `--log-level ${user_config.log_level}`, not the declared default `info`, and the
bridge refuses it, as it refuses any level not in its four (`apps/claude-channel/src/main.rs`). `{"log_level":
"info"}` then gave `held` again. Only `log_level` was tried empty or invalid; `profile` and `endpoint` were not.

Not run: the in-session `/plugin configure`, since the driver runs `-p` sessions only, and a way to remove a key once it is set (stdin takes an object of
strings, and none was found that unsets one).

## 4. Upgrading the plugin

INSTALL.md step 6's order: replace `~/.local/share/interweave/claude-plugin` with the newer marketplace, start a new
session, then `claude plugin marketplace update interweave` and `claude plugin update interweave@interweave`.

### From the `v0.2.0` release to the `v0.3.0` release

Run second, after the round below. The scratch account's plugin was uninstalled (`claude plugin uninstall`, `claude plugin marketplace remove`, both
`✔ Successfully …`), its folder moved aside, and step 6 run again from the `v0.2.0` archive's `claude-plugin/`:
`Version: 0.2.0`. `pluginConfigs` was then `{profile: claude, endpoint: claude}`: the `log_level` section 3 had set
was gone. The daemons stayed the 0.3.0 archive's.

| step | command | seen |
|---|---|---|
| session (v1) | status | the bridge logs `started, version 0.2.0`; `held` against the 0.3.0 daemon |
| replace | the folder moved aside, the `v0.3.0` archive's `claude-plugin/` copied in its place | `claude plugin list`: `Version: 0.3.0`; `installed_plugins.json` still `0.2.0` |
| session (v2) | status | `started, version 0.3.0`; `held` |
| update | `claude plugin marketplace update interweave` | `✔ Successfully updated marketplace: interweave` |
| update | `claude plugin update interweave@interweave` | `✔ Plugin "interweave" updated from 0.2.0 to 0.3.0 for scope user. Restart to apply changes.`; `installed_plugins.json` `0.3.0` |
| session (v3) | status | `started, version 0.3.0`; `held` on endpoint `claude` of the `claude` profile's daemon; `log.level` `info`, which is also the unset default. `pluginConfigs` was not read after this update. Only `profile` is shown kept, by the lease, since its default names a profile `default` that has no daemon; `endpoint` `claude` and `log_level` `info` are both defaults, so the lease and `status` would read the same had they been lost |

### From the `v0.3.0` release to a local build at 0.3.1

Run first, on the account as installed from the `v0.3.0` archive.

| step | command | seen |
|---|---|---|
| replace | `rm -rf ~/.local/share/interweave/claude-plugin && cp -R <0.3.1 marketplace> ~/.local/share/interweave/claude-plugin` | `claude plugin list` at once: `Version: 0.3.1`, `Read from: ~/.local/share/interweave/claude-plugin/interweave`. `~/.claude/plugins/installed_plugins.json` still `version` `0.3.0`, `installPath` `~/.claude/plugins/cache/interweave/interweave/0.3.0` |
| session (u1) | status | `init` lists the plugin at `0.3.1` from the folder; the bridge logs `started, version 0.3.1`; `held` |
| update | `claude plugin marketplace update interweave` | `Validating local marketplace`, `✔ Successfully updated marketplace: interweave`, exit 0 |
| update | `claude plugin update interweave@interweave` | `✔ Plugin "interweave" updated from 0.3.0 to 0.3.1 for scope user. Restart to apply changes.`, exit 0. `installed_plugins.json`: `version` `0.3.1`, `installPath` `…/cache/interweave/interweave/0.3.1`. The cache holds both versions, the 0.3.0 one marked `.orphaned_at` |
| update again | `claude plugin update interweave@interweave` | `interweave is read from its folder, ~/.local/share/interweave/claude-plugin/interweave: nothing to update. Edits there take effect at the next session start or /reload-plugins.`, then `✔ interweave is already at the latest version (0.3.1).` |
| session (u2) | status | `init` lists `0.3.1` from the folder; `started, version 0.3.1`; `held`; `log.level` `info`. `pluginConfigs` still `{profile: claude, endpoint: claude, log_level: info}` |

In both rounds the replaced folder is what the next session ran, before either update command, as S20 measured on
2.1.286. What
the two commands changed is Claude Code's install record and its cache copy, and the report from the old version to
the new that INSTALL.md says to expect. All three settings were read back after the 0.3.0 to 0.3.1 update; after the
0.2.0 to 0.3.0 one, only `profile` is shown kept (row v3).

Not run: an update to a lower version, which was not attempted because the request asked for the upgrade path, though
the 0.2.0 and 0.3.0 archives on hand would have allowed one; an update, or the folder replaced, while a session was
running, which was not attempted: every session here ended before the next step ran; and a 0.2.0
bridge's channel traffic against the 0.3.0 daemon (only its `status`), since the request was the upgrade path and not
an old bridge against a new daemon.

## 5. Step 8's interactive session

architect-cto asked (REQUEST 01a1e54b) for step 8's interactive session, which the 2026-11-11 run left "Not run":
whether a warning screen is shown (SPIKE-001 fact 30), and the `Channel notifications registered` line. It ran later
the same day, 17:22 to 17:30 UTC, in a fresh scratch `HOME` set up as above: steps 1 to 6 from the `v0.3.0` release
archive (`sha256sum -c` `OK`, `Version: 0.3.0`, `enabled`) and the daemon as a transient unit (`health Healthy`).
Step 7 was presented by bubblewrap with the same private `/etc` as on 2026-11-11: every host entry bound read-only,
and `/etc/claude-code` a directory holding the host's `managed-settings.json` unchanged and step 7's drop-in.

- **Claude Code** 2.1.322, as the session's banner reported it. The auto-updater was off.
- **The session** was `claude --channels plugin:interweave@interweave --debug-file <path>`, step 8's command plus the
  debug file, run in a pseudo-terminal of 120 by 40 columns by a driver modelled on SPIKE-001's `run_tty.py`. The
  driver waited for the prompt, waited 5 s, typed "Call the InterWeave status tool once and print its full output
  verbatim and nothing else.", waited 60 s, and typed `/exit` and Enter. That was the script of i1 to i3 and ic1.
  x0 to x2 came first, to find the screens a new `HOME` shows, and typed no prompt: x0 waited 20 s; x1 waited 8 s,
  pressed Enter on the text-style picker (keeping its default), Enter at "Press Enter to continue", and waited 10 s;
  x2 waited for the folder-trust question, pressed Down and Enter, and waited 25 s; each then typed `/exit` and
  Enter. So x2 called no `status`. The `status` values below are read from the terminal's text.
- **Permission mode.** The banner showed auto mode on, and `status` ran with no prompt to approve it.

| run | step 7 | screens before the prompt | debug log | `status` |
|---|---|---|---|---|
| x0 | yes | the text-style picker; the closing `/exit` and Enter answered it with its default, the security notes followed, and the driver ended the session there with SIGTERM 5 s later; that answer was not kept, since x1 showed the picker again | no `Channel` line for the server | not called |
| x1 | yes | text style, then the security notes ("Press Enter to continue"), then the folder-trust question, whose default is "No, exit"; the closing `/exit` and Enter answered it with that default, and the session ended | no `Channel` line for the server | not called |
| x2 | yes | the folder-trust question; Down and Enter chose "Yes, I trust this folder" | `Channel notifications registered` | not called |
| i1, i2, i3 | yes | none | `Channel notifications registered`, twice in each | `endpoint_lease_state` `held`, `local_peer_id` the profile's PeerId, `local_endpoint` `claude`, `transport_health` `healthy`, `log.level` `info` |
| ic1 | no (the host's `managed-settings.json` alone) | none | `Channel notifications skipped: plugin interweave@interweave is not on the approved channels allowlist (use --dangerously-load-development-channels for local dev)`, twice | as i1 |

Every line is the server `plugin:interweave:interweave`'s. The registration line was `Channel notifications
registered` and not the `re-registered after reconnect` that a `--channels` print session logs (SPIKE-001 fact 40).

What the runs show:

- **No warning screen, 4 runs of 4** that reached the prompt with step 7 (x2, i1 to i3). No screen named a
  development channel or asked to confirm one. The screens before the prompt were Claude Code's own first-run screens
  in a new `HOME` — text style, security notes, folder trust. x0 and x1 ended at them, with no channel registered. The
  two that x1 accepted, text style and security notes, were not shown again. The folder trust, which x1 answered
  "No, exit", was shown again in x2, and not again once x2 answered "Yes": x2 showed only the folder trust, and i1 to i3 showed none. x0's answer to the picker, given as the
  driver ended it, was not kept.
- **The registration line, 4 runs of 4** that reached the prompt with step 7, as INSTALL.md gives it.
- **Without step 7 the terminal says so.** In ic1, besides the debug log's line, the terminal showed `plugin
  interweave@interweave is not on the approved channels allowlist (use --dangerously-load-development-channels…`. It
  was captured after the prompt was typed, while the debug log's line came before. A print session without step 7
  showed no such line on stdout or stderr (SPIKE-001 S25, fact 47). One run, with `--debug-file`; whether the
  terminal shows it without debug mode was not run.
- **`status`'s `log.file` and `profile_desired_channels`.** In all four sessions that called it (i1 to i3, ic1), `log.file` was
  `null` with `log.file_error` `directory_not_private`, and `profile_desired_channels` was `{"unknown":
  "ConfigDirUnguarded"}`. The 2026-11-11 record traced both to bubblewrap's user namespace, where `/` and `/home` are
  owned by uid 65534, and found the same binary outside it reporting the log file and `{"as_configured":[]}`. These
  runs used the same stand-in, so they say nothing new there.

Not run: a message from a peer into the interactive session, and a reply; the development-flag path; a session
without `--debug-file`; a step 7 written by root in the host's `/etc`; and another build of Claude Code. The driver,
screens and debug logs stayed in the session's scratch directory and are not committed: they carry host paths.

## What INSTALL.md said that this changes

- **Step 8.** It now cites section 5 for the absence of a warning screen and for the registration line, limits both
  counts to the 4 sessions that reached the prompt, of 6 started with step 7, names the first-run screens seen in the one new `HOME` run,
  and quotes the refusal fragment the terminal showed without step 7 in the one run made.
- **Step 6, configure.** It said a JSON object piped to `--values-stdin` changes the settings and the next start
  gets them. It did not say that a key left out keeps its value, that a value is not checked until the bridge
  starts, or that an empty value is shown as `not set` and still refused by the bridge. INSTALL.md now says so.
- **Step 6, upgrade.** It now says `claude plugin list` and the next session see the new version once the folder is
  replaced, and that the two commands bring Claude Code's install record to it.
- **"More than one session per endpoint."** It said the second session keeps retrying. It now says it takes the
  endpoint once the first session ends.
- **"What a missed message was."** The `lease_gap` and the refused reply token were as it says. It now gives the
  refusal's text, and says the peer's connection came back by itself in the run made.

## What was not run

Beyond each section's own list:

- A real fresh Linux account, `systemctl --user enable --now`, and a root-written step 7 in the host's `/etc` (stood
  in for, as on 2026-11-11).
- An interactive session in sections 1 to 4: their driver runs `-p` stream-json sessions only, with no terminal.
  Section 5 ran step 8's interactive session with a driver of its own.
- `uninstall.sh`. On 2026-11-11 it stopped this login's live units, and removal was not part of this request. The
  scratch account's plugin was not uninstalled at the end either: the scratch `HOME` was deleted whole.

The drivers, raw stream-json events, debug logs and peer outputs stayed in the session's scratch directory and are
not committed: they carry host paths and reply tokens. The driver was `drive.py`, one stream-json session through a
list of user turns, shell steps and waits, with the token passed only in the session's environment.
