#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/check_apk_classes.py
#
# >>> help
# Check which Java classes an APK holds, by its own dex files:
#
#   tools/ci/check_apk_classes.py <apk> [--holds <class>]... [--holds-none <package>]...
#
# The classes an APK holds are the class definitions of its classes*.dex
# entries (classes.dex, classes2.dex, ...): each dex's class_defs, named
# through its type and string tables. A class only referenced is not held.
#
#   --holds <class>         the APK must define <class> (a binary name,
#                           org.interweave.human.MainActivity), else exit 1
#   --holds-none <package>  the APK must define no class in <package> or a
#                           package under it (org.interweave.human.test:
#                           its nested classes too), else exit 1 naming each
#
# At least one condition is required. The android apk job holds the
# release APK to none of the instrumentation's package (DECISION
# 01a12770: the test-only JNI entry, org.interweave.human.test.E2eCase,
# is the androidTest APK's alone) and to its main Activity, which shows
# the dex was read; and the androidTest APK to E2eCase, which shows a
# class of that package is seen where it is.
#
# Fails closed: an APK with no classes*.dex, a dex whose magic is not
# 035, 037, 038 or 039 (041's container holds several dex files in one,
# and this reader would see the first), or a table pointing outside its
# file is exit 2, never a pass.
#
# Exit codes:
#   0  every condition holds
#   1  one does not (named on stderr)
#   2  usage, or an APK or a dex this cannot read
# <<< help

import re
import struct
import sys
import zipfile
from pathlib import Path

DEX_ENTRY = re.compile(r"classes[0-9]*\.dex")
DEX_VERSIONS = {b"035", b"037", b"038", b"039"}


class Unreadable(Exception):
    pass


def help_text() -> str:
    lines = Path(__file__).read_text().splitlines()
    start = lines.index("# >>> help") + 1
    end = lines.index("# <<< help")
    return "\n".join(line[2:] for line in lines[start:end]) + "\n"


def uleb128(data: bytes, at: int) -> tuple[int, int]:
    value = shift = 0
    while True:
        byte = data[at]
        at += 1
        value |= (byte & 0x7F) << shift
        if byte < 0x80:
            return value, at
        shift += 7
        if shift > 28:
            raise Unreadable("a uleb128 longer than five bytes")


def defined_classes(dex: bytes) -> list[str]:
    """The binary names of the classes dex defines."""
    if len(dex) < 0x70 or dex[:4] != b"dex\n" or dex[7] != 0:
        raise Unreadable("not a dex file")
    if dex[4:7] not in DEX_VERSIONS:
        raise Unreadable(f"dex version {dex[4:7].decode('ascii', 'replace')} is not one this reads")
    try:
        strings_size, strings_off, types_size, types_off = struct.unpack_from("<4I", dex, 0x38)
        defs_size, defs_off = struct.unpack_from("<2I", dex, 0x60)
        names = []
        for i in range(defs_size):
            (type_idx,) = struct.unpack_from("<I", dex, defs_off + 32 * i)
            if type_idx >= types_size:
                raise Unreadable(f"class_def {i} names type {type_idx} of {types_size}")
            (string_idx,) = struct.unpack_from("<I", dex, types_off + 4 * type_idx)
            if string_idx >= strings_size:
                raise Unreadable(f"type {type_idx} names string {string_idx} of {strings_size}")
            (data_off,) = struct.unpack_from("<I", dex, strings_off + 4 * string_idx)
            _, start = uleb128(dex, data_off)
            end = dex.index(b"\0", start)
            # A class descriptor is ASCII in practice; MUTF-8 decodes as
            # UTF-8 for everything but NUL and supplementary characters.
            descriptor = dex[start:end].decode("utf-8", "replace")
            if not (descriptor.startswith("L") and descriptor.endswith(";")):
                raise Unreadable(f"class_def {i} is not a class: {descriptor!r}")
            names.append(descriptor[1:-1].replace("/", "."))
        return names
    except (struct.error, IndexError, ValueError) as error:
        raise Unreadable(f"a table points outside the file ({error})") from error


def apk_classes(apk: str) -> tuple[int, set[str]]:
    try:
        with zipfile.ZipFile(apk) as archive:
            entries = [name for name in archive.namelist() if DEX_ENTRY.fullmatch(name)]
            if not entries:
                raise Unreadable("no classes*.dex in it")
            classes: set[str] = set()
            for name in entries:
                try:
                    classes.update(defined_classes(archive.read(name)))
                except Unreadable as error:
                    raise Unreadable(f"{name}: {error}") from error
            return len(entries), classes
    except (OSError, zipfile.BadZipFile) as error:
        raise Unreadable(str(error)) from error


def main(argv: list[str]) -> int:
    if argv[:1] in (["-h"], ["--help"]):
        sys.stdout.write(help_text())
        return 0
    usage = "usage: check_apk_classes.py <apk> [--holds <class>]... [--holds-none <package>]..."
    if not argv or argv[0].startswith("-"):
        print(usage, file=sys.stderr)
        return 2
    apk, rest = argv[0], argv[1:]
    holds: list[str] = []
    holds_none: list[str] = []
    while rest:
        if len(rest) < 2 or rest[0] not in ("--holds", "--holds-none") or not rest[1]:
            print(usage, file=sys.stderr)
            return 2
        (holds if rest[0] == "--holds" else holds_none).append(rest[1])
        rest = rest[2:]
    if not holds and not holds_none:
        print(usage, file=sys.stderr)
        return 2
    try:
        dex_count, classes = apk_classes(apk)
    except Unreadable as error:
        print(f"check_apk_classes: {apk}: {error}", file=sys.stderr)
        return 2
    print(f"{apk}: {len(classes)} classes in {dex_count} dex files")
    failed = False
    for name in holds:
        if name in classes:
            print(f"  holds {name}")
        else:
            print(f"check_apk_classes: {apk} does not hold {name}", file=sys.stderr)
            failed = True
    for package in holds_none:
        found = sorted(c for c in classes if c.startswith(package + "."))
        if found:
            print(f"check_apk_classes: {apk} holds {len(found)} classes of {package}:", file=sys.stderr)
            for name in found:
                print(f"  {name}", file=sys.stderr)
            failed = True
        else:
            print(f"  holds none of {package}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
