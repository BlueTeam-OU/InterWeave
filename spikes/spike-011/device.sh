#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# SPIKE-011's device rows: run.sh's timings and deletion rows on the Stage
# 17 reference device, on the store at the harness's pin (BEFORE) and at
# schema v8's walk index (AFTER), interleaved by round.
#
#   WORK=<empty dir> ./device.sh build <after rev>     # on the host
#   .agent-fabric/bin/phone-run spikes/spike-011/device.sh run <WORK> <serial> > <log>
#
# `build` cross-builds for aarch64-linux-android (API 30, as the app) the
# plaintext and the vendored SQLCipher harness on each side, and the
# inspector at the pin; the AFTER side is a copy of harness/ with its
# three pins moved, so the harness's own pin and lock are untouched.
# `run` is phone-run's command: it connects, pushes the binaries to a
# directory of its own under /data/local/tmp (shell's, not the app's
# sandbox: the medium and the CPU are the device's, the app's data is not
# touched), runs the rows -- each in a directory given to the harness as
# the platform's boundary, as the app's data directory is -- and removes
# that directory however it ends.
# Stress at 20000 kept is three reps per engine, labelled, not a criterion.
#
# The key is made here, per run, and never printed; it reaches the device
# in the command's environment.
set -euo pipefail
HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
TARGET=aarch64-linux-android
SIZES=(500 2000)
STRESS=20000
ROUNDS=3
REPS=30
BINS=(before-plain before-vend before-inspect after-plain after-vend)
# Where a run's store sits under its boundary: the Android client's layout
# (`ProfilePaths::resolve_embedded`), which the harness opens when
# SPIKE011_BOUNDARY is set -- `/data` is system 0771, so the desktop
# opener, judging up to `/`, refuses any path on the phone.
STORE=interweave/state/profiles/spike-011/human/human.sqlite

build_one() { # <src> <name> <cargo args...>
    local src=$1 name=$2; shift 2
    (cd "$src" && CARGO_TARGET_DIR="$WORK/target-$name" \
        cargo ndk -t arm64-v8a -P 30 build -q --release --locked "$@")
    cp "$WORK/target-$name/$TARGET/release/spike-011" "$WORK/bin/$name"
}

