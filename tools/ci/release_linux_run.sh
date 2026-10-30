#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_linux_run.sh
#
# >>> help
# Run every program of a Linux release archive, in the image of its glibc
# floor (the release run's linux-run job: a clean ubuntu:22.04, nothing
# installed):
#
#   tools/ci/release_linux_run.sh <archive.tar.gz>
#
# The archive is unpacked into scratch and each program started with
# LD_BIND_NOW=1, so every symbol is bound before main and a missing
# glibc, libgcc_s or libstdc++ version fails here rather than on first
# use -- what check_glibc_floor.py does not read (GCC_*, GLIBCXX_*) is
# covered by this run:
#   bin/transport-daemon --version, bin/transportctl --version,
#   bin/interweave-human --version, and the plugin's
#   claude-plugin/interweave/bin/claude-channel --version   exit 0;
#   bin/human-desktop, with no argument                     exit 64
#     (EX_USAGE: it has no --version, and printing its usage shows its
#     main ran).
# Each run is bounded at 30 seconds. A program that exits otherwise, or
# whose output names the dynamic loader's refusal, fails the run; every
# program is run and each failure named.
#
# Exit codes:
#   0  every program ran as above
#   1  one or more did not, or the archive holds no such program
#   2  usage, or the archive does not unpack
# <<< help

set -uo pipefail

case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
esac
[[ $# -eq 1 && -f "$1" ]] || { echo "usage: release_linux_run.sh <archive.tar.gz>" >&2; exit 2; }
archive="$1"

scratch="$(mktemp -d)" || { echo "release_linux_run: no scratch directory" >&2; exit 2; }
trap 'rm -rf "$scratch"' EXIT
tar -xzf "$archive" -C "$scratch" || { echo "release_linux_run: $archive does not unpack" >&2; exit 2; }
shopt -s nullglob
tops=("$scratch"/*/)
(( ${#tops[@]} == 1 )) || { echo "release_linux_run: $archive does not hold one directory" >&2; exit 2; }
top="${tops[0]%/}"

failures=0
# probe <path in the archive> <wanted exit> <argument>...
probe() {
    local rel="$1" want="$2" rc=0 out
    shift 2
    if [[ ! -x "$top/$rel" ]]; then
        echo "::error::release_linux_run: the archive holds no executable $rel"
        failures=$((failures + 1)); return
    fi
    out="$(LD_BIND_NOW=1 timeout 30 "$top/$rel" "$@" 2>&1 </dev/null)" || rc=$?
    if (( rc != want )) || grep -qE "version \`[A-Z_]+[0-9.]*' not found|error while loading shared libraries|symbol lookup error" <<<"$out"; then
        echo "::error::release_linux_run: $rel${*:+ $*} exited $rc, not $want: $(head -n 3 <<<"$out")"
        failures=$((failures + 1)); return
    fi
    echo "ran $rel${*:+ $*} (exit $rc)"
}
probe bin/transport-daemon 0 --version
probe bin/transportctl 0 --version
probe bin/interweave-human 0 --version
probe claude-plugin/interweave/bin/claude-channel 0 --version
probe bin/human-desktop 64

(( failures == 0 )) || { echo "release_linux_run: $failures program(s) did not run at this glibc ($(getconf GNU_LIBC_VERSION))" >&2; exit 1; }
echo "release_linux_run: every program ran at $(getconf GNU_LIBC_VERSION)"
