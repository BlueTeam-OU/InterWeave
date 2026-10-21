#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/android_ci_toolchain.sh
#
# Installs on a CI runner what tools/host/android/android-toolchain.pins
# names, so the Android jobs build with the host's versions and not the
# runner image's: the JDK by its checksum, every SDK package by its
# revision, the Rust target, and cargo-ndk at its version.
#
#   bash tools/ci/android_ci_toolchain.sh <component>...
#
# Components, any of:
#   jdk       the pinned Temurin JDK, its tarball checked against
#             JDK_SHA256, unpacked under $RUNNER_TEMP; JAVA_HOME points at it
#   sdk       every PKG_<n>=<package>@<revision> into the runner's SDK
#             ($ANDROID_HOME) by sdkmanager, each then held to its pinned
#             revision by its own source.properties; the install is tried
#             SDK_INSTALL_ATTEMPTS times (default 3, SDK_RETRY_DELAY seconds
#             apart, default 15), its downloads cleared between. sdkmanager installs
#             the newest revision of a package, so a revision Google
#             replaced fails here, by name, rather than building with it.
#             Never a HOST_PKG_<n>: those are for other projects' builds on
#             a host's shared SDK, not this repository's CI
#   rust      the rust-toolchain.toml toolchain and RUST_ANDROID_TARGET
#   cargo-ndk cargo-ndk at CARGO_NDK_VERSION, --locked
#
# What the later steps need is written to $GITHUB_ENV and $GITHUB_PATH
# when they are set (on a runner), and printed either way: JAVA_HOME, and
# the JDK's bin on PATH.
#
# Seams for the self-test: ANDROID_TOOLCHAIN_PINS, SDKMANAGER, FETCH (a
# command run as `$FETCH <url> <out>`), RUSTUP, CARGO, SDK_INSTALL_ATTEMPTS,
# SDK_RETRY_DELAY.
#
# Exit codes: 0 installed; 1 an install or a pin check failed; 2 usage,
# environment or pins problem.
set -uo pipefail
HERE="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
REPO="$( cd -- "$HERE/../.." && pwd )"
PINS="${ANDROID_TOOLCHAIN_PINS:-$REPO/tools/host/android/android-toolchain.pins}"
me="android_ci_toolchain"
die() { echo "$me: $*" >&2; exit 2; }
fail() { echo "$me: $*" >&2; exit 1; }

(( $# )) || die "name at least one component: jdk sdk rust cargo-ndk"
for c in "$@"; do
    case "$c" in jdk|sdk|rust|cargo-ndk) ;; *) die "unknown component '$c' (jdk sdk rust cargo-ndk)" ;; esac
done
[[ -r "$PINS" ]] || die "cannot read the pins file $PINS"
pin() { awk -F= -v k="$1" '$1==k {print substr($0, index($0, "=") + 1); exit}' "$PINS"; }
# need <var> <key>: assigns in THIS shell, so a missing pin's die exits the
# script — inside $(…) it would leave only the subshell and carry on.
need() { printf -v "$1" '%s' "$(pin "$2")"; [[ -n "${!1}" ]] || die "no $2 in $PINS"; }

# A step's environment reaches the next steps only through these files.
export_env() {  # <name> <value>
    echo "$me: $1=$2"
    [[ -n "${GITHUB_ENV:-}" ]] && printf '%s=%s\n' "$1" "$2" >> "$GITHUB_ENV"
    return 0
}
export_path() {  # <dir>
    echo "$me: PATH += $1"
    [[ -n "${GITHUB_PATH:-}" ]] && printf '%s\n' "$1" >> "$GITHUB_PATH"
    return 0
}

