#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_commit_stamp.sh
#
# Self-test for tools/stamp/commit_stamp.rs, the build script that stamps
# a program's --version with the commit it was built from (ADR-0065
# decision 6). It lives here rather than beside the script because CI's
# tool self-tests and `cargo xtask selftests` read test_*.sh from
# tools/{checks,gh,ci,host/android} only.
#
# A throwaway package in a throwaway git repository takes the script as
# its build script and prints the stamp. An INCREMENTAL build after each
# move of HEAD must print the new commit: a commit with reflogs on; a
# commit on a packed branch with reflogs off and no loose ref yet (the
# case #386's review found, where only HEAD and packed-refs were
# watched and neither changes); a reset back. Built from no checkout,
# the stamp is `unknown`.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/../stamp/commit_stamp.rs"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }
command -v cargo >/dev/null || { echo "test_commit_stamp: no cargo on PATH" >&2; exit 1; }
command -v git >/dev/null || { echo "test_commit_stamp: no git on PATH" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_commit_stamp: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# package <dir>: a package whose build script is the one under test.
package() {
    mkdir -p "$1/src"
    cp "$UNDER_TEST" "$1/stamp.rs"
    printf '[package]\nname = "stampcase"\nversion = "0.1.0"\nedition = "2024"\nbuild = "stamp.rs"\n\n[workspace]\n' \
        > "$1/Cargo.toml"
    printf 'fn main() { println!("{}", env!("INTERWEAVE_COMMIT")); }\n' > "$1/src/main.rs"
    printf 'target\n' > "$1/.gitignore"
}
# stamp <dir>: build incrementally, print what the binary says.
stamp() {
    CARGO_TARGET_DIR="$1/target" cargo build -q --offline --manifest-path "$1/Cargo.toml" \
        >"$SANDBOX/build.log" 2>&1 || { cat "$SANDBOX/build.log" >&2; echo "build failed"; return; }
    "$1/target/debug/stampcase"
}
head12() { git -C "$1" rev-parse HEAD | cut -c1-12; }
repo() {
    git -C "$1" init -q -b main
    git -C "$1" config user.email test@example.invalid
    git -C "$1" config user.name test
    git -C "$1" config commit.gpgsign false
}

echo "commit_stamp.rs:"

R="$SANDBOX/reflogs"
package "$R"; repo "$R"
git -C "$R" add -A && git -C "$R" commit -q -m one
first="$(stamp "$R")"
[[ "$first" == "$(head12 "$R")" ]] && pass "the stamp is HEAD's first twelve hex digits" \
    || fail "the first build stamped $first, not $(head12 "$R")"
git -C "$R" commit -q --allow-empty -m two
[[ "$(stamp "$R")" == "$(head12 "$R")" ]] && pass "a commit restamps an incremental build" \
    || fail "after a commit the stamp stayed $first"
git -C "$R" reset -q --hard HEAD~1
[[ "$(stamp "$R")" == "$first" ]] && pass "a reset back restamps it" \
    || fail "after a reset the stamp is not $first"

P="$SANDBOX/packed"
package "$P"; repo "$P"
git -C "$P" config core.logAllRefUpdates false
git -C "$P" add -A && git -C "$P" commit -q -m one
rm -rf "$P/.git/logs"
git -C "$P" pack-refs --all
if [[ -e "$P/.git/refs/heads/main" || -e "$P/.git/logs" ]]; then
    fail "the control: the branch is not packed, or a reflog is left" "$(ls -R "$P/.git/refs" "$P/.git/logs" 2>&1)"
fi
first="$(stamp "$P")"
git -C "$P" commit -q --allow-empty -m two
[[ "$(stamp "$P")" == "$(head12 "$P")" ]] \
    && pass "a commit on a packed branch with reflogs off restamps an incremental build" \
    || fail "with reflogs off and refs packed, the stamp stayed $first after a commit"

N="$SANDBOX/no-checkout"
package "$N"
# The ceiling keeps git from finding a repository above the sandbox.
stamp_out="$(GIT_CEILING_DIRECTORIES="$SANDBOX" stamp "$N")"
[[ "$stamp_out" == unknown ]] && pass "built from no checkout, the stamp is unknown" \
    || fail "built from no checkout, the stamp is $stamp_out"

echo
if (( failures )); then
    echo "test_commit_stamp: $failures assertion(s) failed" >&2
    exit 1
fi
echo "test_commit_stamp: all assertions passed"
