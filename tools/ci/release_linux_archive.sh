#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_linux_archive.sh
#
# >>> help
# Build the Linux release archive (plan section 22, packaging/linux):
#
#   tools/ci/release_linux_archive.sh [--bin-dir DIR] <version> <out-dir>
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
#   systemd/interweave-daemon@.service  systemd/interweave-client@.service
#   install.sh  uninstall.sh  README.md  LICENSE  THIRD-PARTY-NOTICES.md
#
# Without --bin-dir the three programs are built here, `cargo build
# --release --locked` for x86_64-unknown-linux-gnu; with it, DIR holds
# them already built (the self-test's stand-ins). THIRD-PARTY-NOTICES.md
# is release_notices.py over `cargo metadata --locked` for that target,
# or over the file RELEASE_METADATA_JSON names.
#
# The archive is byte-for-byte reproducible from the same inputs: names
# sorted, owner and group 0, modes fixed whatever the umask, every mtime
# SOURCE_DATE_EPOCH (default: the commit time of HEAD), gzip without a
# name or time.
#
# REFUSED when <version> is not MAJOR.MINOR.PATCH, or a program, a unit
# or a script is missing. On any failure once <version> is checked, a
# refusal or a step after the checks, none of the three outputs is left
# in <out-dir>: all three are written in scratch and moved together at
# the end, and a failure removes every one of their names from <out-dir>,
# what this run moved and what an earlier run left.
#
# Exit codes:
#   0  written; the archive's SHA-256 is printed
#   1  a refusal above, or a build step failed
#   2  usage
# <<< help

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
target=x86_64-unknown-linux-gnu
packages=(interweave-transport-daemon interweave-transportctl interweave-human-desktop)
programs=(transport-daemon transportctl human-desktop)
units=(interweave-daemon@.service interweave-client@.service)

usage() {
    echo "usage: release_linux_archive.sh [--bin-dir DIR] <version> <out-dir>" >&2
    exit 2
}
die() { echo "release_linux_archive: $*" >&2; exit 1; }

bin_dir=""
case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
--bin-dir) [[ $# -ge 2 ]] || usage; bin_dir="$2"; shift 2 ;;
esac
[[ $# -eq 2 ]] || usage
version="$1" out_dir="$2"
[[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] \
    || die "version '$version' is not MAJOR.MINOR.PATCH"

name="interweave-$version-$target"
archive="$name.tar.gz"
outputs=("$archive" "$archive.sha256" linux-values.json)
scratch="$(mktemp -d)"
finish() {
    local status=$? f
    rm -rf "$scratch"
    if (( status != 0 )); then
        for f in "${outputs[@]}"; do rm -f -- "$out_dir/$f"; done
    fi
}
trap finish EXIT
mkdir -p "$out_dir"

if [[ -z "$bin_dir" ]]; then
    build_args=(build --release --locked --target "$target")
    for p in "${packages[@]}"; do build_args+=(-p "$p"); done
    (cd "$root" && cargo "${build_args[@]}") || die "cargo build failed"
    bin_dir="${CARGO_TARGET_DIR:-$root/target}/$target/release"
fi
for p in "${programs[@]}"; do
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

metadata="${RELEASE_METADATA_JSON-}"
if [[ -z "$metadata" ]]; then
    metadata="$scratch/metadata.json"
    (cd "$root" && cargo metadata --locked --format-version 1 --filter-platform "$target") > "$metadata" \
        || die "cargo metadata failed"
fi
python3 "$here/release_notices.py" "$metadata" "$stage/THIRD-PARTY-NOTICES.md" "${packages[@]}" >&2 \
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
