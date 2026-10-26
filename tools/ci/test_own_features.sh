#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_own_features.sh
#
# Self-test for own_features.sh, with cargo faked: a package is selected
# exactly when its own feature set differs from the workspace build's
# (including a package whose own build has SOME of the workspace's
# features, and one cargo tree prints twice); each selected package gets a
# clippy of its own, never two -p in one; `test` is not a mode; a failure in one still
# runs the rest and exits 1 naming it; a cargo metadata or tree failure,
# or a bad mode, is exit 2.
set -uo pipefail
HERE="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
SUT="$HERE/own_features.sh"
PASS=0; FAIL=0
ok()  { PASS=$((PASS+1)); printf '  ok   %s\n' "$1"; }
bad() { FAIL=$((FAIL+1)); printf '  FAIL %s\n' "$1"; [[ -n "${2:-}" ]] && printf '%s\n' "$2" | sed 's/^/       /' | head -8; }

S="$(mktemp -d)"
[ -n "$S" ] && [ -d "$S" ] || { echo "test_own_features: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$S"' EXIT
mkdir -p "$S/repo"

# The fake cargo. metadata lists the members; `tree --workspace` and
# `tree -p <pkg>` print $S/tree-ws and $S/tree-<pkg> (as cargo tree's
# '{p}|{f}' format does, `(*)` repeats included); clippy records its argv
# in $FAKE_LOG and fails for the package in $FAKE_FAIL.
cat > "$S/cargo" <<'C'
#!/usr/bin/env bash
case "$1" in
  metadata) [ -n "${FAKE_META_FAIL:-}" ] && exit 101
            printf '{"packages":[{"name":"alpha"},{"name":"beta"},{"name":"gamma"},{"name":"delta"}]}\n' ;;
  tree) shift; sel=""
        while (( $# )); do case "$1" in --workspace) sel=ws ;; -p) sel="$2"; shift ;; esac; shift; done
        [ "$sel" = "${FAKE_TREE_FAIL:-}" ] && exit 101
        cat "$FAKE_DIR/tree-$sel" ;;
  clippy) echo "$*" >> "$FAKE_LOG"
        for a in "$@"; do [ "$a" = "${FAKE_FAIL:-}" ] && exit 101; done; exit 0 ;;
  *) exit 99 ;;
esac
C
chmod +x "$S/cargo"
# alpha: own build has no features, the workspace gives it `desktop`.
# beta:  the same set both ways (printed twice, once with (*)).
# gamma: own build has `a`, the workspace `a,b` -- a subset still differs.
# delta: two sets in the workspace (normal and build-dependency), one own.
cat > "$S/tree-ws" <<'T'
alpha v0.1.0 (/r/alpha)|desktop
beta v0.1.0 (/r/beta)|x
beta v0.1.0 (/r/beta)|x (*)
gamma v0.1.0 (/r/gamma)|a,b
delta v0.1.0 (/r/delta)|
delta v0.1.0 (/r/delta)|std
serde v1.0.0|derive,std
T
printf 'alpha v0.1.0 (/r/alpha)|\nserde v1.0.0|std\n' > "$S/tree-alpha"
printf 'beta v0.1.0 (/r/beta)|x\n' > "$S/tree-beta"
printf 'gamma v0.1.0 (/r/gamma)|a\n' > "$S/tree-gamma"
printf 'delta v0.1.0 (/r/delta)|std\n' > "$S/tree-delta"
run() { rm -f "$S/log"; env OWN_FEATURES_REPO="$S/repo" CARGO="$S/cargo" FAKE_DIR="$S" FAKE_LOG="$S/log" "$@" bash "$SUT" $MODE 2>&1; }

echo "own_features: which packages differ"
out="$(MODE=list run)"; rc=$?
[[ $rc -eq 0 ]] && ok "list: exit 0" || bad "list rc $rc" "$out"
grep -q '^own_features: alpha — workspace build \[desktop\], its own \[(none)\]$' <<<"$out" \
    && ok "alpha: desktop in the workspace, none on its own" || bad "alpha not named so" "$out"
