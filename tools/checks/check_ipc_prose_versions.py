#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/check_ipc_prose_versions.py
#
# >>> help
# Does the prose of architecture/contracts/LOCAL-IPC.md name the IPC minor
# and the IPC schema versions the code and the schema files hold?
#
#   tools/checks/check_ipc_prose_versions.py
#   tools/checks/check_ipc_prose_versions.py --root <dir>
#
# crates/api/ipc-protocol/tests/schema_agreement.rs reads the contract's
# two catalogue tables and nothing of its prose, so a minor bump could
# leave the prose a version behind with every test green (#503: three
# sites at once, each found by a reader). Two inputs:
#   MAX      the integer N in `pub const IPC_MAX_MINOR: u64 = N;` in
#            crates/api/ipc-protocol/src/version.rs
#   V(name)  `x-contract.version` of
#            architecture/contracts/schemas/ipc/<name>.schema.json
# Five rules, each a regular expression over the whole file:
#   1. highest-minor sentence: /the server's highest minor, (\d+) since /
#      matches exactly once and its number is MAX; the /(\d+) since /
#      chain from there to the first "never a literal" after it runs
#      MAX, MAX-1, ... 3, contiguous and descending.
#   2. version history: the set of N in /brought 2\.(\d+)/ over the
#      whole file is exactly {2..MAX}.
#   3. method parenthetical: every
#      /`([a-z-]+)` \((\d+\.\d+\.\d+) since [^()]*?which 2\.(\d+) brings/
#      has version == V(name) and minor <= MAX. Only the first version
#      after "(" is the claim; a later clause of the parenthetical is
#      history and is not matched.
#   4. no schema version above its file: every
#      /`ipc\/([a-z-]+)` (\d+\.\d+\.\d+)/ names a schema file that
#      exists, at a version <= V(name) in semver order. History names
#      older versions; none may name a newer one. Status ("approved") is
#      not part of the rule.
#   5. no minor above the code: no /(?<![\d.])2\.(\d{1,2})(?![\d.])/
#      exceeds MAX.
# A failure names the rule and the line. An input that cannot be read
# (no constant, no schema, no version) is a failure, never a skip.
# Limit: these are the prose's current shapes; a sentence reworded out
# of a rule's grammar is no longer checked by it (rules 1 and 2 fail on
# a vanished sentence, rules 3 and 4 match nothing and pass).
#
# Exit codes: 0 every rule holds; 1 a problem, each on its own line;
# 2 usage.
# <<< help
from __future__ import annotations

import argparse
import json
import os
import re
import sys

CONTRACT = "architecture/contracts/LOCAL-IPC.md"
VERSION_RS = "crates/api/ipc-protocol/src/version.rs"
SCHEMAS = "architecture/contracts/schemas/ipc"

MAX_RE = re.compile(r"pub const IPC_MAX_MINOR: u64 = (\d+);")
HIGHEST = re.compile(r"the server's highest minor, (\d+) since ")
CHAIN = re.compile(r"(\d+) since ")
CHAIN_END = "never a literal"
BROUGHT = re.compile(r"brought 2\.(\d+)")
PAREN = re.compile(r"`([a-z-]+)` \((\d+\.\d+\.\d+) since [^()]*?which 2\.(\d+) brings")
NAMED = re.compile(r"`ipc/([a-z-]+)` (\d+\.\d+\.\d+)")
MINOR = re.compile(r"(?<![\d.])2\.(\d{1,2})(?![\d.])")
# The chain's floor: minor 3 is the first the sentence lists a reason for.
FLOOR = 3


def semver(text: str) -> tuple[int, ...]:
    return tuple(int(part) for part in text.split("."))


