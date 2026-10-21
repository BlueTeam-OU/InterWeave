#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# SPIKE-011's rewrite and key-file rows (ADR-0060 decisions 3 and 6, A
# 2026-10-21), onto stdout (redirect it to REPRODUCTION-rewrite-<date>.log):
# the plaintext-to-encrypted rewrite under a crash forced after each step,
# each followed by the next start; the key file missing, of a newer
# version, and malformed; and the shipping loader's marker check at each
# point. The prototype is `harness/src/migrate.rs`.
#
#   WORK=<empty dir> ./rewrite.sh [kept]     (default: 500)
set -euo pipefail
HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
: "${WORK:?WORK=<an empty directory> is required}"
[ -d "$WORK" ] && [ -z "$(ls -A "$WORK")" ] || { echo "WORK must be an empty directory" >&2; exit 2; }
WORK="$(cd -- "$WORK" && pwd)"
chmod 700 "$WORK"
KEPT="${1:-500}"
export PATH="$HOME/.cargo/bin:$PATH"
unset SPIKE011_KEY

(cd "$HERE/harness" && CARGO_TARGET_DIR="$WORK/target-vendored" cargo build -q --release --locked --features cipher-vendored)
(cd "$HERE/harness" && CARGO_TARGET_DIR="$WORK/target-plain" cargo build -q --release --locked)
VEND="$WORK/target-vendored/release/spike-011"
PLAIN="$WORK/target-plain/release/spike-011"

echo "# SPIKE-011 rewrite and key-file rows"
echo "# date:      $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "# commit:    $(git -C "$HERE" rev-parse HEAD)"
echo "# dirty:     $(git -C "$HERE" status --porcelain -- . | wc -l) changed path(s) under spikes/spike-011"
echo "# toolchain: $(cargo --version), $(rustc --version)"
echo "# host:      $(uname -srm), $(findmnt -no FSTYPE --target "$WORK") under WORK"
for b in "$VEND" "$PLAIN"; do
    echo "# binary:    $(sha256sum "$b" | cut -c1-16)  ${b#"$WORK"/}"
done
echo "# kept:      $KEPT, plus the unread backlog"

seeded() { # <dir>: a plaintext store, its content seen in clear (the control)
    mkdir -m 700 "$1"
    "$VEND" migrate-seed "$1" "$KEPT"
    if grep -qa "the quick brown fox jumps" "$1/state/human/human.sqlite"; then
        echo "  control: the seeded plaintext holds the content in clear"
    else
        echo "  CONTROL FAILED: the search does not see the seeded content"; exit 1
    fi
}

echo "## a clean rewrite, then the next start"
d="$WORK/clean"; seeded "$d"
"$VEND" start "$d"
"$VEND" start "$d"
"$PLAIN" rollback-check "$d"

echo "## a crash after each step, then the next start, then one more"
for step in marker-raised key-written exported renamed-encrypted scrubbed unlinked renamed-final; do
    echo "# crash after: $step"
    d="$WORK/crash-$step"; seeded "$d"
    if SPIKE011_CRASH_AT="$step" "$VEND" start "$d" >/dev/null 2>&1; then
        echo "  THE CRASH DID NOT HAPPEN"; exit 1
    fi
    echo "  left: human/ $(ls "$d/state/human" | tr '\n' ' ')| state/ $(ls "$d/state" | grep -v '^human$' | tr '\n' ' ')"
    "$PLAIN" rollback-check "$d" | sed 's/^/  /'
    "$VEND" start "$d" | sed 's/^/  /'
    "$VEND" start "$d" | sed 's/^/  /'
done

echo "## the key file"
for case in missing newer-version malformed; do
    echo "# key file: $case"
    d="$WORK/key-$case"; seeded "$d"
    "$VEND" start "$d" >/dev/null
    key="$d/state/human-store.key"
    before="$(sha256sum "$d/state/human/human.sqlite" | cut -c1-16)"
    case $case in
        missing) mv "$key" "$key.aside" ;;
        newer-version) printf '\x02' | dd of="$key" bs=1 seek=4 conv=notrunc status=none ;;
        malformed) truncate -s 20 "$key" ;;
    esac
    "$VEND" start "$d" | sed 's/^/  /'
    after="$(sha256sum "$d/state/human/human.sqlite" | cut -c1-16)"
    echo "  the store's file: $before before, $after after ($([ "$before" = "$after" ] && echo untouched || echo CHANGED))"
done
echo "# end"
