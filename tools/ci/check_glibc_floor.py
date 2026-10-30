#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/check_glibc_floor.py
#
# >>> help
# Refuse an executable that needs a newer glibc than the release's floor:
#
#   tools/ci/check_glibc_floor.py <floor> <file>...
#
# <floor> is a glibc version, 2.35 say. For each <file> the versions it
# requires of the C library -- the GLIBC_* names of its .gnu.version_r,
# which the dynamic loader checks before main and refuses with "version
# `GLIBC_x' not found" -- are read with `readelf -V -W`, and the highest
# is printed beside the file's name. A file needing one above the floor
# is named with the symbols' versions that put it there.
#
# Fails closed: a GLIBC_ABI_* marker is read as the release that
# introduced it (GLIBC_ABI_DT_RELR, packed relative relocations, as
# 2.36), and any other GLIBC_* name it cannot order (GLIBC_PRIVATE, a
# new marker) is a refusal. Every requirement counts, a weak symbol's
# too (Rust's std reaches pidfd_spawnp@GLIBC_2.39 weakly): the flags are
# not read, since a program that starts without one of its functions is
# no promise it runs.
# A file with no version requirements at all (static) needs nothing.
# Only the C library's names are read: libgcc_s's GCC_* and libstdc++'s
# GLIBCXX_* come from the same build image as glibc and are not checked
# here; the floor image's run of every program is what covers them.
#
# Exit codes:
#   0  every file is at or under the floor
#   1  a file needs more than the floor, or a name it cannot order
#   2  usage, or a file readelf cannot read as ELF
# <<< help

import re
import subprocess
import sys
from pathlib import Path

VERSION = re.compile(r"[0-9]+(\.[0-9]+)+")
NEED = re.compile(r"^\s*0x[0-9a-f]+:\s+Name: (\S+)\s+Flags: (\S+)", re.M)
# The release each loader marker first shipped in (glibc NEWS).
MARKERS = {"GLIBC_ABI_DT_RELR": "2.36"}


def help_text() -> str:
    lines = Path(__file__).read_text().splitlines()
    start = lines.index("# >>> help") + 1
    end = lines.index("# <<< help")
    return "\n".join(line[2:] for line in lines[start:end]) + "\n"


def key(version: str) -> tuple[int, ...]:
    return tuple(int(part) for part in version.split("."))


def needs(path: str) -> list[str]:
    """The GLIBC_* names path's .gnu.version_r asks for."""
    done = subprocess.run(["readelf", "-V", "-W", path], capture_output=True, text=True)
    if done.returncode != 0 or "Not an ELF file" in done.stderr:
        print(f"check_glibc_floor: {path}: not readable as ELF: {done.stderr.strip()}", file=sys.stderr)
        sys.exit(2)
    section = done.stdout.split("Version needs section", 1)
    if len(section) < 2:
        return []
    return [name for name, _flags in NEED.findall(section[1]) if name.startswith("GLIBC_")]


def main(argv: list[str]) -> int:
    if len(argv) >= 1 and argv[0] in ("-h", "--help"):
        sys.stdout.write(help_text())
        return 0
    if len(argv) < 2 or not VERSION.fullmatch(argv[0]):
        print("usage: check_glibc_floor.py <floor> <file>...", file=sys.stderr)
        return 2
    floor = key(argv[0])
    refused = 0
    for path in argv[1:]:
        ordered: list[tuple[tuple[int, ...], str]] = []
        unknown: list[str] = []
        for name in sorted(set(needs(path))):
            version = MARKERS.get(name, name.removeprefix("GLIBC_"))
            if VERSION.fullmatch(version):
                ordered.append((key(version), name))
            else:
                unknown.append(name)
        highest = max(ordered)[1] if ordered else "none"
        print(f"{path}: {highest}")
        over = [name for k, name in sorted(ordered) if k > floor]
        if over:
            print(f"check_glibc_floor: {path} needs {', '.join(over)}, above the floor {argv[0]}", file=sys.stderr)
            refused += 1
        if unknown:
            print(f"check_glibc_floor: {path} needs {', '.join(unknown)}, which has no version to compare", file=sys.stderr)
            refused += 1
    return 1 if refused else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
