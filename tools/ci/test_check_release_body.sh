#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_check_release_body.sh
#
# Self-test for check_release_body.sh: a page of exactly the limit
# passes and one character more is refused, naming where the limit came
# from; the count is characters, not bytes (a page of two-byte characters
# at the limit passes); a page that is not UTF-8, a missing page and a
# missing argument are refused. The limit is read from the script, so the
# cases follow the constant when it is corrected.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/check_release_body.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_check_release_body: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

LIMIT="$(sed -n 's/^RELEASE_BODY_LIMIT=\([0-9][0-9]*\)$/\1/p' "$UNDER_TEST")"
[[ "$LIMIT" =~ ^[0-9]+$ ]] || { echo "test: no RELEASE_BODY_LIMIT=<n> line in $UNDER_TEST" >&2; exit 1; }

# page <file> <count> <char>: a page of <count> copies of <char>.
page() { python3 -c 'import sys; open(sys.argv[1], "w", encoding="utf-8").write(sys.argv[3] * int(sys.argv[2]))' "$1" "$2" "$3"; }
run() { bash "$UNDER_TEST" "$@" >"$SANDBOX/out" 2>&1; }

echo "check_release_body.sh (limit $LIMIT):"
page "$SANDBOX/at.md" "$LIMIT" a
run "$SANDBOX/at.md" && grep -qF "is $LIMIT characters, within $LIMIT" "$SANDBOX/out" \
    && pass "a page of exactly the limit passes, its length printed" \
    || fail "a page of exactly the limit was refused" "$(cat "$SANDBOX/out")"

page "$SANDBOX/over.md" "$((LIMIT + 1))" a
run "$SANDBOX/over.md"; status=$?
if [[ $status == 1 ]] && grep -qF "is $((LIMIT + 1)) characters, over the release-body limit of $LIMIT" "$SANDBOX/out" \
    && grep -qF "github.com/intuit/auto issue 2418" "$SANDBOX/out"; then
    pass "one character over the limit is refused, exit 1, naming the limit and where it came from"
else
    fail "one character over: exit $status" "$(cat "$SANDBOX/out")"
fi

page "$SANDBOX/wide.md" "$LIMIT" é
(( $(wc -c < "$SANDBOX/wide.md") > LIMIT )) || fail "the two-byte page is not over the limit in bytes"
run "$SANDBOX/wide.md" \
    && pass "the count is characters, not bytes: $LIMIT two-byte characters pass" \
    || fail "a page of $LIMIT two-byte characters was refused" "$(cat "$SANDBOX/out")"

# CRLF: each line ending counts as the two characters the body carries.
python3 -c 'import sys; open(sys.argv[1], "w", encoding="utf-8", newline="").write("a\r\n" * (int(sys.argv[2]) // 3 + 1))' "$SANDBOX/crlf.md" "$LIMIT"
run "$SANDBOX/crlf.md"; status=$?
[[ $status == 1 ]] && grep -qF "over the release-body limit" "$SANDBOX/out" \
    && pass "a CRLF counts as two characters: a page over the limit only with its CRs counted is refused" \
    || fail "a CRLF page over the limit: exit $status" "$(cat "$SANDBOX/out")"

printf 'notes \xff\xfe\n' > "$SANDBOX/binary.md"
run "$SANDBOX/binary.md"; status=$?
[[ $status == 1 ]] && grep -qF "is not UTF-8 text" "$SANDBOX/out" \
    && pass "a page that is not UTF-8 is refused, exit 1" \
    || fail "a non-UTF-8 page: exit $status" "$(cat "$SANDBOX/out")"

run "$SANDBOX/no-such.md"; status=$?
[[ $status == 2 ]] && pass "a missing page is exit 2" || fail "a missing page: exit $status" "$(cat "$SANDBOX/out")"
run; status=$?
[[ $status == 2 ]] && pass "no argument is a usage error" || fail "no argument: exit $status"
help_out="$(bash "$UNDER_TEST" --help 2>/dev/null)"
[[ "$help_out" == *"ADR-0058 A 2026-10-22 (ii)"* ]] && pass "--help prints the help block" || fail "--help" "$help_out"

echo
if (( failures )); then
    echo "test_check_release_body: $failures failure(s)" >&2
    exit 1
fi
echo "test_check_release_body: all passed"
