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
#   - each path is a file inside the repository, and it defines fn as a
#     test: `\bfn <name>\b` on a line whose first non-blank characters are
#     not `//`, carrying `#[test]` or `#[tokio::test...]` among the
#     attributes above it (past doc comments and blank lines). The file is
#     read as code: comments (`//`, nested `/* */`) and string, raw-string
#     and character literals are blanked first, so a test disabled in a
#     block comment, a name in a string or comment, and a helper of the
#     same name vouch for nothing.
# A test compiled out by #[cfg(any())] or #[cfg(false)], on the fn or on
# an enclosing inline item (a `mod t { ... }`, its brace on any line), or
# by the inner #![cfg(...)] of its mod or file, does not vouch; an
# #[ignore]d one does (CI runs ignored tests). Limits: any other cfg that
# can never be true is not evaluated; an out-of-line `mod t;` is not
# followed; an inner #![cfg(...)] is read only on a line of its own and
# whole (not after `mod t {` on the same line, nor split across lines);
# and a `;` inside a compiled-out item's header (`fn f(x: [u8;`) ends it
# early. A test in any of those still vouches.
# A row with no `path::fn` at all is empty: reported always, and a
# failure only under --strict, which Stage 18's close runs. It is carried
# instead when every item in its tests cell is an owed marker naming the
# plan section that carries the row, `owed: <what> in Stage NN (plan
# section MM)`: reported always, labelled carried, and never a failure.
# Any other owed marker, `—` and a carried marker beside either leave the
# row empty. Limit: the shape is read, not the plan; whether section MM
# exists and holds Stage NN, or NN is a later stage, is not checked.
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
# An owed marker that names where the row's work is carried: the row is
# carried out of this stage, not empty (Row grammar, tests/SECURITY-MATRIX.md).
CARRIED = re.compile(r"owed: .+ in Stage (?P<stage>\d+) \(plan section (?P<section>\d+)\)")


TEST_ATTRIBUTE = re.compile(r"#\[\s*(test\s*\]|tokio::test\b)")
# A test compiled out never runs anywhere, so it proves nothing: cfg(any())
# and cfg(false) are the always-false cfgs. A general cfg that can never be
# true needs a cfg evaluator, which this is not (the help block above names the limit).
# #[ignore] still vouches: ignored tests run in CI with --include-ignored.
COMPILED_OUT = re.compile(r"#\[\s*cfg\s*\(\s*(?:any\s*\(\s*\)|false)\s*\)\s*\]")
# Each qualifier is a distinct word followed by required space, so the repetition cannot split one input two
# ways (an `extern\s+\S*\s*` alternative could, and backtracked exponentially: CodeQL on #268). An `extern "C" fn`
# is left out: it holds no test.
HOLDS_TESTS = re.compile(r"(?:pub(?:\([^)]*\))?\s+)?(?:(?:unsafe|async|const)\s+)*(?:mod|fn|impl|trait)\b")
INNER_COMPILED_OUT = re.compile(r"#!\[\s*cfg\s*\(\s*(?:any\s*\(\s*\)|false)\s*\)\s*\]")
CHAR = re.compile(r"'(?:\\.|[^\\'\n])'")
RAW_OPEN = re.compile(r'(?:b|c)?r(#*)"')


def code_only(source: str) -> list[str]:
    """The source's lines with every comment and literal blanked out, line
    breaks kept: `//` and `/* */` comments (nested, as Rust nests them),
    "..." strings, r"..." / r#"..."# raw strings and 'x' character literals.
    What remains is code, so a name inside a disabled /* test */, a string
    or a trailing comment is not there to be found."""
    out: list[str] = []
    i, n, depth = 0, len(source), 0
    while i < n:
        c = source[i]
        if depth:  # inside a block comment
            if source.startswith("/*", i):
                depth, i = depth + 1, i + 2
            elif source.startswith("*/", i):
                depth, i = depth - 1, i + 2
            else:
                out.append("\n" if c == "\n" else " ")
                i += 1
            continue
        if source.startswith("/*", i):
            depth, i = 1, i + 2
            continue
        if source.startswith("//", i):
            end = source.find("\n", i)
            i = n if end < 0 else end
            continue
        raw = RAW_OPEN.match(source, i) if c in "bcr" and (i == 0 or not (source[i - 1].isalnum() or source[i - 1] == "_")) else None
        if raw:
            close = '"' + raw.group(1)
            end = source.find(close, raw.end())
            end = n if end < 0 else end + len(close)
            out.append('""' + "\n" * source.count("\n", i, end))
            i = end
            continue
        if c == '"':
            j = i + 1
            while j < n and source[j] != '"':
                j += 2 if source[j] == "\\" else 1
            out.append('""' + "\n" * source.count("\n", i, j))
            i = j + 1
            continue
        if c == "'":
            literal = CHAR.match(source, i)
            if literal:
                out.append("' '")
                i = literal.end()
                continue
        out.append(c)
        i += 1
    return "".join(out).split("\n")


