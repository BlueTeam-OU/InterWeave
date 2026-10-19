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
#   - the root build.gradle.kts not calling `lockAllConfigurations()`
#     inside `allprojects { dependencyLocking { } }`, or not locking the
#     buildscript classpath (`activateDependencyLocking()`), or no
#     buildscript-gradle.lockfile; comments do not count;
#   - any Gradle script, version catalog or buildSrc/build-logic Kotlin
#     under the root naming LockMode.LENIENT, ignoredDependencies,
#     unlockAllConfigurations, deactivateDependencyLocking, lockFile,
#     startParameter, writeDependencyLocks or LockedDependenciesToUpdate
#     -- read raw, so a comment naming one fails too.
#
# AND THE DECLARATIONS: the same files may not name a banned group
# (a plugins { } id, a dependency coordinate, a catalog entry), except
# on a dependency line of a test or lint configuration. A subproject's
# own plugin classpath is in no lockfile, so this is what refuses the
# Google services plugin applied there.
#
# WHAT IT CANNOT SEE: under Gradle's DEFAULT lock mode a locked
# configuration with NO lock state (a new build type's classpath before
# --write-locks) resolves unvalidated and appears in no lockfile.
# LockMode.STRICT closes that, and is the build's to set; the
# releaseRuntimeClasspath control above covers the one configuration
# that ships today.
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

# A Gradle script with its comments removed and its lines joined, so a
# call inside /* ... */ or after // does not count toward the two REQUIRED
# calls below, and a block can be matched across lines. It does not parse
# strings, so "**/*.kt" or a URL removes text that is code; for a call
# that must be PRESENT that fails closed, which is why nothing is ever
# refused on its output (the loosening scan reads raw text).
code_of() {
    awk '
        {
            line = $0; out = ""
            while (line != "") {
                if (inblock) {
                    e = index(line, "*/")
                    if (e == 0) { line = ""; break }
                    line = substr(line, e + 2); inblock = 0; continue
                }
                b = index(line, "/*"); c = index(line, "//")
                if (b > 0 && (c == 0 || b < c)) {
                    out = out substr(line, 1, b - 1); line = substr(line, b + 2); inblock = 1; continue
                }
                if (c > 0) { out = out substr(line, 1, c - 1); line = "" ; break }
                out = out line; line = ""
            }
            printf "%s ", out
        }
    ' "$1"
}

root_build="$GRADLE/build.gradle.kts"
if [ ! -f "$root_build" ]; then
    fail "apps/human-android/android/build.gradle.kts: missing -- nothing says the configurations are locked"
else
    root_code="$(code_of "$root_build")"
    # Inside allprojects, so every project is locked and not the root
    # alone; [^}]* keeps the match inside the block, and a block with a
    # nested one before the call fails closed.
    grep -Eq 'allprojects[[:space:]]*\{[^}]*dependencyLocking[[:space:]]*\{[^}]*lockAllConfigurations\(\)' <<<"$root_code" \
        || fail "apps/human-android/android/build.gradle.kts: no lockAllConfigurations() in allprojects { dependencyLocking { } } -- a configuration outside the lock is outside this guard"
    grep -Eq 'buildscript[[:space:]]*\{[^}]*activateDependencyLocking\(\)' <<<"$root_code" \
        || fail "apps/human-android/android/build.gradle.kts: the buildscript classpath is not locked -- buildscript-gradle.lockfile would not describe the plugins"
fi
[ -f "$GRADLE/buildscript-gradle.lockfile" ] \
    || fail "apps/human-android/android/buildscript-gradle.lockfile: missing -- the plugin classpath is unread"

# The build's own text: every Gradle script, the version catalog, and the
# Kotlin of a buildSrc or build-logic plugin, under the root.
build_sources() {
    gradle_files -name '*.gradle' -o -name '*.gradle.kts' -o -name '*.toml' \
        -o \( -name '*.kt' \( -path '*/buildSrc/*' -o -path '*/build-logic/*' \) \)
}

# What loosens a lock, read from the RAW text: a subproject's own
# dependencyLocking block is evaluated after the root's allprojects and
# overrides it for that project (review of #307, finding 2). Raw and not
# comment-stripped, because a stripper that does not parse strings reads
# the glob "**/*.kt" as a comment opening and drops the rest of the file
# (review of #307 round 2, finding 1). A comment naming one fails too:
# write the prose another way.
#   LockMode.LENIENT              resolves past the lock without failing
#   ignoredDependencies           neither written to the lock nor checked
#   unlockAllConfigurations       undoes lockAllConfigurations()
#   deactivateDependencyLocking   one configuration out of the lock
#   lockFile                      the lock state read from another file
#   startParameter                write-locks or update-locks on every build
#   writeDependencyLocks / LockedDependenciesToUpdate  the same, by name
LOOSEN='LockMode\.LENIENT|ignoredDependencies|unlockAllConfigurations|deactivateDependencyLocking|lockFile|startParameter|[Ww]riteDependencyLocks|[Ll]ockedDependenciesToUpdate'

# A banned group named in the build's own text, whatever resolves: a
# subproject's plugins { id(...) version ... } resolves into that
# project's buildscript classpath, which no lockfile here records
# (review of #307 round 2, risk 1), so the declaration is refused where
# it is written. A dependency line on a test or lint configuration may
# name one, as decision 2 allows; the catalog cannot say where an entry
# is used, so it may not.
BANNED='(com\.google\.android\.gms|com\.google\.firebase|com\.google\.android\.play|com\.android\.billingclient|com\.google\.gms)([.:"'"'"'/[:space:]]|$)'
TOOLING_LINE='^[0-9]+:[[:space:]]*"?(test|androidTest|lintChecks|lintPublish|[a-z]+UnitTest|[a-z]+AndroidTest)[A-Za-z]*"?[[:space:](]'

sources=0
while IFS= read -r src; do
    [ -n "$src" ] || continue
    sources=$((sources + 1))
    rel="${src#"$ROOT"/}"
    hits="$(grep -noE "$LOOSEN" "$src" || true)"
    while IFS= read -r h; do
        [ -n "$h" ] && fail "$rel: ${h#*:} (line ${h%%:*}) -- the lockfile would no longer describe what resolves"
    done <<<"$hits"
    hits="$(grep -nE "$BANNED" "$src" || true)"
    case "$src" in *.toml) ;; *) hits="$(grep -vE "$TOOLING_LINE" <<<"$hits" || true)" ;; esac
    while IFS= read -r h; do
        [ -n "$h" ] && fail "$rel: declares a banned group (line ${h%%:*}) -- ${h#*:}"
    done <<<"$hits"
done <<EOF
$(build_sources)
EOF
[ "$sources" -gt 0 ] || fail "apps/human-android/android: no Gradle script read"

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
