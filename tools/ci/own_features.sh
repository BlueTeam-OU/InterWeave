#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/own_features.sh
#
# Clippy and the tests for every workspace package that the workspace build
# compiles with features its OWN build does not have.
#
#   bash tools/ci/own_features.sh list|clippy|test
#
# `cargo clippy --workspace` and `cargo nextest run --workspace` unify
# features across every member: apps/human-desktop depends on
# interweave-human-ui-slint with `desktop`, so the workspace build never
# compiles ui-slint without it, and code that exists only without a feature
# -- a `#[cfg(not(feature = ...))]` branch, a helper only the featureless
# tests use, a `cfg_attr(not(feature), expect(dead_code))` -- is never
# linted or run. `cargo clippy -p <package>` alone, which is what a
# downstream crate or a person's `cargo test -p` builds, can then fail on a
# tree CI called green (twice in ui-slint before this step existed).
#
# Which packages: those whose own feature set (`cargo tree -p <package>`,
# dev-dependencies included, as --all-targets builds them) differs from the
# set the workspace build gives them (`cargo tree --workspace`). Computed on
# every run rather than listed, so a package that gains a feature, or a new
# member that enables one on another, is covered the day it lands; `list`
# prints them and why.
#
# Each package is its own cargo invocation: two `-p` in one command unify
# their features again.
#
# Not covered here: a feature that no build turns on, own or workspace
# (`cargo tree` shows it in neither, so nothing differs), and another
# target's feature set (`android` is the android check job's).
#
# `test` runs `cargo nextest run -p <package> --all-targets --locked
# --profile ci`, with --no-tests=pass: a package of helpers has none, and
# that is not a failure of this step. CARGO overrides the cargo binary (the
# self-test's seam), OWN_FEATURES_REPO the workspace root.
#
# Exit codes: 0 every package passes (or none differs); 1 clippy or a test
# failed, every package still run and the failures named; 2 usage or a
# cargo metadata/tree failure.
set -uo pipefail
HERE="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
REPO="${OWN_FEATURES_REPO:-$( cd -- "$HERE/../.." && pwd )}"
CARGO="${CARGO:-cargo}"
me="own_features"
die() { echo "$me: $*" >&2; exit 2; }

mode="${1:-}"
[[ $# -eq 1 && "$mode" =~ ^(list|clippy|test)$ ]] || die "usage: $0 list|clippy|test"
cd "$REPO" || die "cannot enter $REPO"

members="$("$CARGO" metadata --no-deps --format-version 1 --locked \
           | python3 -c 'import json,sys; print("\n".join(sorted(p["name"] for p in json.load(sys.stdin)["packages"])))')" \
    || die "cargo metadata failed in $REPO"
[[ -n "$members" ]] || die "cargo metadata listed no workspace package"

# One line per (package, feature set) cargo tree prints: "name|f1,f2". A
# package can appear with two sets (a build-dependency is resolved apart
# from a normal one), so a set of lines, sorted, is what is compared.
tree_features() {  # <cargo tree selection args...>
    "$CARGO" tree --locked -e normal,build,dev -f '{p}|{f}' --prefix none "$@" \
        | sed -E 's/ \(\*\)$//; s/^([^ ]+) [^|]*\|/\1|/' | sort -u
}
sets_of() { awk -F'|' -v p="$1" '$1 == p { printf "%s%s", sep, ($2 == "" ? "(none)" : $2); sep = " " }'; }
ws="$(tree_features --workspace)" || die "cargo tree --workspace failed"

pkgs=()
while IFS= read -r p; do
    own="$(tree_features -p "$p")" || die "cargo tree -p $p failed"
    w="$(sets_of "$p" <<<"$ws")"
    o="$(sets_of "$p" <<<"$own")"
    [[ "$w" == "$o" ]] && continue
    pkgs+=("$p")
    echo "$me: $p — workspace build [${w}], its own [${o}]"
done <<<"$members"

if (( ${#pkgs[@]} == 0 )); then
    echo "$me: every package's own feature set is the workspace's; nothing to run"
    exit 0
fi
[[ "$mode" == list ]] && exit 0

failed=()
for p in "${pkgs[@]}"; do
    case "$mode" in
      clippy) cmd=("$CARGO" clippy -p "$p" --all-targets --locked -- -D warnings) ;;
      test)   cmd=("$CARGO" nextest run -p "$p" --all-targets --locked --profile ci --no-tests=pass) ;;
    esac
    echo "$me: ${cmd[*]}"
    "${cmd[@]}" || failed+=("$p")
done
if (( ${#failed[@]} )); then
    echo "$me: $mode failed with each package's own features: ${failed[*]}" >&2
    exit 1
fi
echo "$me: OK — $mode passes for ${pkgs[*]} with each package's own features"
