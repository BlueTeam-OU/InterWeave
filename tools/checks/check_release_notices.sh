#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/checks/check_release_notices.sh
#
# >>> help
# Would every shipped build's third-party notices be written today?
#
#   tools/checks/check_release_notices.sh [--root DIR]
#
# tools/ci/release_notices.py refuses a build whose notices it cannot
# ship whole (ADR-0058): a crate publishing no licence file that
# tools/ci/notice_derived.json does not list, a listed header changed
# under its digest, a build-dependency tools/ci/notice_sources.json does
# not classify, a sources entry for another version. The release run is
# where it runs, so without this a dependency bump that brings one is
# found at the tag. This runs the tool, as the release does, over each
# shipped build's graph, writing the page to a scratch file:
#
#   the Linux archive  `target=` and `packages=(...)` in
#                      tools/ci/release_linux_archive.sh;
#   the APK's library  the one `release_crate_notices.sh <target>
#                      <package>` call in .github/workflows/release.yml.
#
# Each READ from where the release takes it, one line of the shape known
# here, or exit 2 asking for the reading to be updated with the release.
# The graph is the release scripts' own: `cargo metadata --locked
# --format-version 1 --filter-platform <target>` and `cargo tree --locked
# -e normal --target <target> --prefix none -f '{p}' -p <package>...`;
# neither builds anything, and neither needs the target installed.
#
# Exit codes:
#   0  every shipped build's notices are written; the counts are printed
#   1  release_notices.py refuses one; its message is printed
#   2  usage, cargo failed, or a release source could not be read
# <<< help

set -uo pipefail

me="check_release_notices"
ROOT="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )/../.." && pwd )"

case "${1:-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
--root)
    [[ $# -eq 2 ]] || { echo "usage: $me [--root DIR]" >&2; exit 2; }
    ROOT="$2"
    ;;
"") ;;
*) echo "usage: $me [--root DIR]" >&2; exit 2 ;;
esac
cd "$ROOT" || exit 2

ARCHIVE=tools/ci/release_linux_archive.sh
RELEASE=.github/workflows/release.yml
WORD='[A-Za-z0-9_.-]+'

scratch="$(mktemp -d)"; trap 'rm -rf "$scratch"' EXIT

# the_line <file> <ERE>: the one line of <file> matching <ERE>, or exit 2.
the_line() {
    local hits n
    hits="$(grep -E -- "$2" "$1" 2>/dev/null)"
    n=0; [[ -n "$hits" ]] && n="$(wc -l <<<"$hits")"
    if (( n != 1 )); then
        echo "$me: $1 has $n line(s) of the shape /$2/, not one — update this check's reading with the release" >&2
        exit 2
    fi
    printf '%s\n' "$hits"
}

line="$(the_line "$ARCHIVE" "^target=$WORD\$")" || exit 2
linux_target="${line#target=}"
line="$(the_line "$ARCHIVE" "^packages=\\(($WORD)( $WORD)*\\)\$")" || exit 2
line="${line#packages=(}"
read -r -a linux_packages <<<"${line%)}"
# A call naming its target and package literally; a second call, or one
# taking them from an expression, is a reading to update.
n="$(grep -c 'release_crate_notices\.sh' "$RELEASE" 2>/dev/null)"
(( n == 1 )) || { echo "$me: $RELEASE names release_crate_notices.sh on ${n:-0} lines, not one — update this check's reading with the release" >&2; exit 2; }
line="$(the_line "$RELEASE" "^ *bash tools/ci/release_crate_notices\\.sh $WORD $WORD \\\\\$")" || exit 2
read -r -a call <<<"$line"
apk_target="${call[2]}" apk_package="${call[3]}"

bad=0
# check <label> <target> <package...>
check() {
    local label="$1" target="$2" p out
    shift 2
    local tree=(tree --locked -e normal --target "$target" --prefix none -f '{p}')
    for p in "$@"; do tree+=(-p "$p"); done
    if ! cargo metadata --locked --format-version 1 --filter-platform "$target" >"$scratch/metadata.json" 2>"$scratch/err"; then
        echo "$me: cargo metadata for $target failed:" >&2; tail -5 "$scratch/err" >&2; exit 2
    fi
    if ! cargo "${tree[@]}" >"$scratch/compiled.txt" 2>"$scratch/err"; then
        echo "$me: cargo ${tree[*]} failed:" >&2; tail -5 "$scratch/err" >&2; exit 2
    fi
    # The tool's exit 1 is a refusal; anything else is not an answer.
    out="$(python3 tools/ci/release_notices.py "$scratch/metadata.json" "$scratch/compiled.txt" \
        "$scratch/notices.md" "$@" 2>&1)"
    case $? in
    0) echo "$me: $label ($target: $*): $out" ;;
    1) echo "FAIL: $label ($target: $*): $out"; bad=$((bad + 1)) ;;
    *) echo "$me: release_notices.py did not answer for $label: $out" >&2; exit 2 ;;
    esac
}
check "the Linux archive" "$linux_target" "${linux_packages[@]}"
check "the APK's library" "$apk_target" "$apk_package"

if (( bad )); then
    echo
    echo "The release would refuse these notices. \`tools/ci/release_notices.py --help\` says what each"
    echo "refusal asks for: an entry in tools/ci/notice_derived.json or tools/ci/notice_sources.json."
    exit 1
fi
exit 0
