#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/checks/check_shipped_test_hooks.sh
#
# >>> help
# Does any shipped build's feature graph turn on a `test-hooks` feature?
#
#   tools/checks/check_shipped_test_hooks.sh [--root DIR]
#
# A `test-hooks` feature is TEST-ONLY: declared in a crate's manifest,
# turned on by a self dev-dependency or a test crate, never by default
# and never by a shipped program's [dependencies]. Builds that compile dev
# targets unify it in, so a shipped build is the one place it must be
# off, and reading the graph by hand once (apps/transport-daemon's
# manifest records such a reading) holds nothing for the next change.
#
# The shipped builds, each READ from where the release takes it rather
# than copied here, so a package added there is held without editing this:
#
#   the Linux archive  `target=` and `packages=(...)` in
#                      tools/ci/release_linux_archive.sh, built there as
#                      one `cargo build --release -p ... -p ...`;
#   the APK's library  the cargo-ndk call in
#                      apps/human-android/android/app/build.gradle.kts:
#                      its `-t` ABI and `-p` package, and its release
#                      profile `listOf("--release")`, i.e. no features.
#
# Each is one `cargo tree -e normal --target <t> -p ... -f '{p}|{f}'`:
# the packages of one build resolve their features together, as the
# build does, and `{f}` is each package's resolved feature set — the
# package's own included, which `-e features` edges do not show for a
# root. A feature named exactly `test-hooks` on ANY package fails, not
# only on the crates that declare one today.
#
# A DEPENDENCY-GRAPH CLAIM, NOT AN ARTIFACT CLAIM: what is held is what
# cargo resolves for the release's own command line; the debug APK
# (dev-stand-ins, e2e-cases) is not a shipped build and is not read.
#
# The reader is a strict subset, not a parser: each line it reads must
# appear exactly once in the shape it knows, or this is exit 2 asking for
# the reading to be updated with the release. In particular a release
# profile that gains a `--features` is refused here until it is read.
#
# POSITIVE CONTROLS, each exit 2 when it fails: every package of a build
# names a line of its graph (a graph that came back empty or for other
# packages would pass having checked nothing); at least one workspace
# member declares `test-hooks`; and the first of them, built with it on,
# is found by the same matcher — so a change in cargo's output that
# would blind the match is caught here rather than read as clean.
#
# Exit codes:
#   0  no shipped build turns `test-hooks` on
#   1  one does; the build and the packages are printed
#   2  cargo failed, a release source could not be read, or a positive
#      control failed
# <<< help

set -uo pipefail

me="check_shipped_test_hooks"
ROOT="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )/../.." && pwd )"

case "${1:-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
--root)
    [[ $# -eq 2 ]] || { echo "usage: $me [--root DIR]" >&2; exit 2; }
    ROOT="$2"
    ;;
"") ;;
*) echo "usage: $me [--root DIR]" >&2; exit 2 ;;
esac
cd "$ROOT" || exit 2

FEATURE=test-hooks
ARCHIVE=tools/ci/release_linux_archive.sh
GRADLE=apps/human-android/android/app/build.gradle.kts
WORD='[A-Za-z0-9_.-]+'

err="$(mktemp)"; trap 'rm -f "$err"' EXIT

# the_line <file> <ERE>: the one line of <file> matching <ERE>, or exit 2.
the_line() {
    local hits n
    hits="$(grep -E -- "$2" "$1" 2>/dev/null)"
    n=0; [[ -n "$hits" ]] && n="$(wc -l <<<"$hits")"
    if (( n != 1 )); then
        echo "$me: $1 has $n line(s) of the shape /$2/, not one — update this check's reading with the release" >&2
        exit 2
    fi
    printf '%s\n' "$hits"
}

# The Linux archive.
line="$(the_line "$ARCHIVE" "^target=$WORD\$")" || exit 2
linux_target="${line#target=}"
line="$(the_line "$ARCHIVE" "^packages=\\(($WORD)( $WORD)*\\)\$")" || exit 2
line="${line#packages=(}"
read -r -a linux_packages <<<"${line%)}"