def test_fns(source: str, fn: str) -> bool:
    """Whether fn is defined as a test: in code (comments and literals
    blanked by code_only), an item `fn <fn>` with #[test] or
    #[tokio::test...] among the attributes above it. Attributes are read
    whole, a multi-line one included, by balancing brackets, past blank
    lines; an attribute and its fn may share a line. Any other code clears
    the attributes collected so far, so a helper of the same name, or a
    name in a string or comment, vouches for nothing. An item compiled out
    (COMPILED_OUT among its attributes) takes everything inside its braces
    with it: braces are counted in code, and nothing vouches until the one
    that closes it."""
    pattern = re.compile(rf"\bfn\s+{re.escape(fn)}\b")
    attributes: list[str] = []
    depth = 0
    braces = 0
    dead_at: int | None = None  # the brace level a compiled-out item's body closes back to
    pending = False  # a compiled-out item whose body has not opened yet
    for line in code_only(source):
        rest = line.strip()
        while rest:
            if depth:  # inside a multi-line attribute
                close = 0
                for k, ch in enumerate(rest):
                    depth += {"[": 1, "]": -1}.get(ch, 0)
                    if depth == 0:
                        close = k + 1
                        break
                attributes[-1] += " " + (rest[:close] if close else rest)
                rest = rest[close:].strip() if close else ""
                continue
            if rest.startswith("#!["):
                # An inner attribute compiles out the item it sits in (the whole file at the top level), back
                # to the brace that closes it: one level up from here, or never for a file.
                if INNER_COMPILED_OUT.match(rest) and dead_at is None:
                    dead_at = braces - 1
                rest = ""
                continue
            if rest.startswith("#["):
                depth, end = 0, len(rest)
                for k, ch in enumerate(rest[1:], 1):  # from the opening [
                    depth += {"[": 1, "]": -1}.get(ch, 0)
                    if depth == 0:
                        end = k + 1
                        break
                attributes.append(rest[:end])
                rest = rest[end:].strip()
                continue
            own = any(COMPILED_OUT.match(a) for a in attributes)
            compiled_out = pending or own
            if (dead_at is None and not compiled_out and pattern.search(rest)
                    and any(TEST_ATTRIBUTE.match(a) for a in attributes)):
                return True
            # A compiled-out item's body may open on a later line (a where clause, a brace on its own line):
            # an item that can hold a test (mod, fn, impl, trait) stays pending until its first `{`, or a `;`
            # that ends it. Only the item's own attributes start it, so a compiled-out field or variant (ending
            # in `,`) never reaches the next live item.
            was_pending, pending = pending, False
            if compiled_out and dead_at is None:
                if "{" in rest:
                    dead_at = braces
                elif ";" not in rest and (was_pending or (own and HOLDS_TESTS.match(rest))):
                    pending = True
            braces += rest.count("{") - rest.count("}")
            if dead_at is not None and braces <= dead_at:
                dead_at = None
            attributes = []
            rest = ""
    return False


def defines(path: str, fn: str) -> bool:
    with open(path, encoding="utf-8", errors="replace") as handle:
        return test_fns(handle.read(), fn)


def check(root: str, strict: bool) -> tuple[list[str], list[str], list[str], int]:
    """(problems, empty rows, carried rows, tests named)."""
    matrix = os.path.join(root, MATRIX)
    if not os.path.isfile(matrix):
        return [f"{MATRIX}: missing; the security matrix the Stage 18 gate reads is not there"], [], [], 0
    real_root = os.path.realpath(root)
    problems: list[str] = []
    empty: list[str] = []
    carried: list[str] = []
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
        carries = [CARRIED.fullmatch(raw.strip()) for raw in cells[3].split("; ")]
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
                problems.append(f"{where}: {first}: {path} defines no test fn {fn} (#[test] or #[tokio::test])")
        named += tests
        if tests == 0 and all(carries):
            # Every place the row's work goes is named, once each, in the cell's order.
            to = dict.fromkeys(f"Stage {c['stage']} (plan section {c['section']})" for c in carries)
            carried.append(f"{where}: {first} names no test, carried to {', '.join(to)}")
        elif tests == 0:
            empty.append(f"{where}: {first} names no test")
    if not seen:
        problems.append(f"{MATRIX}: no row with an S21-NN, TM-NN or C-NN id; the gate would read nothing")
    if strict:
        problems += [f"{e} (--strict: an empty row fails the close)" for e in empty]
    return problems, empty, carried, named


def main(argv: list[str] | None = None) -> int:
    # --help prints the rules block at the top of this file, where what vouches and the limits are written down.
    with open(__file__, encoding="utf-8") as handle:
        block = handle.read().split("# >>> help\n", 1)[1].split("# <<< help", 1)[0]
    parser = argparse.ArgumentParser(description="\n".join(line[2:] for line in block.splitlines()),
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", default=os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
    parser.add_argument("--strict", action="store_true", help="fail on a row that names no test (Stage 18's close)")
    args = parser.parse_args(argv)
    problems, empty, carried, named = check(args.root, args.strict)
    for line in problems:
        print(line)
    for line in carried:
        print(f"{line} (carried; --strict passes it)")
    if not args.strict:
        for line in empty:
            print(f"{line} (reported; --strict fails it)")
    if problems:
        print(f"check_security_matrix: {len(problems)} problem(s)", file=sys.stderr)
        return 1
    print(f"check_security_matrix: OK, {named} named test(s) exist; {len(empty)} empty row(s), {len(carried)} carried")
    return 0


if __name__ == "__main__":
    sys.exit(main())
