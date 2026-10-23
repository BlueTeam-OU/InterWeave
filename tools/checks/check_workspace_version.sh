#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/checks/check_workspace_version.sh
#
# >>> help
# Is the workspace version the one place a first-party crate's version
# comes from (ADR-0065 decision 5)?
#
#   tools/checks/check_workspace_version.sh
#   tools/checks/check_workspace_version.sh --root <dir>
#
# The release version is the root Cargo.toml's `[workspace.package]
# version`; the release run refuses a tag that is not `v` and it
# (tools/ci/release_tag.sh). That check reads one value, so it holds for
# every artifact only while every crate takes that value. This refuses:
#
#   - a root manifest with no `[workspace.package] version`, or one
#     carrying build metadata (`+…`): a tag carries none (decision 6),
#     so such a version could never be released;
#   - a workspace member whose `[package] version` is anything but
#     `version.workspace = true` -- its own literal, even one equal to
#     the workspace's today, is a second source that the next bump
#     leaves behind, and an omitted version is cargo's 0.0.0;
#   - a member whose version, as cargo resolves it, differs from the
#     workspace's.
#
# THE MEMBERS ARE CARGO'S, not a glob: `cargo metadata --no-deps` from
# the root names them, so a crate added to `[workspace].members` is
# covered the day it lands. The spike harnesses under spikes/ are their
# own workspaces with frozen locks (check_spike_locks.sh) and are not
# members; the vendored crates under third_party/ keep their upstream
# versions and are not members either.
#
# Exit codes:
#   0  every member inherits the workspace version
#   1  a member carries its own version, or the workspace has none
#   2  usage, or cargo metadata failed
# <<< help

set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
--root)
    [[ $# -eq 2 ]] || { echo "usage: check_workspace_version.sh [--root <dir>]" >&2; exit 2; }
    root="$2"
    ;;
"") ;;
*) echo "usage: check_workspace_version.sh [--root <dir>]" >&2; exit 2 ;;
esac

metadata="$(cargo metadata --no-deps --format-version 1 --manifest-path "$root/Cargo.toml")" \
    || { echo "check_workspace_version: cargo metadata failed in $root" >&2; exit 2; }

# The program is an argument, not stdin: stdin carries cargo's JSON.
program=$(cat <<'PY'
import json, sys, tomllib

root_manifest = sys.argv[1]
metadata = json.load(sys.stdin)
with open(root_manifest, "rb") as f:
    workspace = tomllib.load(f).get("workspace", {}).get("package", {})

version = workspace.get("version")
if not isinstance(version, str):
    sys.exit(f"check_workspace_version: {root_manifest} has no [workspace.package] version "
             "(ADR-0065 decision 5: it is the release version)")
if "+" in version:
    sys.exit(f"check_workspace_version: the workspace version {version} carries build metadata, "
             "which a release tag never does (ADR-0065 decision 6)")

members = set(metadata["workspace_members"])
problems = []
for package in metadata["packages"]:
    if package["id"] not in members:
        continue
    path = package["manifest_path"]
    with open(path, "rb") as f:
        declared = tomllib.load(f).get("package", {}).get("version")
    if declared != {"workspace": True}:
        problems.append(f"{path}: version is {declared!r}, not version.workspace = true")
    elif package["version"] != version:
        problems.append(f"{path}: cargo resolves {package['version']}, not the workspace's {version}")

if problems:
    print("check_workspace_version: a first-party crate carries its own version "
          "(ADR-0065 decision 5: the workspace version is the only one):", file=sys.stderr)
    for p in problems:
        print(f"  {p}", file=sys.stderr)
    sys.exit(1)
print(f"check_workspace_version: {len(members)} workspace members inherit version {version}")
PY
)
printf '%s' "$metadata" | python3 -c "$program" "$root/Cargo.toml"
