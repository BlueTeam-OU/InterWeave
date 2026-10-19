#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# >>> help
# check_android_no_play_services.sh — no Google Play services, Firebase,
# Play, Play Billing or Google services plugin artifact on the Android
# app's classpath
#
#   tools/checks/check_android_no_play_services.sh [--root <dir>]
#
# THE RULE, quoted from ADR-0056 decision 2, which ADR-0057 decision 2
# carries:
#
#   "No Google Play services, in any form. The application classpath
#   carries no Google Play services, Firebase, Play Integrity, Play
#   Billing, Play In-App Update, Play Install Referrer or any
#   `com.google.android.gms`, `com.google.firebase`,
#   `com.google.android.play` or `com.android.billingclient` artifact,
#   and no code path depends on their presence on the device. Lint and
#   test tooling may depend on what it depends on; the application may
#   not. A guard over the Gradle lockfiles holds this (devex-tooling's
#   to shape; the rule owes the test)."
#
# ADR-0057's Amendment 2026-10-19 adds a fifth group to its decision 2:
# `com.google.gms`, "the Google services Gradle plugin's group, the
# usual door for the others". It is refused like the four.
#
# This is that guard. It reads every Gradle lockfile under
# apps/human-android/android/ (`gradle.lockfile` per project,
# `buildscript-gradle.lockfile`, `settings-gradle.lockfile`) and fails
# on any line whose group is one of the five, or a subgroup of one
# (`com.google.firebase.foo` is Firebase too), locked into a
# configuration that is not lint or test tooling.
#
# FAIL-CLOSED ON THE CONFIGURATION NAME. The exemption is a list of the
# lint and test configurations the decision allows, and everything else
# is the application -- a configuration Gradle or a plugin adds tomorrow
# included. The other way round (a list of "application" configurations)
# lets the next classpath through unnamed. The exempt shapes:
#   lint:  androidLintTool, lintChecks, lintPublish, *LintChecksClasspath
#   test:  *UnitTest*, *AndroidTest*, androidTest*,
#          unified-test-platform-*
# The build's own plugin classpath (`classpath` in
# buildscript-gradle.lockfile) and the Kotlin compiler's are NOT exempt:
# the decision names lint and test tooling only, and nothing there needs
# these groups today -- and the buildscript classpath is exactly where
# the Google services plugin (`com.google.gms`) would land. Widening the list is the decision's owner's word
# (architect-cto), not an edit to get CI green.
#
# A GUARD OVER A LOCKFILE IS ONLY AS GOOD AS THE LOCK, so it also fails
# when the lock could have stopped describing the build:
#   - no apps/human-android/android/app/gradle.lockfile (an absent input
#     is not a clean one);
#   - the app lockfile present but not among the files read (a search
#     that read nothing is not a clean result);
#   - a project directory with a build.gradle.kts or build.gradle and no
#     gradle.lockfile beside it;
#   - the app lockfile not naming `releaseRuntimeClasspath`, the
#     classpath the shipped APK is built from (the control that the
#     lockfile covers what ships);
#   - the root build.gradle.kts not calling `lockAllConfigurations()`, or
#     naming LockMode.LENIENT, under which Gradle would resolve past the
#     lock instead of failing.
#
# Exit codes:
#   0  no banned group on an application configuration, lock intact
#   1  a banned artifact on an application configuration, or the lock
#      cannot be trusted
#   2  invocation error
# <<< help

set -uo pipefail

usage() { sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'; }

ROOT=""
while [ $# -gt 0 ]; do
    case "$1" in
        --root)
            [ $# -ge 2 ] || { echo "check_android_no_play_services: --root needs a directory" >&2; exit 2; }
            ROOT="$2"; shift 2 ;;
        --help|-h) usage; exit 0 ;;
        *) echo "check_android_no_play_services: unknown option '$1'" >&2; exit 2 ;;
    esac
done

if [ -z "$ROOT" ]; then
    ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
fi
[ -n "$ROOT" ] || { echo "check_android_no_play_services: no --root and not in a git repo" >&2; exit 2; }
[ -d "$ROOT" ] || { echo "check_android_no_play_services: not a directory: $ROOT" >&2; exit 2; }

GRADLE="$ROOT/apps/human-android/android"
problems=0
fail() { echo "$1"; problems=$((problems + 1)); }

# Files under the Gradle root, Gradle's own output and cache directories
# pruned BELOW it. Pruned by name and not by `-not -path '*/build/*'`:
# -path matches the whole path, the root's included, so a clone under any
# directory named build read nothing at all (review of #307, finding 1).
gradle_files() {
    find "$GRADLE" \( \( -name build -o -name .gradle \) -type d -prune \) -o \( "$@" \) -type f -print 2>/dev/null | sort
}

