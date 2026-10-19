#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# Self-test for check_android_no_play_services.sh.
#
# Every case starts from a copy of the repository's own Gradle tree --
# the lockfiles the guard reads in CI, not a shape written for the test
# -- and changes one thing in it.

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
GUARD="$SCRIPT_DIR/check_android_no_play_services.sh"
REPO="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
REAL="$REPO/apps/human-android/android"
[ -f "$GUARD" ] || { echo "test: guard not found at $GUARD" >&2; exit 1; }
[ -f "$REAL/app/gradle.lockfile" ] || { echo "test: no real lockfile at $REAL/app/gradle.lockfile" >&2; exit 1; }

failures=0
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

ok()  { echo "  ✓ $1"; }
bad() { echo "  ✗ $1" >&2; failures=$((failures + 1)); }

run_code() { bash "$GUARD" --root "$1" >/dev/null 2>&1; printf '%s' "$?"; }
run()      { bash "$GUARD" --root "$1" 2>&1; }

# A fresh copy of the real Gradle tree's lockfiles and build scripts.
# mktemp, not a counter: tree runs in $(...), so a counter would never
# advance and every case would share one directory.
tree() {
    local r g
    r="$(mktemp -d "$TMP/r.XXXXXX")"
    g="$r/apps/human-android/android"
    mkdir -p "$g/app"
    cp "$REAL/build.gradle.kts" "$REAL/settings.gradle.kts" "$REAL/buildscript-gradle.lockfile" "$REAL/settings-gradle.lockfile" "$g/"
    cp "$REAL/app/build.gradle.kts" "$REAL/app/gradle.lockfile" "$g/app/"
    # The real catalog too: the guard reads it, so a copy of the real tree
    # must carry it.
    cp -r "$REAL/gradle" "$g/"
    printf '%s' "$r"
}
# Adds one artifact line to a lockfile, above its `empty=` line as
# Gradle writes it.
add() { sed -i "/^empty=/i $2" "$1"; }

expect_fail() {  # <root> <what> <text the output must name>
    local out code
    out="$(run "$1")"; code="$(run_code "$1")"
    if [ "$code" = "1" ]; then ok "$2"; else bad "$2: exit $code, want 1: $out"; fi
    case "$out" in
        *"$3"*) ok "  and names $3" ;;
        *) bad "  should name '$3': $out" ;;
    esac
}
expect_pass() {  # <root> <what>
    if [ "$(run_code "$1")" = "0" ]; then ok "$2"; else bad "$2 should pass: $(run "$1")"; fi
}

printf 'test_check_android_no_play_services\n'

echo "a copy of the real tree passes"
R="$(tree)"
expect_pass "$R" "the real lockfiles, copied"

echo "each banned group on the shipped classpath fails"
for coord in \
    com.google.android.gms:play-services-base:18.5.0 \
    com.google.firebase:firebase-messaging:24.1.0 \
    com.google.android.play:integrity:1.4.0 \
    com.android.billingclient:billing:7.1.1 \
    com.google.gms:google-services:4.4.2; do
    R="$(tree)"
    add "$R/apps/human-android/android/app/gradle.lockfile" "$coord=releaseRuntimeClasspath"
    expect_fail "$R" "${coord%%:*} on releaseRuntimeClasspath" "$coord on releaseRuntimeClasspath"
done

echo "a subgroup of a banned group is that group"
R="$(tree)"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.firebase.crashlytics:x:1.0=debugCompileClasspath"
expect_fail "$R" "com.google.firebase.crashlytics" "com.google.firebase.crashlytics:x:1.0 on debugCompileClasspath"

echo "the Google services plugin on the buildscript classpath fails (ADR-0057 A 2026-10-19)"
R="$(tree)"
add "$R/apps/human-android/android/buildscript-gradle.lockfile" "com.google.gms:google-services:4.4.2=classpath"
expect_fail "$R" "com.google.gms on classpath" "buildscript-gradle.lockfile: com.google.gms:google-services:4.4.2 on classpath"

echo "a group that only shares a prefix is not banned"
R="$(tree)"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.gmsx:x:1.0=releaseRuntimeClasspath"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.player:x:1.0=releaseRuntimeClasspath"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.firebaseui:x:1.0=releaseRuntimeClasspath"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.gmsx:x:1.0=releaseRuntimeClasspath"
expect_pass "$R" "com.google.android.gmsx, com.google.android.player, com.google.firebaseui, com.google.gmsx"