# The APK's library: the ABI cargo-ndk is given, mapped to its target.
line="$(the_line "$GRADLE" "\"cargo\", \"ndk\", \"-t\", \"$WORD\",")" || exit 2
[[ "$line" =~ \"-t\",\ \"($WORD)\" ]]
abi="${BASH_REMATCH[1]}"
case "$abi" in
arm64-v8a) apk_target=aarch64-linux-android ;;
armeabi-v7a) apk_target=armv7-linux-androideabi ;;
x86_64) apk_target=x86_64-linux-android ;;
x86) apk_target=i686-linux-android ;;
*) echo "$me: $GRADLE gives cargo-ndk the ABI '$abi', which this check cannot map to a target" >&2; exit 2 ;;
esac
line="$(the_line "$GRADLE" "\"build\", \"--locked\", \"-p\", \"$WORD\",")" || exit 2
[[ "$line" =~ \"-p\",\ \"($WORD)\" ]]
apk_package="${BASH_REMATCH[1]}"
the_line "$GRADLE" '^ *if \(variant == "Release"\) listOf\("--release"\) else ' >/dev/null || exit 2

# matches <tree>: "<package>" for each line whose resolved features
# include exactly $FEATURE. `{p}` ends in "(path)", so the features are
# after the LAST '|'.
matches() {
    awk -v want="$FEATURE" '{
        i = match($0, /\|[^|]*$/); if (!i) next
        n = split(substr($0, i + 1), f, ",")
        for (k = 1; k <= n; k++) if (f[k] == want) { split($0, p, " "); print p[1]; break }
    }' <<<"$1" | sort -u
}

# graph <target> <package...>: the build's tree, its roots checked.
graph() {
    local target="$1" p tree args=(tree --locked -e normal --prefix none -f '{p}|{f}' --target "$1")
    shift
    for p in "$@"; do args+=(-p "$p"); done
    tree="$(cargo "${args[@]}" 2>"$err")" || {
        echo "$me: cargo ${args[*]} failed:" >&2; tail -5 "$err" >&2; return 2
    }
    for p in "$@"; do
        if ! awk -v p="$p" '$1 == p && $2 ~ /^v/ { found = 1 } END { exit !found }' <<<"$tree"; then
            echo "$me: the graph for $target names no $p — not checked (positive control)" >&2
            return 2
        fi
    done
    printf '%s\n' "$tree"
}

# The matcher's live control: a member that declares the feature, with it on.
meta="$(cargo metadata --format-version 1 --locked --no-deps 2>"$err")" || {
    echo "$me: cargo metadata failed:" >&2; tail -5 "$err" >&2; exit 2
}
declarers="$(python3 -c '
import json, sys
meta = json.load(sys.stdin)
members = set(meta["workspace_members"])
for p in sorted(meta["packages"], key=lambda p: p["name"]):
    if p["id"] in members and sys.argv[1] in p.get("features", {}):
        print(p["name"])
' "$FEATURE" <<<"$meta" 2>"$err")" || {
    echo "$me: cannot read the workspace members' features:" >&2; tail -3 "$err" >&2; exit 2
}
if [[ -z "$declarers" ]]; then
    echo "$me: no workspace member declares $FEATURE — the matcher's control has nothing to run on" >&2
    exit 2
fi
control="$(head -1 <<<"$declarers")"
tree="$(cargo tree --locked -e normal --prefix none -f '{p}|{f}' -p "$control" --features "$FEATURE" 2>"$err")" || {
    echo "$me: cargo tree -p $control --features $FEATURE failed:" >&2; tail -5 "$err" >&2; exit 2
}
if ! grep -qxF -- "$control" <<<"$(matches "$tree")"; then
    echo "$me: $control built with $FEATURE on is not matched — cargo's '{f}' output changed and this check is blind (positive control)" >&2
    exit 2
fi
echo "$me: $(wc -l <<<"$declarers") member(s) declare $FEATURE: $(paste -sd' ' <<<"$declarers"); the matcher sees $control's when it is on"

bad=0
# check <label> <target> <package...>
check() {
    local label="$1" target="$2" tree found
    shift 2
    tree="$(graph "$target" "$@")" || exit 2
    found="$(matches "$tree")"
    if [[ -n "$found" ]]; then
        echo "FAIL: $label ($target: $*) turns $FEATURE on in: $(paste -sd' ' <<<"$found")"
        bad=$((bad + 1))
    else
        echo "$me: $label ($target: $*) turns $FEATURE on nowhere ($(wc -l <<<"$tree") packages in its graph)"
    fi
}
check "the Linux archive" "$linux_target" "${linux_packages[@]}"
check "the APK's library" "$apk_target" "$apk_package"

if (( bad )); then
    echo
    echo "A test-hooks feature is turned on only by a dev-dependency or a test crate."
    echo "\`cargo tree -e normal,features --target <t> -p <package> -i <crate>\` shows which edge turns it on."
    exit 1
fi
exit 0
