#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/pip_install.sh
#
# >>> help
# Install a CI job's hash-pinned Python requirements so a stalled
# download costs a retry rather than the job.
#
#   tools/ci/pip_install.sh <requirements>
#
# Runs `python3 -m pip install --require-hashes --progress-bar off
# --timeout 30 --disable-pip-version-check -r <requirements>` up to
# three times, each attempt stopped by `timeout` after
# PIP_INSTALL_ATTEMPT_SECONDS (default 120;
# the install takes about 3 s from a healthy index). pip's own --timeout
# drops a connection that stops answering; a download that keeps
# trickling bytes is one it never cuts, and that is what the attempt
# limit is for: merge-group run 41862961434 (2026-11-04) spent 9 min 35 s
# in a --quiet install that takes 2.5 s on the same image, printed
# nothing, and the tool self-tests job then hit its 15-minute timeout.
# So pip is not --quiet here: the log names the download it stalled on
# (and --disable-pip-version-check keeps pip's upgrade notice out of it).
#
# Every attempt is retried on failure, a refused hash included: pip
# exits 1 for a network error after its own retries as for a mismatch,
# and a mismatch repeated costs seconds.
#
# Exit codes:
#   0  installed
#   1  every attempt failed (each one's status is printed)
#   2  usage: no requirements file, a limit that is not a positive integer
# <<< help

set -euo pipefail

die() { echo "pip_install: $1" >&2; exit "${2:-2}"; }

ATTEMPTS=3

case "${1:-}" in
    -h|--help) sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'; exit 0 ;;
esac

[[ $# -eq 1 ]] || die "usage: $0 <requirements> (try --help)"
req="$1"
[[ -f "$req" ]] || die "$req is not a file"
limit="${PIP_INSTALL_ATTEMPT_SECONDS:-120}"
[[ "$limit" =~ ^[1-9][0-9]*$ ]] || die "PIP_INSTALL_ATTEMPT_SECONDS is not a positive integer: $limit"

for ((n = 1; n <= ATTEMPTS; n++)); do
    start="$SECONDS"
    status=0
    # --kill-after: a pip that ignores TERM mid-download must not outlive
    # the attempt that was meant to bound it.
    timeout --kill-after=10 "$limit" \
        python3 -m pip install --require-hashes --progress-bar off --timeout 30 \
        --disable-pip-version-check -r "$req" \
        || status=$?
    if [[ "$status" -eq 0 ]]; then
        echo "pip_install: installed on attempt $n of $ATTEMPTS ($((SECONDS - start)) s)"
        exit 0
    fi
    if [[ "$status" -eq 124 || "$status" -eq 137 ]]; then
        echo "pip_install: attempt $n of $ATTEMPTS stopped after ${limit} s (status $status)" >&2
    else
        echo "pip_install: attempt $n of $ATTEMPTS failed (status $status, $((SECONDS - start)) s)" >&2
    fi
done
die "every attempt failed" 1