echo "lint and test configurations may carry them"
R="$(tree)"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.gms:a:1.0=androidLintTool,lintChecks,debugLintChecksClasspath"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.firebase:b:1.0=debugUnitTestRuntimeClasspath,debugAndroidTestCompileClasspath,androidTestUtil"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.play:c:1.0=unified-test-platform-gradle-work-action"
expect_pass "$R" "lint, unit-test, instrumented-test and test-platform configurations"

echo "a line shared by lint and the application names the application half only"
R="$(tree)"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.gms:d:1.0=androidLintTool,releaseCompileClasspath"
out="$(run "$R")"
expect_fail "$R" "androidLintTool,releaseCompileClasspath" "com.google.android.gms:d:1.0 on releaseCompileClasspath"
case "$out" in *"androidLintTool"*) bad "  should not report the lint configuration: $out" ;; *) ok "  and leaves the lint half out" ;; esac

echo "a configuration nobody has named yet is the application (fail-closed)"
R="$(tree)"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.gms:e:1.0=stagingRuntimeClasspath"
expect_fail "$R" "an unknown configuration" "on stagingRuntimeClasspath"

echo "the build's plugin classpath is not exempt"
R="$(tree)"
add "$R/apps/human-android/android/buildscript-gradle.lockfile" "com.google.android.gms:oss-licenses-plugin:0.10.6=classpath"
expect_fail "$R" "a Play-services plugin on the buildscript classpath" "buildscript-gradle.lockfile: com.google.android.gms:oss-licenses-plugin:0.10.6 on classpath"

echo "the Kotlin compiler's classpaths are not exempt (review of #307, finding 3)"
for conf in kotlinCompilerClasspath kotlinBuildToolsApiClasspath kotlinCompilerPluginClasspathRelease; do
    R="$(tree)"
    add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.gms:k:1.0=$conf"
    expect_fail "$R" "$conf" "com.google.android.gms:k:1.0 on $conf"
done

echo "a lockfile in a new subproject is read"
R="$(tree)"
mkdir -p "$R/apps/human-android/android/lib"
printf '// x\n' > "$R/apps/human-android/android/lib/build.gradle.kts"
printf 'com.android.billingclient:billing:7.1.1=releaseRuntimeClasspath\nempty=\n' > "$R/apps/human-android/android/lib/gradle.lockfile"
expect_fail "$R" "lib/gradle.lockfile" "lib/gradle.lockfile: com.android.billingclient:billing:7.1.1"

echo "a clone under a directory named build or .gradle is still read"
# -path once matched the root's own prefix, so such a clone read nothing
# and printed OK (review of #307, finding 1).
for d in build .gradle; do
    R="$(mktemp -d "$TMP/$d.XXXXXX")"; mkdir -p "$R/$d"
    T="$(tree)"; mv "$T" "$R/$d/clone"; R="$R/$d/clone"
    add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.firebase:firebase-messaging:24.1.0=releaseRuntimeClasspath"
    expect_fail "$R" "a clone below $d/" "com.google.firebase:firebase-messaging:24.1.0 on releaseRuntimeClasspath"
done

echo "an app lockfile the search does not reach is not read, and fails (round 2, finding 3)"
# find -P does not descend a symlinked directory, while [ -f ] follows it.
R="$(tree)"; G="$R/apps/human-android/android"; mv "$G/app" "$R/app-real"; ln -s "$R/app-real" "$G/app"
expect_fail "$R" "a symlinked app directory" "app/gradle.lockfile: not read"

echo "Gradle's output directories below the root are not read"
R="$(tree)"; mkdir -p "$R/apps/human-android/android/app/build/tmp"
printf 'com.google.android.gms:x:1.0=releaseRuntimeClasspath\n' > "$R/apps/human-android/android/app/build/tmp/gradle.lockfile"
expect_pass "$R" "a lockfile under app/build/"

