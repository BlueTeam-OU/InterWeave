#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/check_security_matrix.py
#
# >>> help
# Does every test tests/SECURITY-MATRIX.md names still exist?
#
#   tools/checks/check_security_matrix.py
#   tools/checks/check_security_matrix.py --strict     # Stage 18's close
#   tools/checks/check_security_matrix.py --root <dir>
#
# Each row of the matrix names the tests that guard one threat (plan
# section 21, ruling R1). A test renamed or deleted would empty its row
# with nothing red, so this reads every row and checks:
#   - the id is S21-NN, TM-NN or C-NN, and no id repeats;
#   - the row has the five cells | id | guards | step | tests | limit |;
#   - every tests item is `path::fn` or an owed marker ("owed",
#     "owed: <text>", "owed (<text>)", "—"), items separated by "; ";
#   - each path is a file inside the repository, and it defines fn on a
#     line that is not a comment: `\bfn <name>\b` on a line whose first
#     non-blank characters are not `//`, so neither a doc comment nor a
#     commented-out function vouches for a deleted test.
# A row with no `path::fn` at all is empty: reported always, and a
# failure only under --strict, which Stage 18's close runs.
#
# Exit codes: 0 every named test exists (and, under --strict, no row is
# empty); 1 a problem, each on its own line; 2 usage.
# <<< help
from __future__ import annotations

import argparse
import os
import re
import sys

MATRIX = "tests/SECURITY-MATRIX.md"
ID = re.compile(r"(S21|TM|C)-\d+")
# The header that opens a matrix table. Every row of such a table, with or
# without its leading pipe (both render as a row), must carry an id: a
# backticked, bold, en-dashed or lowercase id is a typo to report, never a
# row to skip with its tests unchecked.
HEADER = re.compile(r"\|?\s*id\s*\|\s*guards\s*\|\s*step\s*\|\s*tests\s*\|\s*limit\s*\|?")
SEPARATOR = re.compile(r"\|?[\s:|-]+\|?")
ITEM = re.compile(r"`?(?P<path>[^`:\s][^`:]*)::(?P<fn>[A-Za-z_][A-Za-z0-9_]*)`?")
# "owed", "owed: <text>" and "owed (<text>)": an owed marker never counts
# as a test, so its wording is free after the word.
OWED = re.compile(r"owed\b.*|—")


def defines(path: str, fn: str) -> bool:
    pattern = re.compile(rf"\bfn {re.escape(fn)}\b")
    with open(path, encoding="utf-8", errors="replace") as handle:
        return any(pattern.search(line) and not line.lstrip().startswith("//") for line in handle)


def check(root: str, strict: bool) -> tuple[list[str], list[str], int]:
    """(problems, empty rows, tests named)."""
    matrix = os.path.join(root, MATRIX)
    if not os.path.isfile(matrix):
        return [f"{MATRIX}: missing; the security matrix the Stage 18 gate reads is not there"], [], 0
    real_root = os.path.realpath(root)
    problems: list[str] = []
    empty: list[str] = []
    seen: dict[str, int] = {}
    named = 0
    with open(matrix, encoding="utf-8") as handle:
        lines = handle.read().splitlines()
    in_table = False
    for number, line in enumerate(lines, 1):
        text = line.strip()
        if HEADER.fullmatch(text):
            in_table = True
            continue
        if not in_table:
            continue
        if "|" not in text:
            in_table = False  # a blank or prose line ends the table
            continue
        if SEPARATOR.fullmatch(text):
            continue
        cells = [c.strip() for c in text.strip("|").split("|")]
        first = cells[0]
        where = f"{MATRIX}:{number}"
        if not ID.fullmatch(first):
            problems.append(f"{where}: id {first!r} is not S21-NN, TM-NN or C-NN")
            continue
        if first in seen:
            problems.append(f"{where}: id {first} repeats line {seen[first]}")
        else:
            seen[first] = number
        if len(cells) != 5:
            problems.append(f"{where}: {first} has {len(cells)} cells, not the five | id | guards | step | tests | limit |")
            continue
        tests = 0
        for raw in cells[3].split("; "):
            item = raw.strip()
            if OWED.fullmatch(item):
                continue
            match = ITEM.fullmatch(item)
            if not match:
                problems.append(f"{where}: {first}: {item!r} is neither `path::fn` nor an owed marker")
                continue
            tests += 1
            path, fn = match.group("path"), match.group("fn")
            full = os.path.realpath(os.path.join(root, path))
            if os.path.isabs(path) or not full.startswith(real_root + os.sep):
                problems.append(f"{where}: {first}: {path} is not a path inside the repository")
            elif not os.path.isfile(full):
                problems.append(f"{where}: {first}: {path} does not exist (named for {fn})")
            elif not defines(full, fn):
                problems.append(f"{where}: {first}: {path} defines no fn {fn} outside a comment")
        named += tests
        if tests == 0:
            empty.append(f"{where}: {first} names no test")
    if not seen:
        problems.append(f"{MATRIX}: no row with an S21-NN, TM-NN or C-NN id; the gate would read nothing")
    if strict:
        problems += [f"{e} (--strict: an empty row fails the close)" for e in empty]
    return problems, empty, named


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Does every test tests/SECURITY-MATRIX.md names still exist?")
    parser.add_argument("--root", default=os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
    parser.add_argument("--strict", action="store_true", help="fail on a row that names no test (Stage 18's close)")
    args = parser.parse_args(argv)
    problems, empty, named = check(args.root, args.strict)
    for line in problems:
        print(line)
    if not args.strict:
        for line in empty:
            print(f"{line} (reported; --strict fails it)")
    if problems:
        print(f"check_security_matrix: {len(problems)} problem(s)", file=sys.stderr)
        return 1
    print(f"check_security_matrix: OK, {named} named test(s) exist; {len(empty)} empty row(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