def check(root: str) -> list[str]:
    problems: list[str] = []

    def read(rel: str) -> str | None:
        try:
            with open(os.path.join(root, rel), encoding="utf-8") as handle:
                return handle.read()
        except OSError as err:
            problems.append(f"{rel}: cannot read: {err.strerror}")
            return None

    rs = read(VERSION_RS)
    text = read(CONTRACT)
    if rs is None or text is None:
        return problems
    found = MAX_RE.findall(rs)
    if len(found) != 1:
        return [f"{VERSION_RS}: `pub const IPC_MAX_MINOR: u64 = N;` found {len(found)} times, want 1"]
    top = int(found[0])

    versions: dict[str, str | None] = {}

    def schema_version(name: str) -> str | None:
        if name not in versions:
            rel = f"{SCHEMAS}/{name}.schema.json"
            path = os.path.join(root, rel)
            version = None
            if os.path.isfile(path):
                try:
                    with open(path, encoding="utf-8") as handle:
                        version = (json.load(handle).get("x-contract") or {}).get("version")
                except (OSError, ValueError):
                    version = None
                if not isinstance(version, str) or not re.fullmatch(r"\d+\.\d+\.\d+", version):
                    problems.append(f"{rel}: no x-contract.version of the form N.N.N")
                    version = None
            versions[name] = version
        return versions[name]

    def line(offset: int) -> int:
        return text.count("\n", 0, offset) + 1

    def at(offset: int, rule: int, what: str) -> None:
        problems.append(f"{CONTRACT}:{line(offset)}: rule {rule}: {what}")

    # Rule 1.
    highest = list(HIGHEST.finditer(text))
    if len(highest) != 1:
        problems.append(f"{CONTRACT}: rule 1: \"the server's highest minor, N since \" found "
                        f"{len(highest)} times, want 1")
    else:
        start = highest[0].start(1)
        end = text.find(CHAIN_END, start)
        if end < 0:
            at(start, 1, f"no \"{CHAIN_END}\" ends the highest-minor sentence")
        else:
            chain = list(CHAIN.finditer(text, start, end))
            want = list(range(top, FLOOR - 1, -1))
            got = [int(m.group(1)) for m in chain]
            if int(highest[0].group(1)) != top:
                at(start, 1, f"the highest minor reads {highest[0].group(1)}, IPC_MAX_MINOR is {top}")
            elif got != want:
                for i, m in enumerate(chain):
                    if i >= len(want) or int(m.group(1)) != want[i]:
                        expected = want[i] if i < len(want) else "nothing"
                        at(m.start(1), 1, f"the minor chain reads {m.group(1)} where {expected} belongs "
                                          f"(want {top}..{FLOOR} contiguous and descending)")
                        break
                else:
                    at(chain[-1].start(1), 1, f"the minor chain stops at {got[-1]}, want it down to {FLOOR}")

    # Rule 2.
    brought = list(BROUGHT.finditer(text))
    if not brought:
        problems.append(f"{CONTRACT}: rule 2: no \"brought 2.N\" anywhere, want 2.2..2.{top}")
    else:
        seen = {int(m.group(1)) for m in brought}
        for m in brought:
            if not 2 <= int(m.group(1)) <= top:
                at(m.start(), 2, f"\"brought 2.{m.group(1)}\" is outside 2.2..2.{top}")
        for minor in sorted(set(range(2, top + 1)) - seen):
            at(brought[-1].start(), 2, f"no \"brought 2.{minor}\" in the version history (want 2.2..2.{top})")

    # Rule 3.
    for m in PAREN.finditer(text):
        name, claimed, minor = m.group(1), m.group(2), int(m.group(3))
        actual = schema_version(name)
        if actual is None:
            at(m.start(), 3, f"`{name}` names no readable {SCHEMAS}/{name}.schema.json")
        elif claimed != actual:
            at(m.start(2), 3, f"`{name}` ({claimed} since ...) but {name}.schema.json is {actual}")
        if minor > top:
            at(m.start(3), 3, f"`{name}`'s parenthetical names 2.{minor}, IPC_MAX_MINOR is {top}")

    # Rule 4.
    for m in NAMED.finditer(text):
        name, claimed = m.group(1), m.group(2)
        if not os.path.isfile(os.path.join(root, SCHEMAS, f"{name}.schema.json")):
            at(m.start(), 4, f"`ipc/{name}` names no schema file {SCHEMAS}/{name}.schema.json")
            continue
        actual = schema_version(name)
        if actual is not None and semver(claimed) > semver(actual):
            at(m.start(2), 4, f"`ipc/{name}` {claimed} is above {name}.schema.json's {actual}")

    # Rule 5.
    for m in MINOR.finditer(text):
        if int(m.group(1)) > top:
            at(m.start(), 5, f"2.{m.group(1)} is above IPC_MAX_MINOR {top}")

    return problems


def main(argv: list[str] | None = None) -> int:
    with open(__file__, encoding="utf-8") as handle:
        source = handle.read()
    block = source.split("# >>> help\n", 1)[1].split("# <<< help", 1)[0]
    parser = argparse.ArgumentParser(description="\n".join(line[2:] for line in block.splitlines()),
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", default=os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
    args = parser.parse_args(argv)
    if not os.path.isdir(args.root):
        print(f"check_ipc_prose_versions: not a directory: {args.root}", file=sys.stderr)
        return 2
    problems = check(args.root)
    for problem in problems:
        print(problem)
    if problems:
        print(f"\ncheck_ipc_prose_versions: {len(problems)} problem(s) in {CONTRACT}'s prose versions.",
              file=sys.stderr)
        return 1
    print(f"check_ipc_prose_versions: OK — {CONTRACT}'s prose agrees with IPC_MAX_MINOR and the ipc schemas.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