echo "the lock itself must be trustworthy"
R="$(tree)"; rm "$R/apps/human-android/android/app/gradle.lockfile"
expect_fail "$R" "no app lockfile" "app/gradle.lockfile: missing"
R="$(tree)"; sed -i 's/releaseRuntimeClasspath/releaseRuntimeClasspathX/g' "$R/apps/human-android/android/app/gradle.lockfile"
expect_fail "$R" "a lockfile that does not cover the shipped classpath" "names no releaseRuntimeClasspath"
R="$(tree)"; sed -i 's/^\([[:space:]]*\)lockAllConfigurations()/\1\/\/ lockAllConfigurations()/' "$R/apps/human-android/android/build.gradle.kts"
expect_fail "$R" "lockAllConfigurations() commented out" "no lockAllConfigurations()"
R="$(tree)"; sed -i 's/lockAllConfigurations()/lockAllConfigurations()\n        lockMode.set(LockMode.LENIENT)/' "$R/apps/human-android/android/build.gradle.kts"
expect_fail "$R" "LockMode.LENIENT" "LENIENT (line"
R="$(tree)"; mkdir -p "$R/apps/human-android/android/lib"; printf '// x\n' > "$R/apps/human-android/android/lib/build.gradle.kts"
expect_fail "$R" "a subproject with no lockfile" "android/lib: a project with no gradle.lockfile"
R="$(tree)"; mkdir -p "$R/apps/human-android/android/lib"; printf '// x\n' > "$R/apps/human-android/android/lib/build.gradle"
expect_fail "$R" "a Groovy subproject with no lockfile" "android/lib: a project with no gradle.lockfile"

echo "a subproject's script cannot loosen the lock (review of #307, finding 2)"
APP_BUILD=apps/human-android/android/app/build.gradle.kts
for loosen in \
    'dependencyLocking { lockMode.set(LockMode.LENIENT) }:LENIENT' \
    'dependencyLocking { ignoredDependencies.add("com.google.firebase:*") }:ignoredDependencies' \
    'dependencyLocking { unlockAllConfigurations() }:unlockAllConfigurations' \
    'configurations.named("releaseRuntimeClasspath") { resolutionStrategy.deactivateDependencyLocking() }:deactivateDependencyLocking'; do
    R="$(tree)"; printf '\n%s\n' "${loosen%:*}" >> "$R/$APP_BUILD"
    expect_fail "$R" "${loosen##*:} in app/build.gradle.kts" "app/build.gradle.kts: ${loosen##*:}"
    R="$(tree)"; printf '\n// %s\n/*\n%s\n*/\n' "${loosen%:*}" "${loosen%:*}" >> "$R/$APP_BUILD"
    # Read raw: a stripper that does not parse strings once let a glob
    # hide the rest of the file, so a comment naming one fails as well.
    expect_fail "$R" "  the same commented out" "app/build.gradle.kts:"
done
R="$(tree)"; printf '\ndependencyLocking { lockMode.set(LockMode.LENIENT) }\n' >> "$R/apps/human-android/android/settings.gradle.kts"
expect_fail "$R" "LENIENT in settings.gradle.kts" "settings.gradle.kts: LENIENT"

echo "a string that looks like a comment hides nothing (review of #307 round 2, finding 1)"
R="$(tree)"; printf '\nval kt = fileTree("src") { include("**/*.kt") }\nval p = "a//b"\ndependencyLocking { lockMode.set(LockMode.LENIENT) }\n' >> "$R/$APP_BUILD"
expect_fail "$R" "LENIENT after a glob and a // in a string" "LENIENT (line"

echo "moving the lock state elsewhere loosens it too (round 2, finding 2)"
R="$(tree)"; printf '\ndependencyLocking { lockFile.set(file("alt.lock")) }\n' >> "$R/$APP_BUILD"
expect_fail "$R" "lockFile in app/build.gradle.kts" "app/build.gradle.kts:"
R="$(tree)"; printf '\ngradle.startParameter.isWriteDependencyLocks = true\n' >> "$R/apps/human-android/android/settings.gradle.kts"
expect_fail "$R" "startParameter write-locks in settings.gradle.kts" "settings.gradle.kts:"
R="$(tree)"; printf '\nval sp = gradle.startParameter\n' >> "$R/apps/human-android/android/settings.gradle.kts"
expect_fail "$R" "any startParameter use, whatever it then sets" "settings.gradle.kts: startParameter"
# The Java getters, which the case-sensitive startParameter does not match
# (round 3, finding 3).
R="$(tree)"; printf '\ngradle.getStartParameter().setWriteDependencyLocks(true)\n' >> "$R/apps/human-android/android/settings.gradle.kts"
expect_fail "$R" "setWriteDependencyLocks through the getter" "settings.gradle.kts: WriteDependencyLocks"
R="$(tree)"; printf '\ngradle.getStartParameter().setLockedDependenciesToUpdate(listOf("x:y"))\n' >> "$R/apps/human-android/android/settings.gradle.kts"
expect_fail "$R" "setLockedDependenciesToUpdate through the getter" "settings.gradle.kts: LockedDependenciesToUpdate"
R="$(tree)"; printf '\nimport org.gradle.api.artifacts.dsl.LockMode.*\ndependencyLocking { lockMode.set(LENIENT) }\n' >> "$R/$APP_BUILD"
expect_fail "$R" "LENIENT through a star import" "app/build.gradle.kts: LENIENT"
R="$(tree)"; mkdir -p "$R/apps/human-android/android/buildSrc/src/main/kotlin"
printf 'fun Project.x() { dependencyLocking { unlockAllConfigurations() } }\n' > "$R/apps/human-android/android/buildSrc/src/main/kotlin/Locks.kt"
expect_fail "$R" "unlockAllConfigurations in buildSrc Kotlin" "buildSrc/src/main/kotlin/Locks.kt: unlockAllConfigurations"

