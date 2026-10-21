#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# SPIKE-011's index rows, onto stdout (redirect it to
# REPRODUCTION-index-<date>.log): the same timings as run.sh, on the store
# BEFORE and AFTER schema v8's walk index (architect-cto's ruling, REPLY
# 01a15f1a), interleaved by round so a slow moment of the host lands on
# both alike.
#
#   WORK=<empty dir> ./index.sh <before rev> <after rev>
#
# Each side is the harness at the given store revision: a copy of
# harness/ under WORK with its three pins moved to that revision and its
# lock resolved for it, so the harness's own pin and lock are untouched.
# Two engines: the plaintext baseline and the candidate, SQLCipher over
# vendored OpenSSL (run.sh measured the system-libcrypto build within the
# candidate's spread; it does not cross-build for the phone). Sizes as
# run.sh: 500 and 2000 kept, three rounds of 30; STRESS, 20000 kept, one
# round of 10, labelled and not where a criterion is judged.
#
# WORK is left for inspection and is the caller's to remove. The key is
# made here, per run, and never printed.
set -euo pipefail
HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
: "${WORK:?WORK=<an empty directory> is required}"
[ $# -eq 2 ] || { echo "usage: WORK=<empty dir> $0 <before rev> <after rev>" >&2; exit 2; }
[ -d "$WORK" ] && [ -z "$(ls -A "$WORK")" ] || { echo "WORK must be an empty directory" >&2; exit 2; }
WORK="$(cd -- "$WORK" && pwd)"
chmod 700 "$WORK"
BEFORE=$(git -C "$HERE" rev-parse "$1^{commit}")
AFTER=$(git -C "$HERE" rev-parse "$2^{commit}")
SIZES=(500 2000)
STRESS=20000
ROUNDS=3
REPS=30
export PATH="$HOME/.cargo/bin:$PATH"
PIN=$(sed -n 's/^interweave-human-store = .*rev = "\([0-9a-f]*\)".*/\1/p' "$HERE/harness/Cargo.toml")
[ -n "$PIN" ] || { echo "no store pin in harness/Cargo.toml" >&2; exit 2; }

build() { # <side> <rev> <engine> <cargo args...>
    local side=$1 rev=$2 engine=$3; shift 3
    local src="$WORK/harness-$side"
    if [ ! -d "$src" ]; then
        mkdir "$src"
        (cd "$HERE/harness" && tar --exclude=./target -cf - .) | (cd "$src" && tar -xf -)
        sed -i "s/rev = \"$PIN\"/rev = \"$rev\"/" "$src/Cargo.toml"
        (cd "$src" && cargo update -q -p interweave-human-store -p interweave-profile-config \
            -p interweave-transport-api)
    fi
    (cd "$src" && CARGO_TARGET_DIR="$WORK/target-$side-$engine" cargo build -q --release --locked "$@")
    echo "$WORK/target-$side-$engine/release/spike-011"
}

echo "# SPIKE-011 index rows"
echo "# date:      $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "# commit:    $(git -C "$HERE" rev-parse HEAD) (the tree run)"
echo "# dirty:     $(git -C "$HERE" status --porcelain -- . | wc -l) changed path(s) under spikes/spike-011"
echo "# before:    $BEFORE (store schema v$(git -C "$HERE" show "$BEFORE:crates/human/store/src/schema.rs" | sed -n 's/^pub const SCHEMA_VERSION: i64 = \([0-9]*\);/\1/p'))"
echo "# after:     $AFTER (store schema v$(git -C "$HERE" show "$AFTER:crates/human/store/src/schema.rs" | sed -n 's/^pub const SCHEMA_VERSION: i64 = \([0-9]*\);/\1/p'))"
echo "# toolchain: $(cargo --version), $(rustc --version)"
echo "# host:      $(uname -srm), $(nproc) cpu, $(findmnt -no FSTYPE --target "$WORK") under WORK"
echo "# cpu:       $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ *//')"

declare -A BIN
for side in before after; do
    rev=$BEFORE; [ $side = after ] && rev=$AFTER
    BIN[$side-plain]=$(build $side "$rev" plain)
    BIN[$side-vend]=$(build $side "$rev" vend --features cipher-vendored)
done
for k in before-plain before-vend after-plain after-vend; do
    b=${BIN[$k]}
    echo "# binary:    $(sha256sum "$b" | cut -c1-16)  $(stat -c %s "$b") bytes  ${b#"$WORK"/}"
done

SPIKE011_KEY="$(head -c32 /dev/urandom | od -An -tx1 | tr -d ' \n')"
export SPIKE011_KEY

echo "## timings: ${SIZES[*]} kept, $ROUNDS rounds of $REPS per side and engine"
for n in "${SIZES[@]}"; do
    for r in $(seq "$ROUNDS"); do
        for k in before-plain after-plain before-vend after-vend; do
            d="$WORK/bench-$n-$r-$k"; mkdir -m 700 "$d"
            "${BIN[$k]}" bench "$d" "$n" "$REPS" | sed "s/^{/{\"side\":\"${k%-*}\",\"round\":$r,/"
            rm -rf "$d"
        done
    done
done

echo "## stress, not a criterion: $STRESS kept, 1 round of 10 per side and engine"
for k in before-plain after-plain before-vend after-vend; do
    d="$WORK/stress-$k"; mkdir -m 700 "$d"
    "${BIN[$k]}" bench "$d" "$STRESS" 10 | sed "s/^{/{\"side\":\"${k%-*}\",\"stress\":true,/"
    rm -rf "$d"
done
