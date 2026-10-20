#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# Write the XDG tree this checkout's build leaves after ordinary use, as
# test-data/upgrade/<commit>/, for the upgrade and rollback cases of every
# later build (tests/desktop-e2e/tests/human_app/release.rs; plan section
# 22). Run it from a clean checkout of the commit the tree is named for,
# with a display (the cases beside it need one; this one does not):
#
#   test-data/upgrade/make-tree.sh
#
# It builds the three desktop executables, runs the one ignored case that
# writes the tree, and prints where it went. Nothing in CI runs it: a tree
# is made once per release, at the release's commit, and committed.
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
if [ -n "$(git status --porcelain)" ]; then
    echo "make-tree.sh: the checkout is not clean; a tree names the commit that made it" >&2
    exit 2
fi
commit=$(git rev-parse --short=8 HEAD)
out="$root/test-data/upgrade/$commit"
if [ -e "$out" ]; then
    echo "make-tree.sh: $out exists" >&2
    exit 2
fi

cargo build -p interweave-transport-daemon -p interweave-transportctl \
    -p interweave-human-desktop
INTERWEAVE_UPGRADE_TREE_OUT="$out" INTERWEAVE_UPGRADE_TREE_COMMIT="$commit" \
    cargo test -p interweave-desktop-e2e-tests --test human_app -- \
    --ignored --exact release::write_the_previous_build_tree
echo "$out"
