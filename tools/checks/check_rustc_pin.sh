#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/checks/check_rustc_pin.sh
#
# >>> help
# Are the compiler cargo will use, and the clippy and rustfmt it runs, the
# toolchain rust-toolchain.toml pins, and does Cargo.toml's rust-version
# state the same version?
#
#   tools/checks/check_rustc_pin.sh
#
# rust-toolchain.toml pins `channel`, and rustup honours it. But a host
# without rustup builds with whatever rustc is on PATH: on a Fedora host
# that is the distribution's package, which matched the 1.98.1 pin only
# while Fedora shipped 1.98.1, and a routine system update moves every
# login on the host to a new compiler at once with no commit here. CI
# builds with the pin, so the first sign is a lint that fires on one side
# only.
# This asks the compiler and compares.
#
# WHAT IS ASKED: the compiler cargo would invoke — `$RUSTC`, else
# `$CARGO_BUILD_RUSTC`, else `rustc` — run with --version from the
# repository root, where rustup reads the pin file. Its version token
# (`rustc 1.98.1 (…)` → `1.98.1`) must equal `channel` exactly when the
# channel is X.Y.Z, or be X.Y.<patch> when it is X.Y; a pre-release
# (`1.98.1-beta.2`) is neither (test_check_rustc_pin.sh, both pin forms).
# A `build.rustc` in a cargo config file also chooses the compiler, and
# this does not resolve one: a config in scope — the repository's
# .cargo/, a parent directory's, or $CARGO_HOME's — with any key spelled
# `rustc` (bare, quoted, dotted or in an inline table; rustc-wrapper is
# another key) is a failure to check, named. Loud rather than parsed:
# TOML spells one key several ways, and a missed spelling would pass
# silently. A line whose first non-blank character is `#` is a comment
# and carries no key, so it is dropped first; a `#` later in a line is
# not stripped, since one inside a string ahead of a real rustc key would
# hide it — a trailing comment naming rustc stays a loud false positive. Cargo searches from the directory it runs in; this from the
# repository root, where `cargo xtask` and CI run — so a tracked cargo
# config below the root, which cargo would read from there, is a failure
# to check too.
#
# THE STATED MINIMUM: Cargo.toml's [workspace.package] rust-version must
# equal `channel` as written. A rust-version below the pin reads as a
# tested floor that nothing builds against; one above it refuses the
# pinned compiler. Both move together, and this is what says so.
#
# CLIPPY AND RUSTFMT are separate packages on a distribution, which can
# update apart from the compiler; under rustup they come with it. What is
# asked is what cargo runs: `cargo clippy --version` and `cargo fmt
# --version`, from the root, with `$CARGO` (set under `cargo xtask`) else
# `cargo`. Predicting the binary instead fails both ways: cargo finds a
# subcommand on PATH, with `$CARGO_HOME/bin` put first only when PATH
# does not hold it already, so neither PATH's tool nor `rustup which` is
# reliably the one run (a CARGO_HOME without rustup's proxies made
# `cargo clippy` the distribution's while rustup's tool was the pin's —
# test_check_rustc_pin.sh, both tools).
#   - A build that names its commit, `clippy 0.1.98 (48a229ceae
#     2026-09-01)` as rustup's do, must name rustc's: one hash a prefix of
#     the other, and the same date.
#   - A build that names none is a distribution's, and its version does
#     not say which compiler it was built on. For clippy, its minor
#     (0.1.<minor>) must be the pin's, and the clippy-driver beside the
#     cargo-clippy cargo finds, by cargo's rule above, with its symlinks
#     resolved as cargo-clippy resolves its own path — the one it runs —
#     must be built on the pin (`clippy-driver
#     --rustc --version`). For rustfmt, whose version (1.9.0) names no
#     compiler, the one cargo-fmt runs ($RUSTFMT, else PATH's) must link a
#     librustc_driver (ldd) in that rustc's sysroot; the library's hash
#     names the toolchain build.
#
# Exit codes:
#   0  the compiler, clippy and rustfmt are the pinned toolchain's, and
#      rust-version states the pin
#   1  one is not; what each is built on is printed
#   2  a failure to check: no rust-toolchain.toml, no `channel`, no
#      Cargo.toml or no rust-version in its [workspace.package], a
#      channel that names no version (`stable`, a nightly), a cargo
#      config in scope setting a rustc key or unreadable, a tracked cargo
#      config below the root, a tree git cannot list, a rustc that cannot
#      be run or prints no version, a `cargo clippy` or `cargo fmt` that
#      cannot be run or prints no version, or for a build naming no
#      commit, a cargo-clippy that cannot be found where cargo looks, a
#      cargo-clippy with no clippy-driver beside it, a
#      clippy-driver that cannot be run or prints no version, or a rustfmt
#      that cannot be found or links no librustc_driver
# <<< help

