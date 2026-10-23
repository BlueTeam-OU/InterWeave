#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_release_publish.sh
#
# Self-test for release_publish.sh, with gh mocked on PATH to record its
# arguments: a pre-release passes --prerelease and --latest=false, a
# release passes neither; both pass --verify-tag, the title, the page and
# the six files; a prerelease value other than true or false is refused
# before gh runs, printed quoted; a failing gh fails the step; a missing
# argument is a usage error.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/release_publish.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_release_publish: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# gh: one argument per line into $SANDBOX/argv, exit $GH_EXIT.
mkdir -p "$SANDBOX/bin" "$SANDBOX/in"
cat > "$SANDBOX/bin/gh" <<'GH'
#!/usr/bin/env bash
printf '%s\n' "$@" > "$GH_ARGV"
exit "${GH_EXIT:-0}"
GH
chmod +x "$SANDBOX/bin/gh"
V=0.4.0-rc.1
for f in "interweave-$V-arm64-v8a.apk" "interweave-$V-arm64-v8a.apk.sha256" \
         "interweave-$V-x86_64-unknown-linux-gnu.tar.gz" "interweave-$V-x86_64-unknown-linux-gnu.tar.gz.sha256" \
         THIRD-PARTY-NOTICES-android.md THIRD-PARTY-NOTICES-android.md.sha256; do
    : > "$SANDBOX/in/$f"
done
: > "$SANDBOX/page.md"
export GH_ARGV="$SANDBOX/argv"
run() { rm -f "$GH_ARGV"; PATH="$SANDBOX/bin:$PATH" bash "$UNDER_TEST" "$@" >"$SANDBOX/out" 2>&1; }
has() { grep -qxF -- "$1" "$GH_ARGV"; }

echo "release_publish.sh:"
run "v$V" "$V" true "$SANDBOX/page.md" "$SANDBOX/in"; status=$?
if [[ $status == 0 ]] && has --prerelease && has --latest=false; then
    pass "a pre-release is created with --prerelease and --latest=false"
else
    fail "a pre-release: exit $status" "$(cat "$SANDBOX/out" "$GH_ARGV" 2>/dev/null)"
fi
expected="$(printf '%s\n' release create "v$V" --verify-tag --prerelease --latest=false --title "InterWeave $V" \
    --notes-file "$SANDBOX/page.md" \
    "$SANDBOX/in/interweave-$V-arm64-v8a.apk" "$SANDBOX/in/interweave-$V-arm64-v8a.apk.sha256" \
    "$SANDBOX/in/interweave-$V-x86_64-unknown-linux-gnu.tar.gz" "$SANDBOX/in/interweave-$V-x86_64-unknown-linux-gnu.tar.gz.sha256" \
    "$SANDBOX/in/THIRD-PARTY-NOTICES-android.md" "$SANDBOX/in/THIRD-PARTY-NOTICES-android.md.sha256")"
[[ "$(cat "$GH_ARGV")" == "$expected" ]] \
    && pass "the whole command: the tag verified, the title, the page and the six files" \
    || fail "the command differs" "$(diff <(printf '%s\n' "$expected") "$GH_ARGV")"

run v0.4.0 0.4.0 false "$SANDBOX/page.md" "$SANDBOX/in"; status=$?
if [[ $status == 0 ]] && ! grep -q -- '--prerelease\|--latest' "$GH_ARGV" && has --verify-tag && has "InterWeave 0.4.0"; then
    pass "a release is created with neither --prerelease nor --latest"
else
    fail "a release: exit $status" "$(cat "$SANDBOX/out" "$GH_ARGV" 2>/dev/null)"
fi

for value in maybe True '' $'true\n::warning::x'; do
    run "v$V" "$V" "$value" "$SANDBOX/page.md" "$SANDBOX/in"; status=$?
    if [[ $status == 1 ]] && [[ ! -e "$GH_ARGV" ]] && [[ "$(grep -c . "$SANDBOX/out")" == 1 ]] \
        && grep -qF "prerelease is $(printf '%q' "$value"), neither true nor false" "$SANDBOX/out"; then
        pass "prerelease $(printf '%q' "$value") is refused before gh runs, quoted on one line"
    else
        fail "prerelease $(printf '%q' "$value"): exit $status" "$(cat "$SANDBOX/out")"
    fi
done

GH_EXIT=1 run "v$V" "$V" true "$SANDBOX/page.md" "$SANDBOX/in"; status=$?
[[ $status != 0 ]] && pass "a failing gh fails the step" || fail "a failing gh: exit $status"

run "v$V" "$V" true "$SANDBOX/page.md"; status=$?
[[ $status == 2 ]] && pass "a missing argument is a usage error" || fail "a missing argument: exit $status"
help_out="$(bash "$UNDER_TEST" --help 2>/dev/null)"
[[ "$help_out" == *"ADR-0065 decision 6"* ]] && pass "--help prints the help block" || fail "--help" "$help_out"

echo
if (( failures )); then
    echo "test_release_publish: $failures assertion(s) failed" >&2
    exit 1
fi
echo "test_release_publish: all assertions passed"
