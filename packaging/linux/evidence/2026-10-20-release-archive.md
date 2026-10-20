<!-- SPDX-License-Identifier: Apache-2.0 -->
# Host run: the Linux release archive, 2026-10-20

The first build of the Linux release archive from the repository's
sources, by `tools/ci/release_linux_archive.sh` with no stand-ins: the
three programs built `--release --locked`, the notices read from the
real dependency graph. It is the evidence that the release run's Linux
step works end to end on a host. The release run itself is
devex-tooling's tagged workflow, and has not run.

- Commit: `24ac205f` (the archive script's commit; the run started there)
- Host: Fedora 43, x86_64; `rustc 1.99.0 (Fedora 1.99.0-1.fc43)`, the
  host's system toolchain, not `rust-toolchain.toml`'s 1.98.1: this
  account has no rustup, so the binaries are not the pinned compiler's
- Command: `CARGO_BUILD_JOBS=2 tools/ci/release_linux_archive.sh 0.0.0 <out>`
  (the build took 55 min 46 s at two jobs)

## Result

```text
598 crates, 347 distinct licence texts, 55 crates with no licence file
linux_sha256  af0885b5f64319b90f5b3f5d6572f9fa92cf42d4d2eb8889203fd56fb1a7f7fb
```

The 598 crates were the metadata's unified graph, the Android app's
features included (Skia's bindings among them), not what these programs
compile; the review of #324 found it (B F1), and since then the notices
list the crates `cargo tree -e normal` gives for the three packages: 460
over the same graph, 42 of them with no licence file, none of Skia's.

The archive, 70 686 957 bytes, as `tar -tzv` lists it (mode, size, name):

```text
drwxr-xr-x         0 interweave-0.0.0-x86_64-unknown-linux-gnu/
-rw-r--r--     11358 interweave-0.0.0-x86_64-unknown-linux-gnu/LICENSE
-rw-r--r--      5021 interweave-0.0.0-x86_64-unknown-linux-gnu/README.md
-rw-r--r--   1113433 interweave-0.0.0-x86_64-unknown-linux-gnu/THIRD-PARTY-NOTICES.md
drwxr-xr-x         0 interweave-0.0.0-x86_64-unknown-linux-gnu/bin/
-rwxr-xr-x 178425664 interweave-0.0.0-x86_64-unknown-linux-gnu/bin/human-desktop
-rwxr-xr-x 157599720 interweave-0.0.0-x86_64-unknown-linux-gnu/bin/transport-daemon
-rwxr-xr-x  20369376 interweave-0.0.0-x86_64-unknown-linux-gnu/bin/transportctl
-rwxr-xr-x      3055 interweave-0.0.0-x86_64-unknown-linux-gnu/install.sh
drwxr-xr-x         0 interweave-0.0.0-x86_64-unknown-linux-gnu/systemd/
-rw-r--r--      1251 interweave-0.0.0-x86_64-unknown-linux-gnu/systemd/interweave-client@.service
-rw-r--r--      1723 interweave-0.0.0-x86_64-unknown-linux-gnu/systemd/interweave-daemon@.service
-rwxr-xr-x      2980 interweave-0.0.0-x86_64-unknown-linux-gnu/uninstall.sh
```

The programs are large because the release profile keeps line tables
(`debug = 1` in the root `Cargo.toml`, for symbolicated backtraces); the
archive does not strip them.

## Checked after the build

- **Reproducible.** The same three binaries archived twice with
  `SOURCE_DATE_EPOCH=1760000000`: both runs
  `98e48a3937cbc0f97fc70798df3bec8eb29ae3c60d8d037a175b746fe8306cb4`.
  (The first run's hash differs because its epoch was HEAD's commit
  time.)
- **It installs as packaged.** Unpacked, its own `install.sh`, run into
  a scratch `HOME` with a `systemctl` that finds no user manager,
  placed the programs and units and said "nothing is enabled";
  `transport-daemon --help` and `transportctl --help` from the installed
  copies print their usage.

## What this does not show

- The units under a user manager: that is
  `2026-10-20-user-units.md`, against the workspace binaries.
- Crates that publish no licence file in their crate (55 in this run's
  list, 42 in the corrected one) are listed with their licence
  expression and source repository, with no text printed. Their texts
  are upstream's, not reproduced here.
- A build by the pinned 1.98.1 toolchain, or on the release runner.