echo "a banned group declared in the build's own text fails (round 2, risk 1)"
R="$(tree)"; printf '\nplugins { id("com.google.gms.google-services") version "4.4.2" }\n' >> "$R/$APP_BUILD"
expect_fail "$R" "the Google services plugin applied in app" "declares a banned group"
R="$(tree)"; printf '\ndependencies { implementation("com.google.firebase:firebase-messaging:24.1.0") }\n' >> "$R/$APP_BUILD"
expect_fail "$R" "a Firebase implementation dependency" "declares a banned group"
R="$(tree)"; mkdir -p "$R/apps/human-android/android/gradle"
# The key reads like a test configuration on purpose: the catalog cannot
# say where an entry is used, so no line of it is exempt.
printf '[libraries]\ntestFcm = { module = "com.google.firebase:firebase-messaging", version = "24.1.0" }\n' > "$R/apps/human-android/android/gradle/libs.versions.toml"
expect_fail "$R" "a Firebase catalog entry" "gradle/libs.versions.toml: declares a banned group (line 2)"
R="$(tree)"; printf '\ndependencies {\n    testImplementation("com.google.firebase:x:1.0")\n    androidTestImplementation("com.google.android.gms:y:1.0")\n    lintChecks("com.google.android.play:z:1.0")\n    implementation("com.google.gmsx:w:1.0")\n}\n' >> "$R/$APP_BUILD"
expect_pass "$R" "test and lint dependency lines, and a lookalike group"
R="$(tree)"; printf '\ndependencies {\n    testImplementation("junit:junit:4.13"); implementation("com.google.firebase:firebase-messaging:24.1.0")\n}\n' >> "$R/$APP_BUILD"
expect_fail "$R" "a second statement after a test call (round 3, finding 2)" "declares a banned group"
R="$(tree)"; printf '\ntesting { plugins { id("com.google.gms.google-services") } }\n' >> "$R/$APP_BUILD"
expect_fail "$R" "a line that only starts with test" "declares a banned group"
R="$(tree)"; printf '\ndependencies {\n    testDebugImplementation("com.google.firebase:x:1.0")\n    androidTestUtil("com.google.android.gms:y:1.0")\n    androidTestReleaseImplementation("com.google.android.play:z:1.0")\n}\n' >> "$R/$APP_BUILD"
expect_pass "$R" "test configurations with a build type, and androidTestUtil (round 4)"
R="$(tree)"; printf '\ndependencies {\n    testingImplementation("com.google.firebase:x:1.0")\n}\n' >> "$R/$APP_BUILD"
expect_fail "$R" "a configuration that only starts with test (testingImplementation)" "declares a banned group"

echo "lockAllConfigurations() counts only where it locks every project (finding 4)"
ROOT_BUILD=apps/human-android/android/build.gradle.kts
R="$(tree)"; sed -i 's/^allprojects {/\/*\nallprojects {/' "$R/$ROOT_BUILD"; printf '*/\n' >> "$R/$ROOT_BUILD"
expect_fail "$R" "the allprojects block inside /* */" "no lockAllConfigurations() in allprojects"
R="$(tree)"; sed -i 's/^allprojects {/dependencyLocking { lockAllConfigurations() }\nsubprojects {/; s/^\([[:space:]]*\)lockAllConfigurations()$/\1lockMode.set(LockMode.DEFAULT)/' "$R/$ROOT_BUILD"
expect_fail "$R" "lockAllConfigurations() for the root project alone" "no lockAllConfigurations() in allprojects"

R="$(tree)"; sed -i 's/^\([[:space:]]*\)lockAllConfigurations()$/\1lockMode.set(LockMode.DEFAULT)/' "$R/$ROOT_BUILD"; printf '\ndependencyLocking { lockAllConfigurations() }\n' >> "$R/$ROOT_BUILD"
expect_fail "$R" "the call after the allprojects block has closed (round 2, finding 3)" "no lockAllConfigurations() in allprojects"

