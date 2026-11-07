#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_check_apk_classes.sh
#
# Self-test for check_apk_classes.py, against APKs built here: zips whose
# classes*.dex are written by a small dex writer below, with the class
# definitions and the merely referenced types each case chooses. A class
# of the refused package is named and refused, a nested one too, in a
# second dex too; a type only referenced is not held; a package whose
# name merely starts the same (org.interweave.human.testing) is not
# under it; --holds passes on a defined class and fails on a referenced
# one; and an APK with no dex, a dex of another version (041's
# container), a bad magic, a table pointing outside its file and a bad
# command line are each exit 2. The reader against real dex files
# (d8's, and a published release APK's against dexdump's count) was
# measured by hand when it was written; the android apk job's control
# (the androidTest APK holds E2eCase) is what keeps that live.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/check_apk_classes.py"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_check_apk_classes: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# The dex writer: mkapk.py <apk> <entry>=<spec> ..., where a spec is
# comma-separated items: a binary name defines that class, ~name only
# references it (a type with no class_def), and the flags v041, badmagic
# and badoffset break the file the way their names say; badtype and
# badstring declare the type or the string table one entry shorter than
# written and point the first class_def (or its type) at that last entry,
# so the slot past the declared table still reads a valid descriptor and
# only the bound check can refuse it.
cat > "$SANDBOX/mkapk.py" <<'PYEOF'
import struct, sys, zipfile

def dex(spec):
    items = [i for i in spec.split(",") if i]
    flags = {i for i in items if i in ("v041", "badmagic", "badoffset", "badtype", "badstring")}
    names = [i for i in items if i not in flags]
    descriptors = ["L" + n.lstrip("~").replace(".", "/") + ";" for n in names]
    defined = [k for k, n in enumerate(names) if not n.startswith("~")]
    n = len(descriptors)
    strings_off = 0x70
    types_off = strings_off + 4 * n
    defs_off = types_off + 4 * n
    data_off = defs_off + 32 * len(defined)
    data, offsets = b"", []
    for d in descriptors:
        offsets.append(data_off + len(data))
        data += bytes([len(d)]) + d.encode() + b"\0"
    if "badoffset" in flags:
        offsets[0] = 0x7FFFFFF0
    strings_size = types_size = n
    type_strings = list(range(n))
    def_types = list(defined)
    if "badstring" in flags:
        strings_size = n - 1
        type_strings[def_types[0]] = n - 1
    if "badtype" in flags:
        types_size = n - 1
        def_types[0] = n - 1
    header = bytearray(0x70)
    header[0:8] = b"dex\n041\0" if "v041" in flags else b"dey\n039\0" if "badmagic" in flags else b"dex\n039\0"
    struct.pack_into("<4I", header, 0x38, strings_size, strings_off, types_size, types_off)
    struct.pack_into("<2I", header, 0x60, len(defined), defs_off)
    body = b"".join(struct.pack("<I", o) for o in offsets)
    body += b"".join(struct.pack("<I", k) for k in type_strings)
    body += b"".join(struct.pack("<I", k) + bytes(28) for k in def_types)
    struct.pack_into("<I", header, 0x20, 0x70 + len(body) + len(data))
    return bytes(header) + body + data

with zipfile.ZipFile(sys.argv[1], "w") as z:
    z.writestr("AndroidManifest.xml", b"\0")
    for arg in sys.argv[2:]:
        entry, spec = arg.split("=", 1)
        z.writestr(entry, dex(spec) if entry.endswith(".dex") else b"x")
PYEOF
mkapk() { python3 "$SANDBOX/mkapk.py" "$@" || { echo "test_check_apk_classes: mkapk failed for $1" >&2; exit 1; }; }
run() { python3 "$UNDER_TEST" "$@" 2>&1; }

MAIN=org.interweave.human.MainActivity
TESTPKG=org.interweave.human.test

echo "check_apk_classes: a release-shaped APK"
mkapk "$SANDBOX/release.apk" "classes.dex=$MAIN,org.interweave.human.Native,~$TESTPKG.E2eCase" \
    "lib/arm64-v8a/libx.so=x"
out="$(run "$SANDBOX/release.apk" --holds "$MAIN" --holds-none "$TESTPKG")"; rc=$?
[[ $rc -eq 0 ]] && pass "holds the Activity, none of the test package: exit 0" || fail "rc $rc" "$out"
grep -q "2 classes in 1 dex files" <<<"$out" && pass "counts what it read: 2 classes, 1 dex" || fail "count" "$out"
out="$(run "$SANDBOX/release.apk" --holds "$TESTPKG.E2eCase")"; rc=$?
[[ $rc -eq 1 ]] && grep -q "does not hold $TESTPKG.E2eCase" <<<"$out" \
    && pass "a type only referenced is not held (--holds on it: exit 1)" || fail "referenced counted as held (rc $rc)" "$out"
