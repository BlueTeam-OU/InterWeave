#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_prune_cargo_target.sh
#
# Self-test for prune_cargo_target.sh.
#
# cargo is a stub on PATH whose `metadata` answers a two-member workspace
# (interweave-wire with test targets relay_client and p2p, and the app
# transport-daemon), or fails, or names nothing, as a file in the sandbox
# says. Each case lays out a target directory of real shapes — what a
# `cargo test --workspace` leaves — and checks what survives: every
# dependency's build, none of the workspace's.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/prune_cargo_target.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(realpath -- "$(mktemp -d)")"
[ -n "$SANDBOX" ] && [ -d "$SANDBOX" ] || { echo "test_prune_cargo_target: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
BIN="$SANDBOX/bin"
mkdir -p "$BIN"

pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

cat > "$BIN/cargo" <<EOF
#!/usr/bin/env bash
echo "cargo \$*" >> "$SANDBOX/log"
[[ "\$1" == metadata ]] || exit 99
[[ -e "$SANDBOX/metadata-fails" ]] && exit 101
if [[ -e "$SANDBOX/metadata-empty" ]]; then echo '{"packages":[]}'; exit 0; fi
cat <<'JSON'
{"packages":[
 {"name":"interweave-wire","targets":[{"name":"interweave_wire"},{"name":"relay_client"},{"name":"p2p"}]},
 {"name":"transport-daemon","targets":[{"name":"transport-daemon"}]}
]}
JSON
EOF
chmod +x "$BIN/cargo"

H=0123456789abcdef
H2=fedcba9876543210

# A target directory as a workspace test build leaves it.
layout() {
    local t="$1" p
    for p in "$t/debug" "$t/x86_64-unknown-linux-gnu/debug"; do
        mkdir -p "$p/deps" "$p/.fingerprint" "$p/build" "$p/incremental/x" "$p/examples"
        # Dependencies: kept.
        touch "$p/deps/libtokio-$H.rlib" "$p/deps/libtokio-$H.rmeta" "$p/deps/tokio-$H.d"
        touch "$p/deps/liblibp2p-$H.rlib" "$p/deps/libp2p-$H.d"
        touch "$p/deps/libserde_derive-$H.so"
        mkdir -p "$p/.fingerprint/tokio-$H" "$p/.fingerprint/libp2p-$H"
        mkdir -p "$p/build/ring-$H/out" "$p/build/ring-$H2"
        # The workspace: removed.
        touch "$p/deps/libinterweave_wire-$H.rlib" "$p/deps/libinterweave_wire-$H.rmeta"
        touch "$p/deps/interweave_wire-$H.d" "$p/deps/interweave_wire-$H2"
        touch "$p/deps/relay_client-$H" "$p/deps/relay_client-$H.d"
        touch "$p/deps/transport_daemon-$H"
        # A workspace proc-macro (.so) and staticlib (.a) drop the prefix.
        touch "$p/deps/libinterweave_wire-$H2.so" "$p/deps/libinterweave_wire-$H2.a"
        # The test target p2p, beside the crate libp2p: only a library's
        # own files drop the lib prefix, so libp2p-<hash>.d stays.
        touch "$p/deps/p2p-$H" "$p/deps/p2p-$H.d"
        mkdir -p "$p/.fingerprint/interweave-wire-$H" "$p/.fingerprint/transport-daemon-$H2"
        mkdir -p "$p/build/interweave-wire-$H/out"
        touch "$p/transport-daemon" "$p/transport-daemon.d" "$p/.cargo-lock"
    done
    mkdir -p "$t/tmp/scratch"
    touch "$t/CACHEDIR.TAG" "$t/.rustc_info.json"
}

# Every path under the target, relative and sorted.
inventory() { (cd "$1" && find . -mindepth 1 | sed 's#^\./##' | sort); }

run() { (PATH="$BIN:$PATH" bash "$UNDER_TEST" "$@" 2>&1); }

echo "== a workspace test build: the dependencies stay, the workspace goes"
T="$SANDBOX/t1"; layout "$T"
out="$(run "$T")"; rc=$?
[[ $rc -eq 0 ]] && pass "exit 0" || fail "exit $rc, expected 0" "$out"
got="$(inventory "$T")"
want="$(for p in debug x86_64-unknown-linux-gnu/debug; do
    printf '%s\n' "$p" "$p/deps" "$p/.fingerprint" "$p/build" \
        "$p/deps/libtokio-$H.rlib" "$p/deps/libtokio-$H.rmeta" "$p/deps/tokio-$H.d" \
        "$p/deps/liblibp2p-$H.rlib" "$p/deps/libp2p-$H.d" "$p/deps/libserde_derive-$H.so" \
        "$p/.fingerprint/tokio-$H" "$p/.fingerprint/libp2p-$H" \
        "$p/build/ring-$H" "$p/build/ring-$H/out" "$p/build/ring-$H2"
    done
    printf '%s\n' x86_64-unknown-linux-gnu CACHEDIR.TAG .rustc_info.json)"