set -uo pipefail

ROOT="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )/../.." && pwd )"
cd "$ROOT" || exit 2
me="check_rustc_pin"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
fi

pin_file="rust-toolchain.toml"
[[ -f "$pin_file" ]] || { echo "$me: no $pin_file at the repository root" >&2; exit 2; }
channel="$(sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*$/\1/p' "$pin_file" | head -1)"
[[ -n "$channel" ]] || { echo "$me: $pin_file has no channel = \"…\"" >&2; exit 2; }
if [[ ! "$channel" =~ ^[0-9]+\.[0-9]+(\.[0-9]+)?$ ]]; then
    echo "$me: $pin_file's channel \"$channel\" names no version — the pin is X.Y.Z" >&2
    exit 2
fi

# The stated minimum: [workspace.package] rust-version, as written.
[[ -f Cargo.toml ]] || { echo "$me: no Cargo.toml at the repository root" >&2; exit 2; }
rust_version="$(awk '
    /^[[:space:]]*\[/ { in_pkg = ($0 ~ /^[[:space:]]*\[workspace\.package\][[:space:]]*(#.*)?$/); next }
    in_pkg && /^[[:space:]]*rust-version[[:space:]]*=/ {
        if (match($0, /"[^"]*"/)) { print substr($0, RSTART + 1, RLENGTH - 2); exit }
    }' Cargo.toml)"
[[ -n "$rust_version" ]] || { echo "$me: Cargo.toml's [workspace.package] has no rust-version = \"…\"" >&2; exit 2; }
if [[ "$rust_version" != "$channel" ]]; then
    echo "FAIL: Cargo.toml's rust-version is $rust_version; $pin_file pins $channel"
    echo "Bump them together: the stated minimum is the pinned compiler."
    exit 1
fi

# A key spelled rustc in a cargo config in scope: cargo's own config
# search, from the repository root up, then $CARGO_HOME. One pattern,
# read by the self-test too.
rustc_key_re="(^|[[:space:].{,])[\"']?rustc[\"']?[[:space:]]*="
# The list from stdout alone: a warning git prints while succeeding is not
# a path. Its stderr is read only to say why it failed.
tracked="$(git -C "$ROOT" ls-files 2>/dev/null)" || {
    echo "$me: cannot list the tree's tracked files, so cargo configs below the root cannot be checked: $(git -C "$ROOT" ls-files 2>&1 >/dev/null | tail -1)" >&2
    exit 2
}
below="$(grep -E '(^|/)\.cargo/config(\.toml)?$' <<<"$tracked" | grep -Ev '^\.cargo/config(\.toml)?$' | head -1)"
if [[ -n "$below" ]]; then
    echo "$me: $below is a cargo config below the repository root, which cargo reads when run from there; this check searches from the root only" >&2
    exit 2
fi
configs=()
dir="$ROOT"
while :; do
    configs+=("$dir/.cargo/config.toml" "$dir/.cargo/config")
    [[ "$dir" == / ]] && break
    dir="$(dirname "$dir")"
done
configs+=("${CARGO_HOME:-$HOME/.cargo}/config.toml" "${CARGO_HOME:-$HOME/.cargo}/config")
for cfg in "${configs[@]}"; do
    [[ -f "$cfg" ]] || continue
    content="$(cat "$cfg" 2>/dev/null)" || { echo "$me: cannot read $cfg, a cargo config in scope" >&2; exit 2; }
    # Filtered into a variable, then matched: not a pipe into grep -q,
    # whose early exit kills the writer under pipefail and reads as no
    # match on a config of some KB (the padded case in the self-test).
    keys="$(grep -Ev '^[[:space:]]*#' <<<"$content")"
    grep -Eq "$rustc_key_re" <<<"$keys" || continue
    echo "$me: $cfg sets a key spelled rustc (build.rustc chooses cargo's compiler); this check does not resolve it — ask that compiler's --version against $pin_file by hand" >&2
    exit 2
done

rustc_bin="${RUSTC:-${CARGO_BUILD_RUSTC:-rustc}}"
if ! out="$("$rustc_bin" --version 2>&1)"; then
    echo "$me: \`$rustc_bin --version\` failed:" >&2
    printf '%s\n' "$out" | tail -3 >&2
    exit 2
fi
version="$(sed -n 's/^rustc \([^ ]*\).*$/\1/p' <<<"$out" | head -1)"
[[ -n "$version" ]] || { echo "$me: \`$rustc_bin --version\` printed no version: $out" >&2; exit 2; }

# pinned <version>: is this compiler version the pin?
pinned() {
    if [[ "$channel" =~ ^[0-9]+\.[0-9]+$ ]]; then
        [[ "$1" =~ ^${channel//./\\.}\.[0-9]+$ ]]
    else
        [[ "$1" == "$channel" ]]
    fi
}

if ! pinned "$version"; then
    echo "FAIL: the compiler is rustc $version; $pin_file pins $channel"
    echo "      ($(command -v "$rustc_bin" || echo "$rustc_bin"): $out)"
    echo "Install the pinned toolchain (rustup reads $pin_file), or bump the pin"
    echo "and Cargo.toml's rust-version together if the new compiler is the intent."
    exit 1
fi

# clippy and rustfmt: as cargo runs them (the help block says why).
cargo_bin="${CARGO:-cargo}"
rustc_build="$(sed -n 's/^rustc [^ ]* (\([0-9a-f]\{7,\}\) \([0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\}\)).*$/\1 \2/p' <<<"$out" | head -1)"

# cargo_says <subcommand> <tool>: the line of `cargo <sub> --version`'s
# stdout that starts with <tool>; a rustup `info:` or a cargo `warning:`
# is not the version. Fails, saying why, when cargo fails or prints no
# such line — and a caller exits on that, since an `exit` here leaves only
# the command substitution it runs in (test_check_rustc_pin.sh: `cargo fmt`
# alone failing).
cargo_says() {
    local o line
    if ! o="$("$cargo_bin" "$1" --version 2>/dev/null)"; then
        echo "$me: \`$cargo_bin $1 --version\` failed:" >&2
        "$cargo_bin" "$1" --version 2>&1 >/dev/null | head -5 >&2
        return 2
    fi
    line="$(grep -m1 "^$2 " <<<"$o")"
    [[ -n "$line" ]] || { echo "$me: \`$cargo_bin $1 --version\` printed no $2 version: $(head -1 <<<"$o")" >&2; return 2; }
    printf '%s\n' "$line"
}

# same_build <version line>: 0 when it names rustc's build, 1 when it
# names another, 2 when it names none.
same_build() {
    local b h d rh rd
    b="$(sed -n 's/^.* (\([0-9a-f]\{7,\}\) \([0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\}\))$/\1 \2/p' <<<"$1")"
    [[ -n "$b" ]] || return 2
    read -r h d <<<"$b"
    read -r rh rd <<<"${rustc_build:-none none}"
    [[ "$d" == "$rd" && ( "$h" == "$rh"* || "$rh" == "$h"* ) ]]
}

cline="$(cargo_says clippy clippy)" || exit 2
same_build "$cline"; rc=$?
if (( rc == 1 )); then
    echo "FAIL: cargo clippy is $cline, not rustc $version's build (${rustc_build:-which names none})"
    exit 1
elif (( rc == 2 )); then
    cminor="$(sed -n 's/^clippy 0\.1\.\([0-9]*\)\([^0-9].*\)\{0,1\}$/\1/p' <<<"$cline")"
    [[ -n "$cminor" ]] || { echo "$me: \`$cargo_bin clippy --version\` printed no version: $cline" >&2; exit 2; }
    if [[ "$cminor" != "$(cut -d. -f2 <<<"$channel")" ]]; then
        echo "FAIL: cargo clippy is $cline; $pin_file pins $channel"
        exit 1
    fi
    # cargo's rule: PATH, with $CARGO_HOME/bin put first only when PATH
    # does not hold it already (measured: with it after /usr/bin on PATH,
    # cargo ran /usr/bin's). cargo-clippy runs the clippy-driver beside its
    # own path, symlinks resolved.
    ch_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
    search="$PATH"
    case ":$PATH:" in *":$ch_bin:"*) ;; *) search="$ch_bin:$PATH" ;; esac
    cc_bin="$(PATH="$search" command -v cargo-clippy)" || {
        echo "$me: no cargo-clippy where cargo looks ($search), though cargo ran $cline" >&2; exit 2; }
    cc_bin="$(readlink -f -- "$cc_bin")" || { echo "$me: cannot resolve $cc_bin" >&2; exit 2; }
    clippy_bin="$(dirname "$cc_bin")/clippy-driver"
    [[ -x "$clippy_bin" ]] || {
        echo "$me: no clippy-driver beside $cc_bin, which names no build ($cline)" >&2; exit 2; }
    if ! cout="$("$clippy_bin" --rustc --version 2>&1)"; then
        echo "$me: \`$clippy_bin --rustc --version\` failed (cargo clippy runs it):" >&2
        printf '%s\n' "$cout" | tail -3 >&2
        exit 2
    fi
    cversion="$(sed -n 's/^rustc \([^ ]*\).*$/\1/p' <<<"$cout" | head -1)"
    [[ -n "$cversion" ]] || { echo "$me: \`$clippy_bin --rustc --version\` printed no version: $cout" >&2; exit 2; }
    if ! pinned "$cversion"; then
        echo "FAIL: clippy is built on rustc $cversion; $pin_file pins $channel"
        echo "      ($clippy_bin: $cout)"
        exit 1
    fi
fi

fline="$(cargo_says fmt rustfmt)" || exit 2
same_build "$fline"; rc=$?
if (( rc == 1 )); then
    echo "FAIL: cargo fmt is $fline, not rustc $version's build (${rustc_build:-which names none})"
    exit 1
elif (( rc == 2 )); then
    # cargo-fmt runs $RUSTFMT, else the rustfmt on PATH.
    fmt_bin="$(command -v "${RUSTFMT:-rustfmt}")" || {
        echo "$me: no ${RUSTFMT:-rustfmt} found for $fline, which names no build" >&2; exit 2; }
    fmt_driver="$(ldd "$fmt_bin" 2>/dev/null | sed -n 's/^[[:space:]]*\(librustc_driver-[^[:space:]]*\.so\).*/\1/p' | head -1)"
    [[ -n "$fmt_driver" ]] || { echo "$me: $fmt_bin links no librustc_driver that ldd reports, so its compiler cannot be told" >&2; exit 2; }
    sysroot="$("$rustc_bin" --print sysroot 2>/dev/null)"
    if [[ -z "$sysroot" ]] || ! compgen -G "$sysroot/lib*/librustc_driver-*.so" >/dev/null; then
        echo "$me: \`$rustc_bin --print sysroot\` names no directory holding a librustc_driver" >&2; exit 2
    fi
    if ! compgen -G "$sysroot/lib*/$fmt_driver" >/dev/null; then
        echo "FAIL: rustfmt ($fmt_bin) links $fmt_driver, which is not rustc $version's"
        echo "      ($sysroot holds $(cd "$sysroot" && ls lib*/librustc_driver-*.so | tr '\n' ' '))"
        exit 1
    fi
fi

echo "$me: rustc $version is the pinned $channel, clippy and rustfmt are built on it, and rust-version states it"