out="$(run "$SANDBOX/release.apk" --holds org.interweave.human.Nope)"; rc=$?
[[ $rc -eq 1 ]] && pass "--holds on an absent class: exit 1" || fail "rc $rc" "$out"

echo "check_apk_classes: the test package is refused, every class of it named"
mkapk "$SANDBOX/leak.apk" "classes.dex=$MAIN" "classes2.dex=$TESTPKG.E2eCase,$TESTPKG.E2eCase\$Inner,$TESTPKG.sub.Deep"
out="$(run "$SANDBOX/leak.apk" --holds "$MAIN" --holds-none "$TESTPKG")"; rc=$?
[[ $rc -eq 1 ]] && pass "a class in classes2.dex: exit 1" || fail "rc $rc" "$out"
for c in "$TESTPKG.E2eCase" "$TESTPKG.E2eCase\$Inner" "$TESTPKG.sub.Deep"; do
    grep -qxF "  $c" <<<"$out" && pass "names $c" || fail "does not name $c" "$out"
done
grep -q "holds 3 classes of $TESTPKG" <<<"$out" && pass "counts the three" || fail "count" "$out"
grep -q "in 2 dex files" <<<"$out" && pass "reads both dex entries" || fail "dex count" "$out"

echo "check_apk_classes: a package is a prefix of names, not of strings"
mkapk "$SANDBOX/near.apk" "classes.dex=$MAIN,org.interweave.human.testing.Thing,org.interweave.human.Test"
out="$(run "$SANDBOX/near.apk" --holds-none "$TESTPKG")"; rc=$?
[[ $rc -eq 0 ]] && pass "org.interweave.human.testing and .Test are not under $TESTPKG" || fail "rc $rc" "$out"

echo "check_apk_classes: what it cannot read is exit 2, never a pass"
mkapk "$SANDBOX/nodex.apk" "lib/arm64-v8a/libx.so=x" "assets/classes.dex=$TESTPKG.E2eCase"
out="$(run "$SANDBOX/nodex.apk" --holds-none "$TESTPKG")"; rc=$?
[[ $rc -eq 2 ]] && grep -q "no classes\*.dex" <<<"$out" \
    && pass "no root classes*.dex (one under assets/ is not the APK's): exit 2" || fail "rc $rc" "$out"
for flag in v041 badmagic badoffset badtype badstring; do
    mkapk "$SANDBOX/$flag.apk" "classes.dex=$MAIN,$TESTPKG.E2eCase,~$TESTPKG.Hidden,$flag"
    out="$(run "$SANDBOX/$flag.apk" --holds-none "$TESTPKG")"; rc=$?
    [[ $rc -eq 2 ]] && grep -q "classes.dex: " <<<"$out" && pass "$flag: exit 2, naming the entry" || fail "$flag: rc $rc" "$out"
done
printf 'not a zip' > "$SANDBOX/junk.apk"
out="$(run "$SANDBOX/junk.apk" --holds-none "$TESTPKG")"; rc=$?
[[ $rc -eq 2 ]] && pass "not a zip: exit 2" || fail "rc $rc" "$out"
out="$(run "$SANDBOX/missing.apk" --holds-none "$TESTPKG")"; rc=$?
[[ $rc -eq 2 ]] && pass "no such file: exit 2" || fail "rc $rc" "$out"

echo "check_apk_classes: usage"
for args in "" "$SANDBOX/release.apk" "$SANDBOX/release.apk --holds" "$SANDBOX/release.apk --holds-none ''" \
            "$SANDBOX/release.apk --nope x" "--holds $MAIN" "$SANDBOX/release.apk --holds-none $TESTPKG." \
            "$SANDBOX/release.apk --holds-none org..interweave" "$SANDBOX/release.apk --holds org/interweave/Main"; do
    # shellcheck disable=SC2086 # the cases are word lists
    out="$(eval run $args)"; rc=$?
    [[ $rc -eq 2 ]] && pass "'${args#"$SANDBOX/"}': exit 2" || fail "'$args': rc $rc" "$out"
done
out="$(python3 "$UNDER_TEST" --help)"; rc=$?
[[ $rc -eq 0 ]] && grep -q -- "--holds-none <package>" <<<"$out" && pass "--help prints the help" || fail "help rc $rc" "$out"

if [[ $failures -ne 0 ]]; then
    echo "test_check_apk_classes: $failures failed" >&2
    exit 1
fi
echo "test_check_apk_classes: all passed"
