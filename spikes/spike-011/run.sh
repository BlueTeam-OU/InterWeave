#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# SPIKE-011's desktop rows, in one pass, onto stdout (redirect it to
# REPRODUCTION-<date>.log): the three engines built --locked on the pinned
# toolchain, their versions, the timings interleaved by round so a slow
# moment of the host lands on every engine alike, the deletion property
# with its control, and the failure rows.
#
#   WORK=<empty dir> ./run.sh [kept ...]     (default: 500 2000)
#
# The sizes are kept messages, the store's bulk (a read message not kept is
# released): 500 is the realistic store, 2000 a large one (the owner,
# 2026-10-21). STRESS, one shorter round at 20000, is labelled as such and
# is not where a criterion is judged.
#
# WORK holds the builds and the stores; it is left for inspection and is
# the caller's to remove. The key is made here, per run, and never printed.
set -euo pipefail
HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
: "${WORK:?WORK=<an empty directory> is required}"
[ -d "$WORK" ] && [ -z "$(ls -A "$WORK")" ] || { echo "WORK must be an empty directory" >&2; exit 2; }
WORK="$(cd -- "$WORK" && pwd)"
chmod 700 "$WORK"
SIZES=("${@:-500 2000}")
STRESS=20000
# shellcheck disable=SC2206
SIZES=(${SIZES[*]})
ROUNDS=3
REPS=30
export PATH="$HOME/.cargo/bin:$PATH"

build() { # <name> <cargo args...>
    local name=$1; shift
    (cd "$HERE/harness" && CARGO_TARGET_DIR="$WORK/target-$name" cargo build -q --release --locked "$@")
    echo "$WORK/target-$name/release/spike-011"
}

echo "# SPIKE-011 desktop rows"
echo "# date:      $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "# commit:    $(git -C "$HERE" rev-parse HEAD) (the tree run; the harness pins the store by its own rev)"
echo "# dirty:     $(git -C "$HERE" status --porcelain -- . | wc -l) changed path(s) under spikes/spike-011"
echo "# toolchain: $(cargo --version), $(rustc --version)"
echo "# host:      $(uname -srm), $(nproc) cpu, $(findmnt -no FSTYPE --target "$WORK") under WORK"
echo "# cpu:       $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ *//')"

PLAIN=$(build plain)
SYS=$(build cipher --features cipher)
VEND=$(build vendored --features cipher-vendored)
INSPECT=$(LIBSQLITE3_FLAGS=SQLITE_ENABLE_DBPAGE_VTAB build inspect --features cipher-vendored)
for b in "$PLAIN" "$SYS" "$VEND" "$INSPECT"; do
    echo "# binary:    $(sha256sum "$b" | cut -c1-16)  $(stat -c %s "$b") bytes  ${b#"$WORK"/}"
done

SPIKE011_KEY="$(head -c32 /dev/urandom | od -An -tx1 | tr -d ' \n')"
export SPIKE011_KEY

echo "## versions"
for b in "$PLAIN" "$SYS" "$VEND"; do "$b" versions; done

echo "## timings: ${SIZES[*]} kept, $ROUNDS rounds of $REPS per engine"
for n in "${SIZES[@]}"; do
    for r in $(seq "$ROUNDS"); do
        for e in plain sys vend; do
            b=$PLAIN; [ $e = sys ] && b=$SYS; [ $e = vend ] && b=$VEND
            d="$WORK/bench-$n-$r-$e"; mkdir -m 700 "$d"
            "$b" bench "$d" "$n" "$REPS" | sed "s/^{/{\"round\":$r,/"
            rm -rf "$d"
        done
    done
done

echo "## stress, not a criterion: $STRESS kept, 1 round of 10 per engine"
for e in plain sys vend; do
    b=$PLAIN; [ $e = sys ] && b=$SYS; [ $e = vend ] && b=$VEND
    d="$WORK/stress-$e"; mkdir -m 700 "$d"
    "$b" bench "$d" "$STRESS" 10 | sed 's/^{/{"stress":true,/'
    rm -rf "$d"
done

echo "## deletion (RETENTION.md section 8)"
for e in plain sys vend; do
    b=$PLAIN; [ $e = sys ] && b=$SYS; [ $e = vend ] && b=$VEND
    for c in none clean unclean; do
        d="$WORK/del-$e-$c"; mkdir -m 700 "$d" "$d/copy"
        "$b" deletion "$d" "$c" | grep -v '"case":"setup"'
        if [ $e != plain ]; then
            cp "$d"/human.sqlite* "$d/copy/"
            "$INSPECT" pages "$d/copy/human.sqlite" | sed "s/^{/{\"case\":\"$c\",/"
        fi
    done
done

echo "## failures"
echo "# a wrong key, on the vendored build's clean store:"
SPIKE011_KEY="$(head -c32 /dev/urandom | od -An -tx1 | tr -d ' \n')" "$VEND" open-once "$WORK/del-vend-clean/human.sqlite" 2>&1 | sed 's/^/  /' || true
echo "# the plaintext build (a rollback) on the encrypted store:"
"$PLAIN" open-once "$WORK/del-vend-clean/human.sqlite" 2>&1 | sed 's/^/  /' || true
echo "# the vendored build on a plaintext store (the key given, the file never encrypted):"
"$VEND" open-once "$WORK/del-plain-clean/human.sqlite" 2>&1 | sed 's/^/  /' || true
echo "# end"