case "${1:-}" in
build)
    : "${WORK:?WORK=<an empty directory> is required}"
    [ $# -eq 2 ] || { echo "usage: WORK=<empty dir> $0 build <after rev>" >&2; exit 2; }
    [ -d "$WORK" ] && [ -z "$(ls -A "$WORK")" ] || { echo "WORK must be an empty directory" >&2; exit 2; }
    WORK="$(cd -- "$WORK" && pwd)"
    chmod 700 "$WORK"
    AFTER=$(git -C "$HERE" rev-parse "$2^{commit}")
    PIN=$(sed -n 's/^interweave-human-store = .*rev = "\([0-9a-f]*\)".*/\1/p' "$HERE/harness/Cargo.toml")
    mkdir "$WORK/bin" "$WORK/harness-after"
    (cd "$HERE/harness" && tar --exclude=./target -cf - .) | (cd "$WORK/harness-after" && tar -xf -)
    sed -i "s/rev = \"$PIN\"/rev = \"$AFTER\"/" "$WORK/harness-after/Cargo.toml"
    (cd "$WORK/harness-after" && cargo update -q -p interweave-human-store \
        -p interweave-profile-config -p interweave-transport-api)
    build_one "$HERE/harness" before-plain
    build_one "$HERE/harness" before-vend --features cipher-vendored
    LIBSQLITE3_FLAGS=SQLITE_ENABLE_DBPAGE_VTAB build_one "$HERE/harness" before-inspect --features cipher-vendored
    build_one "$WORK/harness-after" after-plain
    build_one "$WORK/harness-after" after-vend --features cipher-vendored
    {
        echo "before $PIN"
        echo "after $AFTER"
        echo "commit $(git -C "$HERE" rev-parse HEAD)"
        echo "dirty $(git -C "$HERE" status --porcelain -- . | wc -l)"
        echo "toolchain $(cargo --version), $(rustc --version), $(cargo ndk --version)"
    } > "$WORK/bin/BUILD"
    (cd "$WORK/bin" && sha256sum "${BINS[@]}")
    ;;
run)
    [ $# -eq 3 ] || { echo "usage: phone-run $0 run <WORK> <serial>" >&2; exit 2; }
    WORK="$(cd -- "$2" && pwd)"; SERIAL=$3
    adb connect "$SERIAL" >/dev/null
    A=(adb -s "$SERIAL")
    D=/data/local/tmp/spike-011-$$
    cleanup() { "${A[@]}" shell rm -rf "$D" >/dev/null 2>&1 || true; adb disconnect "$SERIAL" >/dev/null 2>&1 || true; }
    trap cleanup EXIT
    KEY="$(head -c32 /dev/urandom | od -An -tx1 | tr -d ' \n')"
    sh_() { "${A[@]}" shell "SPIKE011_KEY=$KEY; export SPIKE011_KEY; cd $D && $*"; }
    "${A[@]}" shell mkdir -m 700 "$D"
    for b in "${BINS[@]}"; do "${A[@]}" push -q "$WORK/bin/$b" "$D/$b" >/dev/null; done
    "${A[@]}" shell chmod 700 "$D"/'*'

    echo "# SPIKE-011 device rows"
    echo "# date:      $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    sed 's/^/# build:     /' "$WORK/bin/BUILD"
    echo "# device:    $("${A[@]}" shell getprop ro.product.model | tr -d '\r'), Android $("${A[@]}" shell getprop ro.build.version.release | tr -d '\r') (API $("${A[@]}" shell getprop ro.build.version.sdk | tr -d '\r')), $("${A[@]}" shell getprop ro.soc.model | tr -d '\r')"
    echo "# medium:    $("${A[@]}" shell "stat -f -c %T /data/local/tmp; df -h /data/local/tmp | tail -1" | tr '\r\n' '  ')"
    echo "# battery:   $("${A[@]}" shell dumpsys battery | grep -E 'level|AC powered|USB powered|temperature' | tr -d '\r' | tr -s ' \n' ' ')"
    (cd "$WORK/bin" && sha256sum "${BINS[@]}") | cut -c1-16,65- | sed 's/^/# binary:    /'

    echo "## versions"
    for b in before-plain before-vend after-vend; do sh_ "./$b versions"; done

    echo "## timings: ${SIZES[*]} kept, $ROUNDS rounds of $REPS per side and engine"
    for n in "${SIZES[@]}"; do
        for r in $(seq "$ROUNDS"); do
            for k in before-plain after-plain before-vend after-vend; do
                d="$D/bench-$n-$r-$k"
                sh_ "mkdir -m 700 $d && SPIKE011_BOUNDARY=$d ./$k bench $d $n $REPS; rm -rf $d" \
                    | tr -d '\r' | sed "s/^{/{\"side\":\"${k%-*}\",\"round\":$r,/"
            done
        done
    done

    echo "## stress, not a criterion: $STRESS kept, 1 round of 3 per side and engine"
    for k in before-plain after-plain before-vend after-vend; do
        d="$D/stress-$k"
        sh_ "mkdir -m 700 $d && SPIKE011_BOUNDARY=$d ./$k bench $d $STRESS 3; rm -rf $d" \
            | tr -d '\r' | sed "s/^{/{\"side\":\"${k%-*}\",\"stress\":true,/"
    done

    echo "## deletion (RETENTION.md section 8), at the pin"
    for e in plain vend; do
        for c in none clean unclean; do
            d="$D/del-$e-$c"
            sh_ "mkdir -m 700 $d $d/copy && SPIKE011_BOUNDARY=$d ./before-$e deletion $d $c" | tr -d '\r' | grep -v '"case":"setup"'
            if [ $e = vend ]; then
                sh_ "cp $d/$STORE* $d/copy/ && ./before-inspect pages $d/copy/human.sqlite" \
                    | tr -d '\r' | sed "s/^{/{\"case\":\"$c\",/"
            fi
        done
    done

    echo "## failures, at the pin"
    echo "# a wrong key, on the vendored build's clean store:"
    "${A[@]}" shell "SPIKE011_KEY=$(head -c32 /dev/urandom | od -An -tx1 | tr -d ' \n'); export SPIKE011_KEY; cd $D && SPIKE011_BOUNDARY=$D/del-vend-clean ./before-vend open-once $D/del-vend-clean/$STORE 2>&1" | tr -d '\r' | sed 's/^/  /' || true
    echo "# the plaintext build (a rollback) on the encrypted store:"
    sh_ "SPIKE011_BOUNDARY=$D/del-vend-clean ./before-plain open-once $D/del-vend-clean/$STORE 2>&1" | tr -d '\r' | sed 's/^/  /' || true
    echo "# the vendored build on a plaintext store:"
    sh_ "SPIKE011_BOUNDARY=$D/del-plain-clean ./before-vend open-once $D/del-plain-clean/$STORE 2>&1" | tr -d '\r' | sed 's/^/  /' || true
    echo "# end"
    ;;
*)
    echo "usage: WORK=<empty dir> $0 build <after rev> | $0 run <WORK> <serial>" >&2
    exit 2
    ;;
esac
