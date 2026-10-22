#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/check_release_body.sh
#
# >>> help
# Refuse a release page the platform would refuse, before the release is
# created (ADR-0058 A 2026-10-22 (ii)):
#
#   tools/ci/check_release_body.sh <page.md>
#
# The page is the body `gh release create --notes-file` posts. Its length
# is counted in characters as decoded UTF-8 text, never in bytes, and held
# to RELEASE_BODY_LIMIT below. The check runs before gh release create, so
# an over-long page fails the run with its size and the limit named, rather
# than leaving a tag whose release the API refused.
#
# Exit codes:
#   0  within the limit; the length is printed
#   1  over the limit, or the page is not UTF-8 text
#   2  usage, or the page is missing
# <<< help

set -euo pipefail

# THE LIMIT, and where it comes from. GitHub documents no maximum for a
# release body; this figure is the Releases API's own refusal, an HTTP
# 422 reading "body is too long (maximum is 125000 characters)", as a
# public report quotes it (github.com/intuit/auto, issue 2418). Not
# verified against the platform when written (2026-10-22): the first
# tagged publish verifies it, and corrects this line if it differs.
RELEASE_BODY_LIMIT=125000

case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
esac
[[ $# -eq 1 ]] || { echo "usage: check_release_body.sh <page.md>" >&2; exit 2; }
page="$1"
[[ -f "$page" ]] || { echo "check_release_body: no page at $page" >&2; exit 2; }

python3 - "$page" "$RELEASE_BODY_LIMIT" <<'PY'
import sys
path, limit = sys.argv[1], int(sys.argv[2])
try:
    text = open(path, encoding="utf-8").read()
except UnicodeDecodeError as e:
    sys.exit(f"check_release_body: {path} is not UTF-8 text ({e})")
if len(text) > limit:
    sys.exit(f"check_release_body: {path} is {len(text)} characters, over the release-body "
             f"limit of {limit} (the API's 422, github.com/intuit/auto issue 2418; "
             f"see RELEASE_BODY_LIMIT in tools/ci/check_release_body.sh)")
print(f"check_release_body: {path} is {len(text)} characters, within {limit}")
PY
