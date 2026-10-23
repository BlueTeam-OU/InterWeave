#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/test_check_workspace_version.sh
#
# Self-test for check_workspace_version.sh, against throwaway workspaces
# cargo really reads: two members inheriting the workspace version pass;
# a member with its own literal -- equal to the workspace's, so only the
# second source is wrong -- is refused and named, as is a member with no
# version and a member with a different one; a workspace with no version,
# or one with build metadata, is refused; a crate that is not a member
# (an excluded directory, as the spikes are) is not this check's; a
# manifest cargo cannot read and a stray argument are exit 2.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/check_workspace_version.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }
command -v cargo >/dev/null || { echo "test_check_workspace_version: no cargo on PATH" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_check_workspace_version: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# workspace <dir> <workspace.package version line, or empty>: a workspace
# of members a and b, both inheriting, and an excluded crate `spike`
# carrying its own version as a spike harness does.
workspace() {
    local dir="$1" version_line="$2" m
    mkdir -p "$dir"
    printf '[workspace]\nmembers = ["a", "b"]\nexclude = ["spike"]\nresolver = "2"\n\n[workspace.package]\n%s\nedition = "2021"\n' \
        "$version_line" > "$dir/Cargo.toml"
    for m in a b; do
        mkdir -p "$dir/$m/src"
        printf '[package]\nname = "%s"\nversion.workspace = true\nedition.workspace = true\n' "$m" > "$dir/$m/Cargo.toml"
        : > "$dir/$m/src/lib.rs"
    done
    mkdir -p "$dir/spike/src"
    printf '[package]\nname = "spike"\nversion = "0.0.0"\nedition = "2021"\n\n[workspace]\n' > "$dir/spike/Cargo.toml"
    : > "$dir/spike/src/lib.rs"
}
# member_version <dir> <member> <line>: replace that member's version line.
member_version() { sed -i "s/^version\.workspace = true\$/$3/" "$1/$2/Cargo.toml"; }
run() { bash "$UNDER_TEST" "$@" >"$SANDBOX/out" 2>&1; }

echo "check_workspace_version.sh:"
workspace "$SANDBOX/ok" 'version = "0.4.0-rc.1"'
run --root "$SANDBOX/ok" && grep -qF "2 workspace members inherit version 0.4.0-rc.1" "$SANDBOX/out" \
    && pass "two inheriting members pass, a pre-release workspace version included; the excluded crate is not counted" \
    || fail "an inheriting workspace was refused" "$(cat "$SANDBOX/out")"

# The mutation the guard exists for: a crate's own version, equal today.
workspace "$SANDBOX/own" 'version = "0.1.0"'
member_version "$SANDBOX/own" b 'version = "0.1.0"'
run --root "$SANDBOX/own"; status=$?
[[ $status == 1 ]] && grep -qF "$SANDBOX/own/b/Cargo.toml: version is '0.1.0', not version.workspace = true" "$SANDBOX/out" \
    && ! grep -qF "/a/Cargo.toml" "$SANDBOX/out" \
    && pass "a member with its own version, equal to the workspace's, is refused (exit 1) and only it is named" \
    || fail "a member's own equal version: exit $status" "$(cat "$SANDBOX/out")"

workspace "$SANDBOX/differs" 'version = "0.1.0"'
member_version "$SANDBOX/differs" a 'version = "0.2.0"'
run --root "$SANDBOX/differs"; status=$?
[[ $status == 1 ]] && grep -qF "/a/Cargo.toml: version is '0.2.0'" "$SANDBOX/out" \
    && pass "a member with a different version is refused" \
    || fail "a member's different version: exit $status" "$(cat "$SANDBOX/out")"

workspace "$SANDBOX/omitted" 'version = "0.1.0"'
sed -i '/^version\.workspace = true$/d' "$SANDBOX/omitted/a/Cargo.toml"
run --root "$SANDBOX/omitted"; status=$?
[[ $status == 1 ]] && grep -qF "/a/Cargo.toml: version is None" "$SANDBOX/out" \
    && pass "a member with no version (cargo's 0.0.0) is refused" \
    || fail "a member with no version: exit $status" "$(cat "$SANDBOX/out")"

# No workspace version: the members cannot inherit one, so cargo itself
# refuses the manifest -- exit 2, cargo's message on stderr. With the
# members' own literals there is something to read, and the guard names
# the missing workspace version.
workspace "$SANDBOX/none" ''
run --root "$SANDBOX/none"; status=$?
[[ $status == 2 ]] && grep -qF "cargo metadata failed" "$SANDBOX/out" \
    && pass "members inheriting a version the workspace lacks: cargo refuses, exit 2" \
    || fail "inheriting a missing version: exit $status" "$(cat "$SANDBOX/out")"
member_version "$SANDBOX/none" a 'version = "0.1.0"'
member_version "$SANDBOX/none" b 'version = "0.1.0"'
run --root "$SANDBOX/none"; status=$?
[[ $status == 1 ]] && grep -qF "has no [workspace.package] version" "$SANDBOX/out" \
    && pass "a workspace with no version is refused, exit 1" \
    || fail "no workspace version: exit $status" "$(cat "$SANDBOX/out")"

workspace "$SANDBOX/build" 'version = "0.1.0+abc123"'
run --root "$SANDBOX/build"; status=$?
[[ $status == 1 ]] && grep -qF "carries build metadata" "$SANDBOX/out" \
    && pass "a workspace version with build metadata is refused" \
    || fail "build metadata: exit $status" "$(cat "$SANDBOX/out")"

workspace "$SANDBOX/broken" 'version = "0.1.0"'
printf 'not toml [\n' >> "$SANDBOX/broken/Cargo.toml"
run --root "$SANDBOX/broken"; status=$?
[[ $status == 2 ]] && pass "a manifest cargo cannot read is exit 2" || fail "an unreadable manifest: exit $status" "$(cat "$SANDBOX/out")"

run --bogus; status=$?
[[ $status == 2 ]] && pass "a stray argument is a usage error" || fail "a stray argument: exit $status"
help_out="$(bash "$UNDER_TEST" --help 2>/dev/null)"
[[ "$help_out" == *"ADR-0065 decision 5"* ]] && pass "--help prints the help block" || fail "--help" "$help_out"

echo
if (( failures )); then
    echo "test_check_workspace_version: $failures assertion(s) failed" >&2
    exit 1
fi
echo "test_check_workspace_version: all assertions passed"