[ -f "$GRADLE/app/gradle.lockfile" ] || fail "apps/human-android/android/app/gradle.lockfile: missing -- the app's classpath is unlocked or the lockfile was not committed"

root_build="$GRADLE/build.gradle.kts"
if [ ! -f "$root_build" ]; then
    fail "apps/human-android/android/build.gradle.kts: missing -- nothing says the configurations are locked"
else
    grep -Eq '^[^/]*lockAllConfigurations\(\)' "$root_build" \
        || fail "apps/human-android/android/build.gradle.kts: no lockAllConfigurations() -- a configuration outside the lock is outside this guard"
    if grep -Eq '^[^/]*LockMode\.LENIENT' "$root_build"; then
        fail "apps/human-android/android/build.gradle.kts: LockMode.LENIENT -- Gradle would resolve past the lock"
    fi
fi

# Every subproject (a directory below the Gradle root holding a
# build.gradle.kts or a Groovy build.gradle) owes a lockfile beside it.
while IFS= read -r build; do
    [ -n "$build" ] || continue
    dir="$(dirname "$build")"
    [ "$dir" = "$GRADLE" ] && continue
    [ -f "$dir/gradle.lockfile" ] || fail "${dir#"$ROOT"/}: a project with no gradle.lockfile"
done <<EOF
$(gradle_files -name build.gradle.kts -o -name build.gradle)
EOF

if [ -f "$GRADLE/app/gradle.lockfile" ] \
    && ! grep -Eq '(=|,)releaseRuntimeClasspath(,|$)' "$GRADLE/app/gradle.lockfile"; then
    fail "apps/human-android/android/app/gradle.lockfile: names no releaseRuntimeClasspath -- it does not cover what ships"
fi

lockfiles=0
artifacts=0
read_locks="|"
while IFS= read -r lock; do
    [ -n "$lock" ] || continue
    lockfiles=$((lockfiles + 1))
    read_locks="$read_locks$lock|"
    rel="${lock#"$ROOT"/}"
    # One line per artifact: `group:name:version=conf1,conf2`. awk does
    # the matching and prints one finding per banned artifact on an
    # application configuration; the count of artifacts read goes last.
    out="$(awk -v rel="$rel" '
        function banned(g) {
            return g == "com.google.android.gms" || index(g, "com.google.android.gms.") == 1 \
                || g == "com.google.firebase"    || index(g, "com.google.firebase.") == 1 \
                || g == "com.google.android.play" || index(g, "com.google.android.play.") == 1 \
                || g == "com.android.billingclient" || index(g, "com.android.billingclient.") == 1 \
                || g == "com.google.gms" || index(g, "com.google.gms.") == 1
        }
        function tooling(c) {
            return c == "androidLintTool" || c == "lintChecks" || c == "lintPublish" \
                || c ~ /LintChecksClasspath$/ \
                || c ~ /UnitTest/ || c ~ /AndroidTest/ || c ~ /^androidTest/ \
                || c ~ /^unified-test-platform-/
        }
        /^[[:space:]]*#/ || /^[[:space:]]*$/ || /^empty=/ { next }
        {
            eq = index($0, "=")
            if (eq == 0) { next }
            coord = substr($0, 1, eq - 1); confs = substr($0, eq + 1)
            n++
            split(coord, parts, ":")
            if (!banned(parts[1])) { next }
            k = split(confs, c, ",")
            app = ""
            for (i = 1; i <= k; i++) if (!tooling(c[i])) app = app (app == "" ? "" : ",") c[i]
            if (app != "") print rel ": " coord " on " app
        }
        END { print "#count " n + 0 }
    ' "$lock")"
    while IFS= read -r line; do
        case "$line" in
            "#count "*) artifacts=$((artifacts + ${line#\#count })) ;;
            "") ;;
            *) fail "$line" ;;
        esac
    done <<<"$out"
done <<EOF
$(gradle_files -name gradle.lockfile -o -name '*-gradle.lockfile')
EOF

# The app lockfile was checked for above by its path; what was READ is a
# separate question, and an unread input is not a clean one.
case "$read_locks" in
    *"|$GRADLE/app/gradle.lockfile|"*) ;;
    *) fail "apps/human-android/android/app/gradle.lockfile: not read -- the guard's search missed the app's classpath" ;;
esac

if [ "$problems" -gt 0 ]; then
    printf '\ncheck_android_no_play_services: %d problem(s); ADR-0056 decision 2 (ADR-0057 decision 2, as amended 2026-10-19) forbids these on the application classpath.\n' "$problems" >&2
    exit 1
fi

printf 'check_android_no_play_services: OK -- %d lockfile(s), %d artifact(s), none of com.google.android.gms, com.google.firebase, com.google.android.play, com.android.billingclient or com.google.gms on an application configuration.\n' \
    "$lockfiles" "$artifacts"