grep -q 'beta' <<<"$out" && bad "beta, the same both ways, is selected" "$out" || ok "beta (same set, a (*) repeat) is not selected"
grep -q '^own_features: gamma — workspace build \[a,b\], its own \[a\]$' <<<"$out" \
    && ok "gamma: a subset of the workspace's set still differs" || bad "gamma not selected" "$out"
grep -q '^own_features: delta — workspace build \[(none) std\], its own \[std\]$' <<<"$out" \
    && ok "delta: a second set in the workspace build differs" || bad "delta not selected" "$out"
grep -q 'serde' <<<"$out" && bad "a dependency, not a member, is selected" "$out" || ok "a non-member is never selected"
[[ ! -e "$S/log" ]] && ok "list runs no clippy or test" || bad "list ran cargo" "$(cat "$S/log")"

echo "own_features: clippy, one package per command"
out="$(MODE=clippy run)"; rc=$?
[[ $rc -eq 0 ]] && ok "clippy: exit 0" || bad "clippy rc $rc" "$out"
want=$'clippy -p alpha --all-targets --locked -- -D warnings\nclippy -p delta --all-targets --locked -- -D warnings\nclippy -p gamma --all-targets --locked -- -D warnings'
[[ "$(cat "$S/log")" == "$want" ]] && ok "alpha, delta, gamma (by name): each its own clippy, --all-targets --locked -D warnings" \
    || bad "clippy argv" "$(cat "$S/log")"

echo "own_features: a failure runs the rest and is named"
out="$(MODE=clippy run FAKE_FAIL=alpha)"; rc=$?
[[ $rc -eq 1 ]] && ok "exit 1" || bad "rc $rc" "$out"
[[ $(wc -l < "$S/log") -eq 3 ]] && ok "gamma and delta still ran after alpha failed" || bad "stopped early" "$(cat "$S/log")"
grep -q 'clippy failed with each package.s own features: alpha$' <<<"$out" && ok "names alpha" || bad "failure not named" "$out"

echo "own_features: cargo and usage failures are exit 2"
out="$(MODE=clippy run FAKE_META_FAIL=1)"; rc=$?
[[ $rc -eq 2 ]] && grep -q 'cargo metadata failed' <<<"$out" && ok "cargo metadata failure: exit 2, named" || bad "metadata rc $rc" "$out"
out="$(MODE=clippy run FAKE_TREE_FAIL=ws)"; rc=$?
[[ $rc -eq 2 ]] && grep -q 'cargo tree --workspace failed' <<<"$out" && ok "cargo tree --workspace failure: exit 2" || bad "tree ws rc $rc" "$out"
out="$(MODE=clippy run FAKE_TREE_FAIL=gamma)"; rc=$?
[[ $rc -eq 2 ]] && grep -q 'cargo tree -p gamma failed' <<<"$out" && [[ ! -e "$S/log" ]] \
    && ok "cargo tree -p failure: exit 2 before any clippy" || bad "tree -p rc $rc" "$out"
out="$(MODE=lint run)"; rc=$?
[[ $rc -eq 2 ]] && ok "an unknown mode: exit 2" || bad "mode rc $rc" "$out"
out="$(MODE='test' run)"; rc=$?
[[ $rc -eq 2 && ! -e "$S/log" ]] && ok "test is not a mode (clippy only): exit 2" || bad "test mode rc $rc" "$out"
out="$(MODE='' run)"; rc=$?
[[ $rc -eq 2 ]] && ok "no mode: exit 2" || bad "no mode rc $rc" "$out"

echo "own_features: none differs"
cp "$S/tree-ws" "$S/tree-ws.keep"
printf 'alpha v0.1.0 (/r/alpha)|\nbeta v0.1.0 (/r/beta)|x\ngamma v0.1.0 (/r/gamma)|a\ndelta v0.1.0 (/r/delta)|std\n' > "$S/tree-ws"
out="$(MODE=clippy run)"; rc=$?
[[ $rc -eq 0 && ! -e "$S/log" ]] && grep -q 'nothing to run' <<<"$out" && ok "exit 0, says so, runs nothing" || bad "none-differs rc $rc" "$out"
mv "$S/tree-ws.keep" "$S/tree-ws"

echo
echo "test_own_features: $PASS passed, $FAIL failed"
(( FAIL == 0 ))