echo "a string cannot hide or forge a block's brace (review of #307 round 3, finding 1)"
R="$(tree)"; sed -i 's/^\([[:space:]]*\)lockAllConfigurations()$/\1lockMode.set(LockMode.DEFAULT)/' "$R/$ROOT_BUILD"
printf '\nallprojects { repositories { maven { url = uri("https://jitpack.io") } } }\ndependencyLocking { lockAllConfigurations() }\n' >> "$R/$ROOT_BUILD"
expect_fail "$R" "a URL's // in a one-line allprojects block" "no lockAllConfigurations() in allprojects"
R="$(tree)"; sed -i 's/^\([[:space:]]*\)lockAllConfigurations()$/\1lockMode.set(LockMode.DEFAULT)/' "$R/$ROOT_BUILD"
printf '\nallprojects { val g = "src/*" }\nval q = "*/"; dependencyLocking { lockAllConfigurations() }\n' >> "$R/$ROOT_BUILD"
expect_fail "$R" "a glob's /* and a */ in strings" "no lockAllConfigurations() in allprojects"
R="$(tree)"; sed -i 's/^\([[:space:]]*\)lockAllConfigurations()$/\1lockMode.set(LockMode.DEFAULT)/' "$R/$ROOT_BUILD"
printf '\nallprojects { val b = "}" }\ndependencyLocking { lockAllConfigurations() }\n' >> "$R/$ROOT_BUILD"
expect_fail "$R" "a double-quoted string closes at its own quote (the call after the block fails)" "no lockAllConfigurations() in allprojects"
R="$(tree)"; sed -i 's/^allprojects {/allprojects {\n    val note = "a } in a string"\n    val url = "https:\/\/x"/' "$R/$ROOT_BUILD"
expect_pass "$R" "a } and a // in strings inside the real block do not unlock it"

echo "the plugin classpath's lock is checked too"
R="$(tree)"; sed -i 's/^buildscript {/run {/' "$R/$ROOT_BUILD"
expect_fail "$R" "activateDependencyLocking() outside buildscript (round 2, finding 3)" "the buildscript classpath is not locked"
R="$(tree)"; add "$R/apps/human-android/android/buildscript-gradle.lockfile" "com.google.gms.google-services:com.google.gms.google-services.gradle.plugin:4.4.2=classpath"
expect_fail "$R" "the plugin marker, a com.google.gms subgroup (round 2, finding 3)" "com.google.gms.google-services:com.google.gms.google-services.gradle.plugin:4.4.2 on classpath"
R="$(tree)"; sed -i 's/resolutionStrategy.activateDependencyLocking()/\/\/ resolutionStrategy.activateDependencyLocking()/' "$R/$ROOT_BUILD"
expect_fail "$R" "buildscript locking switched off" "the buildscript classpath is not locked"
R="$(tree)"; rm "$R/apps/human-android/android/buildscript-gradle.lockfile"
expect_fail "$R" "no buildscript-gradle.lockfile" "buildscript-gradle.lockfile: missing"

echo "the real repository passes its own guard"
if [ "$(run_code "$REPO")" = "0" ]; then ok "this repository"; else bad "this repository fails: $(run "$REPO")"; fi

echo "--help quotes the decision, its amendment and the five groups"
help="$(bash "$GUARD" --help 2>&1)"; code=$?
[ "$code" = 0 ] && ok "--help exits 0" || bad "--help exit $code"
for s in 'ADR-0056 decision 2' 'com.google.android.gms' 'com.google.firebase' 'com.google.android.play' 'com.android.billingclient' 'com.google.gms`' 'Amendment 2026-10-19' 'Lint and'; do
    case "$help" in *"$s"*) ok "  says $s" ;; *) bad "  --help lacks '$s'" ;; esac
done

echo "invocation errors"
[ "$(run_code "$TMP/nope")" = "2" ] && ok "no such directory" || bad "missing root should exit 2"
if timeout 5 bash "$GUARD" --root >/dev/null 2>&1; then
    bad "--root with no value should be an invocation error"
elif [ "$?" = 124 ]; then
    bad "--root with no value hung instead of failing"
else
    ok "--root with no value"
fi

echo
if [ "$failures" -eq 0 ]; then
    echo "test_check_android_no_play_services: OK — all assertions passed."
    exit 0
fi
echo "test_check_android_no_play_services: FAILED — $failures assertion(s) failed." >&2
exit 1
