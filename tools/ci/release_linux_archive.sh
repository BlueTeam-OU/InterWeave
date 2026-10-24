#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_linux_archive.sh
#
# >>> help
# Build the Linux release archive (plan section 22, packaging/linux):
#
#   tools/ci/release_linux_archive.sh [--bin-dir DIR] [--root DIR] <version> <out-dir>
#
# writes into <out-dir>
#   interweave-<version>-x86_64-unknown-linux-gnu.tar.gz
#   interweave-<version>-x86_64-unknown-linux-gnu.tar.gz.sha256  (sha256sum's format)
#   linux-values.json   {linux_archive, linux_sha256, check:linux_version}
#                       for release_page.py
#
# The archive holds one directory of that name, laid out as
# packaging/linux/install.sh expects to find itself:
#   bin/transport-daemon  bin/transportctl  bin/human-desktop
#   bin/interweave-human
#   systemd/interweave-daemon@.service  systemd/interweave-client@.service
#   install.sh  uninstall.sh  README.md  LICENSE  THIRD-PARTY-NOTICES.md
#   claude-plugin/   the Claude Code marketplace packaging/claude-plugin/build.sh
#                    assembles, with claude-channel in interweave/bin/; it is
#                    added with `claude plugin marketplace add`, not by install.sh;
#                    its plugin.json's version is <version>, which build.sh
#                    refuses unless it is the workspace version (ADR-0065)
#
# Without --bin-dir the four programs and claude-channel are built here,
# `cargo build --release --locked` for x86_64-unknown-linux-gnu; with it,
# DIR holds all five already built (the self-test's stand-ins).
# THIRD-PARTY-NOTICES.md is release_notices.py over `cargo tree --locked
# -e normal` of the five packages for that target (which crates) and
# `cargo metadata --locked` (what each is), or over the files
# RELEASE_COMPILED_LIST and RELEASE_METADATA_JSON name. --root names the
# repository whose manifest, units, scripts and plugin are packaged
# (default: this one).
#
# The archive is byte-for-byte reproducible from the same inputs: names
# sorted, owner and group 0, modes fixed whatever the umask, every mtime
# SOURCE_DATE_EPOCH (default: the commit time of HEAD), gzip without a
# name or time.
#
# REFUSED when <version> is not a Semantic Versioning 2.0.0 version
# without build metadata (MAJOR.MINOR.PATCH[-pre-release], ADR-0065), or
# is not the workspace version -- both release_tag.sh's rule, asked of
# `v<version>` with --manifest, the tag job having asked cargo already --
# or a program, a unit or a script is missing, or build.sh refuses the
# plugin. On any failure once the arguments are read, the version's
# refusal or a step after it, none of the three outputs is left
# in <out-dir>: all three are written in scratch and moved together at
# the end, and a failure removes every one of their names from <out-dir>,
# what this run moved and what an earlier run left.
#
# Exit codes:
#   0  written; the archive's SHA-256 is printed
#   1  a refusal above, or a build step failed
#   2  usage, or release_tag.sh could not read the root manifest
# <<< help

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
target=x86_64-unknown-linux-gnu
# The bridge is built and its crates noticed with the four, but it ships
# only inside the plugin, never in bin/: the plugin's .mcp.json starts
# ${CLAUDE_PLUGIN_ROOT}/bin/claude-channel, and Claude Code, not install.sh,
# is what runs it.
packages=(interweave-transport-daemon interweave-transportctl interweave-human-desktop interweave-human-cli interweave-claude-channel)
programs=(transport-daemon transportctl human-desktop interweave-human)
bridge=claude-channel
units=(interweave-daemon@.service interweave-client@.service)

usage() {
    echo "usage: release_linux_archive.sh [--bin-dir DIR] [--root DIR] <version> <out-dir>" >&2
    exit 2
}
die() { echo "release_linux_archive: $*" >&2; exit 1; }

bin_dir=""
case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
esac
# Options while the first argument is one, each with its value: an option
# left without one is usage, never read as the version.
while [[ "${1-}" == --* ]]; do
    (( $# >= 2 )) || usage
    case "$1" in
    --bin-dir) bin_dir="$2" ;;
    --root) root="$(cd "$2" && pwd)" || usage ;;
    *) usage ;;
    esac
    shift 2