want="$(sort <<<"$want")"
[[ "$got" == "$want" ]] && pass "exactly the dependencies' builds survive, under both profile layouts" \
    || fail "the surviving set differs" "$(diff <(echo "$want") <(echo "$got"))"
grep -q '^prune_cargo_target: .* -> .*, [0-9][0-9]* entries removed$' <<<"$out" \
    && pass "the sizes before and after are printed" || fail "no size line" "$out"
grep -qx "cargo metadata --no-deps --format-version 1 --locked" "$SANDBOX/log" \
    && pass "the names come from cargo metadata --no-deps --locked" || fail "cargo was not asked as expected" "$(cat "$SANDBOX/log")"

echo "== a second run finds nothing more to remove"
out="$(run "$T")"; rc=$?
[[ $rc -eq 0 ]] && grep -q ', 0 entries removed$' <<<"$out" && [[ "$(inventory "$T")" == "$want" ]] \
    && pass "idempotent" || fail "the second run changed something (rc $rc)" "$out"

echo "== cargo metadata fails: nothing is removed"
T="$SANDBOX/t2"; layout "$T"; snapshot="$(inventory "$T")"
touch "$SANDBOX/metadata-fails"
out="$(run "$T")"; rc=$?
rm -f "$SANDBOX/metadata-fails"
[[ $rc -eq 1 ]] && pass "exit 1" || fail "exit $rc, expected 1" "$out"
[[ "$(inventory "$T")" == "$snapshot" ]] && pass "the target is untouched" || fail "a failed metadata still pruned"

echo "== cargo metadata names no member: nothing is removed"
touch "$SANDBOX/metadata-empty"
out="$(run "$T")"; rc=$?
rm -f "$SANDBOX/metadata-empty"
[[ $rc -eq 1 ]] && grep -q 'named no workspace member' <<<"$out" && pass "exit 1, named" || fail "exit $rc, expected 1 named" "$out"
[[ "$(inventory "$T")" == "$snapshot" ]] && pass "the target is untouched" || fail "an empty workspace still pruned"

echo "== usage"
out="$(run)"; rc=$?
[[ $rc -eq 2 ]] && pass "no argument: exit 2" || fail "no argument: exit $rc" "$out"
out="$(run "$SANDBOX/nowhere")"; rc=$?
[[ $rc -eq 2 ]] && pass "a missing directory: exit 2" || fail "a missing directory: exit $rc" "$out"
out="$(run --help)"; rc=$?
[[ $rc -eq 0 ]] && grep -q 'tools/ci/prune_cargo_target.sh <target-dir>' <<<"$out" \
    && pass "--help prints the usage" || fail "--help: exit $rc" "$out"

echo
if [[ $failures -eq 0 ]]; then echo "all passed"; exit 0; fi
echo "$failures failed" >&2
exit 1
