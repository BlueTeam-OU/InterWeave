#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/test_check_android_contacts_and_backup.sh
#
# Self-test for check_android_contacts_and_backup.py. Builds throwaway
# roots under $TMPDIR from copies of the real manifest, extraction rules
# and app lockfile, and breaks each declaration once (plan §29 exit gate
# (b), "the tree check, broken on purpose once per declaration and once
# per allow-list failure: an extra permission, and the camera feature
# declared required"). The
# unbroken copy passes first, so a check that fails everything cannot
# pass this test.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK="$SCRIPT_DIR/check_android_contacts_and_backup.py"
REAL="$SCRIPT_DIR/../.."

pass=0
fail=0
ok()  { printf '  ✓ %s\n' "$1"; pass=$((pass + 1)); }
bad() { printf '  ✗ %s\n' "$1" >&2; fail=$((fail + 1)); }

TMP="$(mktemp -d)"
[ -n "$TMP" ] && [ -d "$TMP" ] || { echo "test_check_android_contacts_and_backup: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$TMP"' EXIT

# Captured, never piped into `grep -q`: pipefail plus an early-exiting
# grep turns correct output into a failed pipeline.
run()      { python3 "$CHECK" --root "$1" 2>&1; }
run_code() { python3 "$CHECK" --root "$1" >/dev/null 2>&1; printf '%s' "$?"; }

APP=apps/human-android/android/app
MAIN="$APP/src/main"

make_root() {
    local r="$1"
    mkdir -p "$r/$MAIN/res/xml" "$r/$MAIN/kotlin/org/interweave/human" "$r/crates/human/android-platform/src" "$r/apps/human-android/src"
    cp "$REAL/$MAIN/AndroidManifest.xml" "$r/$MAIN/AndroidManifest.xml"
    cp "$REAL/$MAIN/res/xml/data_extraction_rules.xml" "$r/$MAIN/res/xml/data_extraction_rules.xml"
    cp "$REAL/$APP/gradle.lockfile" "$r/$APP/gradle.lockfile"
    printf 'package org.interweave.human\nclass Plain\n' > "$r/$MAIN/kotlin/org/interweave/human/Plain.kt"
    printf '// SPDX-License-Identifier: Apache-2.0\npub fn plain() {}\n' > "$r/crates/human/android-platform/src/lib.rs"
}

# edit <file> <python expression over s>: rewrite a file in place.
edit() {
    python3 - "$1" "$2" <<'PY'
import sys
p, expr = sys.argv[1], sys.argv[2]
s = open(p, encoding="utf-8").read()
t = eval(expr)
if t == s:
    sys.exit("the planted edit changed nothing")
open(p, "w", encoding="utf-8").write(t)
PY
}

# fails <name> <expected text> <setup...>: a fresh root, the setup run in
# it, and the check must exit 1 naming the expected text.
fails() {
    local name="$1" want="$2"; shift 2
    local r="$TMP/$name"
    make_root "$r"
    ( cd "$r" && eval "$*" ) || { bad "$name (could not plant)"; return; }
    local out code
    out="$(run "$r")"; code="$(run_code "$r")"
    if [ "$code" = "1" ] && [[ "$out" == *"$want"* ]]; then ok "$name"; else bad "$name (exit $code): $out"; fi
}

M="$MAIN/AndroidManifest.xml"
X="$MAIN/res/xml/data_extraction_rules.xml"

R="$TMP/control"; make_root "$R"
[ "$(run_code "$R")" = "0" ] && ok "the real manifest, rules and lockfile pass" || bad "control should pass: $(run "$R")"

for perm in READ_CONTACTS WRITE_CONTACTS GET_ACCOUNTS; do
    fails "permission-$perm" "declares android.permission.$perm" \
        "edit $M \"s.replace('<uses-permission android:name=\\\"android.permission.INTERNET\\\" />', '<uses-permission android:name=\\\"android.permission.INTERNET\\\" /><uses-permission android:name=\\\"android.permission.$perm\\\" />')\""
done
fails "permission-sdk-23" "<uses-permission-sdk-23> declares android.permission.READ_CONTACTS" \
    "edit $M \"s.replace('<application', '<uses-permission-sdk-23 android:name=\\\"android.permission.READ_CONTACTS\\\" /><application', 1)\""
fails "allow-backup-true" 'android:allowBackup="true"' \
    "edit $M \"s.replace('android:allowBackup=\\\"false\\\"', 'android:allowBackup=\\\"true\\\"')\""
fails "allow-backup-absent" 'does not set android:allowBackup="false"' \
    "edit $M \"s.replace('android:allowBackup=\\\"false\\\"', '')\""
fails "extraction-rules-absent" "sets no android:dataExtractionRules" \
    "edit $M \"s.replace('android:dataExtractionRules=\\\"@xml/data_extraction_rules\\\"', '')\""
for attr in backupAgent fullBackupContent fullBackupOnly; do
    fails "attr-$attr" "sets android:$attr" \
        "edit $M \"s.replace('android:hasCode=\\\"true\\\"', 'android:hasCode=\\\"true\\\" android:$attr=\\\"x\\\"')\""
done
fails "cloud-domain-missing" "<cloud-backup> does not exclude the domain(s) device_root" \
    "edit $X \"s.replace('<exclude domain=\\\"device_root\\\" path=\\\".\\\" />', '', 1)\""
fails "transfer-domain-missing" "<device-transfer> does not exclude the domain(s) external" \
    "edit $X \"s[:s.index('<device-transfer>')] + s[s.index('<device-transfer>'):].replace('<exclude domain=\\\"external\\\" path=\\\".\\\" />', '', 1)\""
fails "transfer-narrowed-path" "<device-transfer> does not exclude the domain(s) file" \
    "edit $X \"s[:s.index('<device-transfer>')] + s[s.index('<device-transfer>'):].replace('<exclude domain=\\\"file\\\" path=\\\".\\\" />', '<exclude domain=\\\"file\\\" path=\\\"cache\\\" />', 1)\""
fails "include-readmits" "re-admits data" \
    "edit $X \"s.replace('</cloud-backup>', '<include domain=\\\"file\\\" path=\\\"store\\\" /></cloud-backup>')\""
fails "section-missing" "0 <device-transfer> sections" \
    "edit $X \"s[:s.index('<device-transfer>')] + s[s.index('</device-transfer>') + len('</device-transfer>'):]\""
fails "rules-file-missing" "the extraction-rules file the main manifest names is missing" "rm $X"
fails "manifest-missing" "missing; an absent manifest is not a clean one" "rm $M"
fails "manifest-unparsable" "does not parse as XML" "edit $M \"s.replace('</manifest>', '')\""
fails "provider-kotlin" "names ContactsContract" \
    "printf 'import android.provider.ContactsContract\n' >> $MAIN/kotlin/org/interweave/human/Plain.kt"
fails "provider-rust" "names com.android.contacts" \
    "printf '// content://com.android.contacts/data\n' >> crates/human/android-platform/src/lib.rs"
fails "provider-authority-manifest" "names com.android.contacts" \
    "edit $M \"s.replace('</application>', '<provider android:name=\\\"x\\\" android:authorities=\\\"com.android.contacts\\\" /></application>')\""
fails "library-in-lockfile" "a dependency coordinate names a contacts library" \
    "printf 'com.example:contacts-picker:1.0=releaseRuntimeClasspath\n' >> $APP/gradle.lockfile"
fails "library-in-gradle" "a dependency coordinate names a contacts library" \
    "printf 'dependencies { implementation(\"com.example.contact:sync:1.0\") }\n' > $APP/build.gradle.kts"
fails "no-lockfile" "no Gradle lockfile read" "rm $APP/gradle.lockfile"
fails "variant-manifest-backup" 'android:allowBackup="true"' \
    "mkdir -p $APP/src/release && printf '<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\"><application android:allowBackup=\"true\" /></manifest>\n' > $APP/src/release/AndroidManifest.xml"

fails "variant-manifest-rules" "sets android:dataExtractionRules in a variant manifest" \
    "mkdir -p $APP/src/release && printf '<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\"><application android:dataExtractionRules=\"@xml/other\" /></manifest>\n' > $APP/src/release/AndroidManifest.xml"
fails "variant-rules-file" "release/res/xml/data_extraction_rules.xml: an <include" \
    "mkdir -p $APP/src/release/res/xml && cp $X $APP/src/release/res/xml/data_extraction_rules.xml && edit $APP/src/release/res/xml/data_extraction_rules.xml \"s.replace('<cloud-backup>', '<cloud-backup><include domain=\\\"file\\\" path=\\\".\\\" />')\""
fails "qualified-rules-file" "xml-v31/data_extraction_rules.xml: an <include" \
    "mkdir -p $MAIN/res/xml-v31 && cp $X $MAIN/res/xml-v31/data_extraction_rules.xml && edit $MAIN/res/xml-v31/data_extraction_rules.xml \"s.replace('<cloud-backup>', '<cloud-backup><include domain=\\\"file\\\" path=\\\".\\\" />')\""
fails "provider-legacy-uri" "names content://contacts" \
    "printf 'val u = \"content://contacts/people\"\n' >> $MAIN/kotlin/org/interweave/human/Plain.kt"

# The permission allow-list (plan §29 step 2): an extra permission, by
# either tag; one the list names left out; a permission defined.
fails "allow-list-extra" "declares android.permission.READ_PHONE_STATE, which the permission allow-list does not name" \
    "edit $M \"s.replace('<application', '<uses-permission android:name=\\\"android.permission.READ_PHONE_STATE\\\" /><application', 1)\""
fails "allow-list-extra-sdk-23" "<uses-permission-sdk-23> declares android.permission.ACCESS_FINE_LOCATION, which the permission allow-list does not name" \
    "edit $M \"s.replace('<application', '<uses-permission-sdk-23 android:name=\\\"android.permission.ACCESS_FINE_LOCATION\\\" /><application', 1)\""
fails "allow-list-extra-variant" "release/AndroidManifest.xml: <uses-permission> declares android.permission.RECORD_AUDIO" \
    "mkdir -p $APP/src/release && printf '<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\"><uses-permission android:name=\"android.permission.RECORD_AUDIO\" /></manifest>\n' > $APP/src/release/AndroidManifest.xml"
fails "allow-list-missing" "does not declare android.permission.USE_BIOMETRIC" \
    "edit $M \"s.replace('<uses-permission android:name=\\\"android.permission.USE_BIOMETRIC\\\" />', '')\""
fails "permission-defined" "<permission> defines org.interweave.human.SEND" \
    "edit $M \"s.replace('<application', '<permission android:name=\\\"org.interweave.human.SEND\\\" /><application', 1)\""
# The camera feature: declared required, its required left out (true),
# not declared, and a camera feature a variant declares required.
fails "camera-required" 'android.hardware.camera is required (android:required="true")' \
    "edit $M \"s.replace('<uses-feature android:name=\\\"android.hardware.camera\\\" android:required=\\\"false\\\" />', '<uses-feature android:name=\\\"android.hardware.camera\\\" android:required=\\\"true\\\" />')\""
fails "camera-required-by-default" "android.hardware.camera is required (no android:required, which means true)" \
    "edit $M \"s.replace('<uses-feature android:name=\\\"android.hardware.camera\\\" android:required=\\\"false\\\" />', '<uses-feature android:name=\\\"android.hardware.camera\\\" />')\""
fails "camera-feature-absent" 'no <uses-feature android:name="android.hardware.camera" android:required="false"/> beside CAMERA' \
    "edit $M \"s.replace('<uses-feature android:name=\\\"android.hardware.camera\\\" android:required=\\\"false\\\" />', '')\""
fails "camera-any-required-variant" "release/AndroidManifest.xml: <uses-feature> android.hardware.camera.any is required" \
    "mkdir -p $APP/src/release && printf '<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\"><uses-feature android:name=\"android.hardware.camera.any\" android:required=\"true\" /></manifest>\n' > $APP/src/release/AndroidManifest.xml"
# A test source set's manifest is test tooling, not the shipped app.
R="$TMP/androidtest"; make_root "$R"
mkdir -p "$R/$APP/src/androidTest"
printf '<manifest xmlns:android="http://schemas.android.com/apk/res/android"><uses-permission android:name="android.permission.READ_CONTACTS" /></manifest>\n' > "$R/$APP/src/androidTest/AndroidManifest.xml"
[ "$(run_code "$R")" = "0" ] && ok "an androidTest manifest is not read" || bad "androidTest should be exempt: $(run "$R")"

[ "$(python3 "$CHECK" --bogus >/dev/null 2>&1; echo $?)" = "2" ] && ok "an unknown argument exits 2" || bad "unknown argument should exit 2"
help_out="$(python3 "$CHECK" --help 2>/dev/null)"
[[ "$help_out" == *"ADR-0059 decision 6"* ]] && ok "--help prints the help block" || bad "--help should print help"

printf '\n'
if [ "$fail" -gt 0 ]; then
    printf 'test_check_android_contacts_and_backup: %d passed, %d FAILED.\n' "$pass" "$fail" >&2
    exit 1
fi
printf 'test_check_android_contacts_and_backup: OK — all %d assertions passed.\n' "$pass"
