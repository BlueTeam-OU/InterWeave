#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_release_tag.sh
#
# Self-test for release_tag.sh, against a throwaway workspace cargo
# really reads, its version set to each case's: the specification's own
# pre-release examples are accepted and printed with prerelease=true, a
# plain version with prerelease=false; a build-metadata tag, a leading
# zero (in a number or a numeric identifier), an empty identifier and a
# tag without its v are refused before cargo is asked; a tag of another
# version than the workspace's -- a pre-release of it included -- is
# refused naming the workspace's; members of two versions are refused;
# a newline in a dispatched version is printed quoted.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/release_tag.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }
command -v cargo >/dev/null || { echo "test_release_tag: no cargo on PATH" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_release_tag: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

WS="$SANDBOX/ws"
# at <version>: the workspace's two members inherit <version>.
at() {
    mkdir -p "$WS/a/src" "$WS/b/src"
    printf '[workspace]\nmembers = ["a", "b"]\nresolver = "2"\n\n[workspace.package]\nversion = "%s"\nedition = "2021"\n' "$1" > "$WS/Cargo.toml"
    for m in a b; do
        printf '[package]\nname = "%s"\nversion.workspace = true\nedition.workspace = true\n' "$m" > "$WS/$m/Cargo.toml"
        : > "$WS/$m/src/lib.rs"
    done
}
run() { bash "$UNDER_TEST" "$@" --root "$WS" >"$SANDBOX/out" 2>&1; }

echo "release_tag.sh:"
# Accepted, each at a workspace of that version.
for v in 0.1.0 1.0.0 10.20.30 \
         0.4.0-rc.1 1.0.0-alpha 1.0.0-alpha.1 1.0.0-0.3.7 1.0.0-x.7.z.92 1.0.0-x-y-z.--- 1.0.0-0a 1.0.0-alpha.beta; do
    at "$v"
    if [[ "$v" == *-* ]]; then want=true; else want=false; fi
    if run "v$v" && [[ "$(cat "$SANDBOX/out")" == "version=$v"$'\n'"prerelease=$want" ]]; then
        pass "v$v is accepted at workspace $v, prerelease=$want"
    else
        fail "v$v at workspace $v" "$(cat "$SANDBOX/out")"
    fi
done

# Refused by the grammar. The workspace is a version cargo takes, and
# the message must be the grammar's, not the workspace comparison's.
at 1.0.0
for t in v1.0.0+build.1 v1.0.0-rc.1+build.1 v01.0.0 v1.00.0 v1.0.00 v1.0.0-rc.01 v1.0.0-00 \
         v1.0.0- v1.0.0-rc..1 v1.0.0-rc. v1.0.0-rc_1 'v1.0.0-é' 1.0.0 V1.0.0 v1.0 v1.0.0.0 'v 1.0.0' ''; do
    run "$t"; status=$?
    if [[ $status == 1 ]] && grep -qF "is not v<MAJOR>.<MINOR>.<PATCH>[-<pre-release>]" "$SANDBOX/out"; then
        pass "$(printf '%q' "$t") is refused by the grammar"
    else
        fail "$(printf '%q' "$t"): exit $status" "$(cat "$SANDBOX/out")"
    fi
done

# Refused: a version that is not the workspace's.
at 0.4.0-rc.1
for t in v0.4.0 v0.4.0-rc.2 v0.3.9; do
    run "$t"; status=$?
    if [[ $status == 1 ]] && grep -qF "tag $t is not the workspace version 0.4.0-rc.1" "$SANDBOX/out" \
        && ! grep -q '^version=' "$SANDBOX/out"; then
        pass "$t is refused at workspace 0.4.0-rc.1, and no version is printed"
    else
        fail "$t at workspace 0.4.0-rc.1: exit $status" "$(cat "$SANDBOX/out")"
    fi
done

at 0.1.0
sed -i 's/^version\.workspace = true$/version = "0.2.0"/' "$WS/b/Cargo.toml"
run v0.1.0; status=$?
[[ $status == 1 ]] && grep -qF "carry more than one version (0.1.0, 0.2.0)" "$SANDBOX/out" \
    && pass "members of two versions are refused, both named" \
    || fail "members of two versions: exit $status" "$(cat "$SANDBOX/out")"

at 0.1.0
run $'v0.1.0\n::warning::injected'; status=$?
[[ $status == 1 ]] && [[ "$(grep -c . "$SANDBOX/out")" == 1 ]] && grep -qF "\$'v0.1.0\\n::warning::injected'" "$SANDBOX/out" \
    && pass "a newline in a tag is refused and printed %q-quoted on one line" \
    || fail "a tag with a newline: exit $status" "$(cat "$SANDBOX/out")"

printf 'not toml [\n' >> "$WS/Cargo.toml"
run v0.1.0; status=$?
[[ $status == 2 ]] && grep -qF "cargo metadata failed" "$SANDBOX/out" \
    && pass "a manifest cargo cannot read is exit 2" || fail "an unreadable manifest: exit $status" "$(cat "$SANDBOX/out")"

bash "$UNDER_TEST" >/dev/null 2>&1; status=$?
[[ $status == 2 ]] && pass "no argument is a usage error" || fail "no argument: exit $status"
bash "$UNDER_TEST" v0.1.0 --rot "$WS" >/dev/null 2>&1; status=$?
[[ $status == 2 ]] && pass "a misspelt --root is a usage error" || fail "a misspelt --root: exit $status"
help_out="$(bash "$UNDER_TEST" --help 2>/dev/null)"
[[ "$help_out" == *"ADR-0065 decisions 5 and 6"* ]] && pass "--help prints the help block" || fail "--help" "$help_out"

echo
if (( failures )); then
    echo "test_release_tag: $failures assertion(s) failed" >&2
    exit 1
fi
echo "test_release_tag: all assertions passed"