install_jdk() {
    local url sum ver tmp tarball home
    need url JDK_URL; need sum JDK_SHA256; need ver JDK_VERSION
    tmp="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/android-ci-jdk"
    rm -rf "$tmp"; mkdir -p "$tmp" || fail "cannot create $tmp"
    tarball="$tmp/jdk.tar.gz"
    "${FETCH:-curl_fetch}" "$url" "$tarball" || fail "could not download the JDK from $url"
    echo "$sum  $tarball" | sha256sum --check --status \
        || fail "the JDK tarball's sha256 is $(sha256sum "$tarball" | cut -d' ' -f1), not the pinned $sum"
    tar -xzf "$tarball" -C "$tmp" || fail "could not unpack the JDK"
    home=""
    for r in "$tmp"/*/release; do [[ -f "$r" ]] && { home="${r%/release}"; break; }; done
    [[ -n "$home" && -x "$home/bin/javac" ]] || fail "no bin/javac in the unpacked JDK under $tmp"
    # The tarball's own release file names its version, which the checksum
    # already fixes; reading it says in the log which JDK the jobs run.
    grep -q "JAVA_RUNTIME_VERSION=\"$ver\"" "$home/release" \
        || fail "the unpacked JDK is $(grep JAVA_RUNTIME_VERSION "$home/release"), not $ver"
    echo "$me: JDK $ver at $home"
    export_env JAVA_HOME "$home"
    export_path "$home/bin"
}
curl_fetch() { curl --fail --silent --show-error --location --retry 3 --output "$2" "$1"; }

# The revision a package's source.properties records.
revision() { awk -F' *= *' '$1=="Pkg.Revision" {print $2; exit}' "$1/source.properties" 2>/dev/null; }

install_sdk() {
    [[ -n "${ANDROID_HOME:-}" ]] || die "ANDROID_HOME is not set; the runner's SDK is needed"
    local sdkm="${SDKMANAGER:-$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager}"
    [[ -x "$sdkm" ]] || die "no sdkmanager at $sdkm"
    local line pkg rev dir got pkgs=() n=0
    while IFS= read -r line; do
        # <package>@<revision>, each `;` segment beginning with a letter or
        # digit, so no segment is `.` or `..`: the value becomes a path the
        # retry below clears a marker under. Exit 2 before sdkmanager runs.
        [[ "${line#*=}" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*(\;[A-Za-z0-9][A-Za-z0-9._-]*)*@[0-9]+(\.[0-9]+)*$ ]] \
            || die "pin '${line}' in $PINS is not <package>@<revision>"
        pkg="${line#*=}"; rev="${pkg##*@}"; pkg="${pkg%@*}"
        pkgs+=("$pkg|$rev")
    done < <(grep -E '^PKG_[0-9]+=' "$PINS")
    (( ${#pkgs[@]} )) || die "no PKG_<n> package in $PINS"
    local names=(); for line in "${pkgs[@]}"; do names+=("${line%|*}"); done
    # Licences answered from a process substitution, not a pipe: under
    # pipefail, `yes | sdkmanager` fails on yes's SIGPIPE when sdkmanager
    # stops reading, however the install went.
    #
    # Retried: one bad download of a package archive ("Error on ZipFile
    # unknown archive", the NDK on runs of #327 and #343's merge group)
    # failed a required job and evicted a reviewed PR from the queue.
    #
    # What a failed attempt leaves, from the installer the runner runs
    # (its log reads "An error occurred while preparing SDK package", the
    # prepare step of repository-core's AbstractPackageOperation, read on
    # this host's cmdline-tools): the download and its unzip under the SDK
    # root's .temp/ (and .downloadIntermediates/), and at the package's
    # install path a .installer/ directory recording the operation, made
    # before the download, with no package content; content reaches the
    # install path only on completion, by an overwrite that restores its
    # backup on failure. So before a retry exactly those are removed: the
    # two staging directories, and for each pinned package not at its
    # revision its .installer/, then its directory by rmdir, which only
    # succeeds on an empty one. Nothing is removed recursively by a name
    # the pins give, so no pin can make the retry delete a category, a
    # grouping (extras/google) or the sdkmanager's own directory (#347
    # review rounds 1-3). What decides is still the revision check below;
    # the last attempt's failure is red. Not cleared: the staging of the
    # newer `android` CLI behind an sdkmanager shim ($SDK/.sdk/), which
    # leaves nothing at the install path when a download fails.
    local attempts="${SDK_INSTALL_ATTEMPTS:-3}" delay="${SDK_RETRY_DELAY:-15}" try=1
    [[ "$attempts" =~ ^[1-9][0-9]*$ ]] || die "SDK_INSTALL_ATTEMPTS '$attempts' is not a positive count"
    until "$sdkm" --install "${names[@]}" < <(yes 2>/dev/null) >/dev/null; do
        (( try < attempts )) || fail "sdkmanager could not install ${names[*]} (attempt $try of $attempts)"
        echo "::warning::$me: sdkmanager failed installing ${names[*]} (attempt $try of $attempts); clearing its downloads and retrying in ${delay}s" >&2
        rm -rf "${ANDROID_HOME:?}/.temp" "${ANDROID_HOME:?}/.downloadIntermediates"
        for line in "${pkgs[@]}"; do
            pkg="${line%|*}"; rev="${line#*|}"; dir="$ANDROID_HOME/${pkg//;//}"
            [[ "$(revision "$dir")" == "$rev" ]] && continue
            rm -rf -- "$dir/.installer"
            rmdir -- "$dir" 2>/dev/null || true
        done
        sleep "$delay"
        try=$((try+1))
    done
    for line in "${pkgs[@]}"; do
        pkg="${line%|*}"; rev="${line#*|}"
        dir="$ANDROID_HOME/${pkg//;//}"
        got="$(revision "$dir")"
        [[ "$got" == "$rev" ]] || fail "$pkg is revision '${got}' at $dir, not the pinned $rev"
        n=$((n+1))
    done
    echo "$me: $n SDK package(s) at their pinned revisions in $ANDROID_HOME"
}

install_rust() {
    local target; need target RUST_ANDROID_TARGET
    local rustup="${RUSTUP:-rustup}"
    (cd "$REPO" && { "$rustup" show active-toolchain || "$rustup" toolchain install; } >/dev/null) \
        || fail "could not install the rust-toolchain.toml toolchain"
    (cd "$REPO" && "$rustup" target add "$target" >/dev/null) || fail "could not add the target $target"
    echo "$me: Rust target $target"
}

install_cargo_ndk() {
    local ver; need ver CARGO_NDK_VERSION
    "${CARGO:-cargo}" install cargo-ndk --version "=$ver" --locked >/dev/null \
        || fail "could not install cargo-ndk $ver"
    echo "$me: cargo-ndk $ver"
}

for c in "$@"; do
    case "$c" in
        jdk) install_jdk ;;
        sdk) install_sdk ;;
        rust) install_rust ;;
        cargo-ndk) install_cargo_ndk ;;
    esac
done
echo "$me: OK — $*"