done
[[ $# -eq 2 ]] || usage
version="$1" out_dir="$2"

name="interweave-$version-$target"
archive="$name.tar.gz"
outputs=("$archive" "$archive.sha256" linux-values.json)
# The trap is set before the version check and before anything after it
# can fail, the scratch directory's mktemp included, so every failure from
# here on -- a refused version too -- removes the outputs' names, an
# earlier run's of them too.
scratch=""
finish() {
    local status=$? f
    if [[ -n "$scratch" ]]; then rm -rf "$scratch"; fi
    if (( status != 0 )); then
        for f in "${outputs[@]}"; do rm -f -- "$out_dir/$f"; done
    fi
}
trap finish EXIT

# One rule for the version, the release run's own (ADR-0065 decisions 5
# and 6): its exit 1 is a refusal, its 2 a manifest it could not read.
rc=0
checked="$(bash "$here/release_tag.sh" "v$version" --root "$root" --manifest 2>&1)" || rc=$?
if (( rc == 1 )); then
    die "${checked#::error::}"
elif (( rc != 0 )); then
    echo "release_linux_archive: ${checked#::error::}" >&2
    exit 2
fi

scratch="$(mktemp -d)"
mkdir -p "$out_dir"

if [[ -z "$bin_dir" ]]; then
    build_args=(build --release --locked --target "$target")
    for p in "${packages[@]}"; do build_args+=(-p "$p"); done
    (cd "$root" && cargo "${build_args[@]}") || die "cargo build failed"
    # Where cargo built is cargo's to say: CARGO_TARGET_DIR relative to the
    # root it ran in, CARGO_BUILD_TARGET_DIR, or a config's build.target-dir.
    # A path computed here from one of them would read another directory,
    # and an older build's executables found there would be archived.
    target_dir="$(cd "$root" && cargo metadata --locked --no-deps --format-version 1 \
        | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')" \
        || die "cargo metadata did not name the target directory"
    [[ -n "$target_dir" ]] || die "cargo metadata did not name the target directory"
    bin_dir="$target_dir/$target/release"
fi
for p in "${programs[@]}" "$bridge"; do
    [[ -f "$bin_dir/$p" && -x "$bin_dir/$p" ]] || die "no executable $bin_dir/$p"
done
for f in packaging/linux/install.sh packaging/linux/uninstall.sh packaging/linux/README.md LICENSE \
    "${units[@]/#/packaging/linux/systemd/}"; do
    [[ -f "$root/$f" ]] || die "no $f"
done

stage="$scratch/$name"
mkdir -p "$stage/bin" "$stage/systemd"
# Their modes are the archive's, not the builder's umask: 077 would make
# the same inputs give other bytes.
chmod 0755 "$stage" "$stage/bin" "$stage/systemd"
for p in "${programs[@]}"; do install -m 0755 "$bin_dir/$p" "$stage/bin/$p"; done
for u in "${units[@]}"; do install -m 0644 "$root/packaging/linux/systemd/$u" "$stage/systemd/$u"; done
install -m 0755 "$root/packaging/linux/install.sh" "$stage/install.sh"
install -m 0755 "$root/packaging/linux/uninstall.sh" "$stage/uninstall.sh"
install -m 0644 "$root/packaging/linux/README.md" "$stage/README.md"
install -m 0644 "$root/LICENSE" "$stage/LICENSE"
bash "$root/packaging/claude-plugin/build.sh" --version "$version" "$stage/claude-plugin" "$bin_dir/$bridge" >/dev/null \
    || die "packaging/claude-plugin/build.sh refused the plugin"
# build.sh's copies take the builder's umask; the archive's modes are fixed
# like the rest.
find "$stage/claude-plugin" -type d -exec chmod 0755 {} +
find "$stage/claude-plugin" -type f -exec chmod 0644 {} +
chmod 0755 "$stage/claude-plugin/interweave/bin/$bridge"

metadata="${RELEASE_METADATA_JSON-}"
if [[ -z "$metadata" ]]; then
    metadata="$scratch/metadata.json"
    (cd "$root" && cargo metadata --locked --format-version 1 --filter-platform "$target") > "$metadata" \
        || die "cargo metadata failed"
fi
compiled="${RELEASE_COMPILED_LIST-}"
if [[ -z "$compiled" ]]; then
    compiled="$scratch/compiled.txt"
    tree_args=(tree --locked -e normal --target "$target" --prefix none -f '{p}')
    for p in "${packages[@]}"; do tree_args+=(-p "$p"); done
    (cd "$root" && cargo "${tree_args[@]}") > "$compiled" || die "cargo tree failed"
fi
python3 "$here/release_notices.py" "$metadata" "$compiled" "$stage/THIRD-PARTY-NOTICES.md" "${packages[@]}" >&2 \
    || die "the third-party notices could not be written"
chmod 0644 "$stage/THIRD-PARTY-NOTICES.md"

epoch="${SOURCE_DATE_EPOCH:-$(git -C "$root" log -1 --format=%ct)}"
[[ "$epoch" =~ ^[0-9]+$ ]] || die "SOURCE_DATE_EPOCH '$epoch' is not a number of seconds"
tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$epoch" --format=gnu \
    -C "$scratch" -cf - "$name" | gzip -n -9 > "$scratch/$archive" || die "tar failed"
(cd "$scratch" && sha256sum -- "$archive" > "$archive.sha256")
sha="$(cut -d' ' -f1 "$scratch/$archive.sha256")"
printf '{"linux_archive": "%s", "linux_sha256": "%s", "check:linux_version": "%s"}\n' \
    "$archive" "$sha" "$version" > "$scratch/linux-values.json"
for f in "${outputs[@]}"; do mv -fT -- "$scratch/$f" "$out_dir/$f"; done
echo "linux_archive $out_dir/$archive"
echo "linux_sha256  $sha"
