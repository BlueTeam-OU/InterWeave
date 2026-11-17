<!-- SPDX-License-Identifier: Apache-2.0 -->
# `claude plugin update` to a lower version, run 2026-11-17

architect-cto asked (REQUEST 01a1e9a9) for the case `INSTALL.md` said was not measured: `claude plugin update` when
the marketplace holds a lower version of the plugin than the one installed. It ran once on this host, by hand from
this login's shell with no Claude Code session, the way SPIKE-001 S20 ran: no model, no channel flag, no credential.
Nothing left the host: no plugin was published and no marketplace entry was made.

## Setup

- **The two builds.** The Linux archives of the `v0.3.0` and `v0.2.0` releases (`gh release download`, each
  `sha256sum -c` `OK`). Each carries `claude-plugin/`, the marketplace directory `build.sh` assembles, with
  `plugin.json` at `0.3.0` and `0.2.0`. They are the released builds, so no local build was made: the case needs
  two plugins of different versions, and these two differ in their code as well as their number.
- **Claude Code** 2.1.322 (`claude --version`), with `DISABLE_AUTOUPDATER=1`.
- **The account.** `CLAUDE_CONFIG_DIR` was an empty scratch directory. `$SCRATCH` stands for the scratch tree,
  which was removed afterwards. The marketplace was `$SCRATCH/mp`, a copy of one archive's `claude-plugin/`.

## The run

```text
$ claude plugin marketplace add $SCRATCH/mp                 # mp: the 0.3.0 archive's claude-plugin/
✔ Successfully added marketplace: interweave (declared in user settings)
$ claude plugin install interweave@interweave
✔ Successfully installed plugin: interweave@interweave (scope: user)
3 userConfig options not yet set — run /plugin configure interweave@interweave in Claude Code, or pass --config KEY=VALUE.
$ claude plugin list
  ❯ interweave@interweave
    Version: 0.3.0
    Read from: $SCRATCH/mp/interweave
# mp replaced by the 0.2.0 archive's claude-plugin/
$ claude plugin marketplace update interweave
Updating marketplace: interweave...Validating local marketplace
✔ Successfully updated marketplace: interweave
$ claude plugin update interweave@interweave
Checking for updates for plugin "interweave@interweave"…
✔ Plugin "interweave" updated from 0.3.0 to 0.2.0 for scope user. Restart to apply changes.
$ claude plugin list
  ❯ interweave@interweave
    Version: 0.2.0
    Read from: $SCRATCH/mp/interweave
```

Every command exited 0. Afterwards `installed_plugins.json` named version `0.2.0`, at
`$SCRATCH/cfg/plugins/cache/interweave/interweave/0.2.0`, and the cache held both `0.2.0` and `0.3.0`.

## What the run shows

- **A lower version is reported and recorded as an update.** The report has the same shape as S20's case B for a
  higher one: "updated from 0.3.0 to 0.2.0". Nothing warned that the version went down, and nothing asked to
  confirm. So `plugin update` follows the version the marketplace holds, higher or lower, and only an equal one is
  "already at the latest version" (S20 case A).
- **`claude plugin list` after the update showed 0.2.0.** With a directory marketplace what runs is what is in the
  folder (S20 fact 26), so this run cannot tell the folder's effect from the update's: no `claude plugin list` was run
  between the folder's replacement and the update.

## Not run

- A session after the update. Which bridge the next session starts was not checked. By S20 fact 26 it is the one in
  the folder, here 0.2.0, but this run did not show it.
- A marketplace other than a local directory. Whether a remote marketplace that lowers its version is handled the
  same way was not measured: no marketplace has been published (`INSTALL.md` installs from the archive's
  directory).
