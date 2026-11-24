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
# sites at once, each found by a reader). Three inputs:
#   MAJOR    the integer N in `pub const IPC_MAJOR: u64 = N;` and
#   MAX      the integer N in `pub const IPC_MAX_MINOR: u64 = N;`, both
#            in crates/api/ipc-protocol/src/version.rs
#   V(name)  `x-contract.version` of
#            architecture/contracts/schemas/ipc/<name>.schema.json
# Major 2 is frozen history (ADR-0067 decision 7): it ended at minor 13,
# so a 2.N anywhere is bounded by 13 and its chain and history are fixed,
# whatever MAX is. A minor of the major the code speaks is written
# MAJOR.N and is what MAX bounds. A major the code moves past joins
# FROZEN below, with its last minor, in the change that moves it.
# Five rules, each a regular expression over the whole file:
#   1. highest-minor sentence:
#      /the server's highest minor on major (\d+), (\d+) since /
#      matches exactly once, naming MAJOR and MAX; the /(\d+) since /
#      chain from there to the frozen clause runs MAX, MAX-1, ... 0,
#      contiguous and descending. The frozen clause,
#      /major 2, [^;]*?ended at minor (\d+) — /, follows it once, names
#      13, and the chain after it to the first "never a literal" runs
#      13, 12, ... 3.
#   2. version history: the set of N in /brought 2\.(\d+)/ over the
#      whole file is exactly {2..13}, and the set of N in
#      /brought MAJOR\.(\d+)/ is exactly {1..MAX} (none while MAX is
#      0); a "brought" of any other major fails.
#   3. method parenthetical: every
#      /`([a-z-]+)` \((\d+\.\d+\.\d+) since [^()]*?which (\d+)\.(\d+) brings/
#      has version == V(name) and a minor within its major's bound (13
#      for 2, MAX for MAJOR, none for another). Only the first version
#      after "(" is the claim; a later clause of the parenthetical is
#      history and is not matched.
#   4. no schema version above its file: every
#      /`ipc\/([a-z-]+)` (\d+\.\d+\.\d+)/ names a schema file that
#      exists, at a version <= V(name) in semver order. History names
#      older versions; none may name a newer one. Status ("approved") is
#      not part of the rule.
#   5. no minor above its major's end: no 2.N exceeds 13 and no
#      MAJOR.N exceeds MAX, each read by /(?<![\d.])M\.(\d{1,2})(?!\d|\.\d)/.
#      A full stop after the minor ends a sentence and is read ("2.13."
#      is 2.13); a digit after it makes a dotted version. A MAJOR.N
#      followed by " or later" is a rule about minors to come ("a
#      capability introduced at 3.1 or later"), not a claim one exists,
#      and is not read.
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

MAJOR_RE = re.compile(r"pub const IPC_MAJOR: u64 = (\d+);")
MAX_RE = re.compile(r"pub const IPC_MAX_MINOR: u64 = (\d+);")
HIGHEST = re.compile(r"the server's highest minor on major (\d+), (\d+) since ")
CHAIN = re.compile(r"(\d+) since ")
CHAIN_END = "never a literal"
BROUGHT = re.compile(r"brought (\d+)\.(\d+)")
PAREN = re.compile(r"`([a-z-]+)` \((\d+\.\d+\.\d+) since [^()]*?which (\d+)\.(\d+) brings")
NAMED = re.compile(r"`ipc/([a-z-]+)` (\d+\.\d+\.\d+)")
# A major's bare minors, the major filled in; " or later" after one is a
# rule about minors to come, not a claim that one exists.
MINOR_OF = r"(?<![\d.]){major}\.(\d{{1,2}})(?!\d|\.\d)(?! or later)"

# Major 2 is frozen: it ended at minor 13 when the Stage 22 release opened
# major 3 (ADR-0067 decision 7). Its chain lists reasons from minor 3 up,
# and its history list "brought" from 2.2 up. The live major's chain runs
# down to 0, the release that opened it, and its history from MAJOR.1.
# Only one frozen major is modelled: a code major other than FROZEN + 1
# fails rule 1 until the major it left is frozen here too.
FROZEN = 2
FROZEN_LAST = 13
FROZEN_FLOOR = 3
FROZEN_BROUGHT_FROM = 2
FROZEN_CLAUSE = re.compile(r"major 2, [^;]*?ended at minor (\d+) — ")
FLOOR = 0


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
    consts = {}
    for name, pattern in (("IPC_MAJOR", MAJOR_RE), ("IPC_MAX_MINOR", MAX_RE)):
        found = pattern.findall(rs)
        if len(found) != 1:
            problems.append(f"{VERSION_RS}: `pub const {name}: u64 = N;` found {len(found)} times, want 1")
        else:
            consts[name] = int(found[0])
    if problems:
        return problems
    major, top = consts["IPC_MAJOR"], consts["IPC_MAX_MINOR"]
    if major != FROZEN + 1:
        # Every rule below reads major FROZEN as history and `major` as live;
        # on another major they would check the wrong bounds.
        return [f"{VERSION_RS}: rule 1: IPC_MAJOR is {major}, and the check models major {FROZEN} "
                f"frozen under major {FROZEN + 1}; freeze the major the code left in it"]
    # Each major's bound on its minors; a major outside it has none.
    bound = {FROZEN: FROZEN_LAST, major: top}

    # name -> (version, None) or (None, why the version cannot be read).
    # The caller reports the why at its own rule and line, so an unreadable
    # schema fails the rule that needed it rather than passing it.
    versions: dict[str, tuple[str | None, str | None]] = {}

    def schema_version(name: str) -> tuple[str | None, str | None]:
        if name not in versions:
            rel = f"{SCHEMAS}/{name}.schema.json"
            try:
                with open(os.path.join(root, rel), encoding="utf-8") as handle:
                    doc = json.load(handle)
            except OSError as err:
                versions[name] = (None, f"{rel} cannot be read: {err.strerror}")
            except ValueError:
                versions[name] = (None, f"{rel} is not JSON")
            else:
                contract = doc.get("x-contract") if isinstance(doc, dict) else None
                version = contract.get("version") if isinstance(contract, dict) else None
                if isinstance(version, str) and re.fullmatch(r"\d+\.\d+\.\d+", version):
                    versions[name] = (version, None)
                else:
                    versions[name] = (None, f"{rel} has no x-contract.version of the form N.N.N")
        return versions[name]

    def line(offset: int) -> int:
        return text.count("\n", 0, offset) + 1

    def at(offset: int, rule: int, what: str) -> None:
        problems.append(f"{CONTRACT}:{line(offset)}: rule {rule}: {what}")

    def chain_runs(start: int, end: int, high: int, floor: int, what: str) -> None:
        # The /(\d+) since / chain in text[start:end] reads high..floor.
        chain = list(CHAIN.finditer(text, start, end))
        want = list(range(high, floor - 1, -1))
        got = [int(m.group(1)) for m in chain]
        if got == want:
            return
        for i, m in enumerate(chain):
            if i >= len(want) or got[i] != want[i]:
                expected = want[i] if i < len(want) else "nothing"
                at(m.start(1), 1, f"{what} reads {m.group(1)} where {expected} belongs "
                                  f"(want {high}..{floor} contiguous and descending)")
                return
        at(chain[-1].start(1) if chain else start, 1,
           f"{what} stops at {got[-1] if got else 'nothing'}, want it down to {floor}")

    # Rule 1.
    highest = list(HIGHEST.finditer(text))
    frozen = list(FROZEN_CLAUSE.finditer(text))
    if len(highest) != 1:
        problems.append(f"{CONTRACT}: rule 1: \"the server's highest minor on major N, N since \" found "
                        f"{len(highest)} times, want 1")
    elif len(frozen) != 1:
        problems.append(f"{CONTRACT}: rule 1: \"major {FROZEN}, ... ended at minor N — \" found "
                        f"{len(frozen)} times, want 1")
    else:
        h, f = highest[0], frozen[0]
        end = text.find(CHAIN_END, f.end())
        if int(h.group(1)) != major:
            at(h.start(1), 1, f"the highest-minor sentence names major {h.group(1)}, IPC_MAJOR is {major}")
        elif int(h.group(2)) != top:
            at(h.start(2), 1, f"the highest minor reads {h.group(2)}, IPC_MAX_MINOR is {top}")
        elif f.start() < h.end():
            at(f.start(), 1, f"the major-{FROZEN} clause comes before the highest-minor sentence")
        elif int(f.group(1)) != FROZEN_LAST:
            at(f.start(1), 1, f"major {FROZEN} ended at minor {FROZEN_LAST}, the clause reads {f.group(1)}")
        elif end < 0:
            at(f.start(), 1, f"no \"{CHAIN_END}\" ends the highest-minor sentence")
        else:
            chain_runs(h.start(2), f.start(), top, FLOOR, f"the major-{major} chain")
            chain_runs(f.end(), end, FROZEN_LAST, FROZEN_FLOOR, f"the major-{FROZEN} chain")

    # Rule 2.
    brought = list(BROUGHT.finditer(text))
    want_brought = {FROZEN: set(range(FROZEN_BROUGHT_FROM, FROZEN_LAST + 1)), major: set(range(1, top + 1))}
    seen: dict[int, set[int]] = {m: set() for m in want_brought}
    for m in brought:
        of, minor = int(m.group(1)), int(m.group(2))
        if of not in want_brought:
            at(m.start(), 2, f"\"brought {of}.{minor}\" names major {of}, neither {FROZEN} nor IPC_MAJOR {major}")
        elif minor not in want_brought[of]:
            at(m.start(), 2, f"\"brought {of}.{minor}\" is outside major {of}'s history")
        else:
            seen[of].add(minor)
    for of, want in want_brought.items():
        for minor in sorted(want - seen[of]):
            if brought:
                at(brought[-1].start(), 2, f"no \"brought {of}.{minor}\" in the version history")
            else:
                problems.append(f"{CONTRACT}: rule 2: no \"brought {of}.{minor}\" anywhere")

    # Rule 3.
    for m in PAREN.finditer(text):
        name, claimed, of, minor = m.group(1), m.group(2), int(m.group(3)), int(m.group(4))
        actual, why = schema_version(name)
        if actual is None:
            at(m.start(), 3, f"`{name}`: {why}")
        elif claimed != actual:
            at(m.start(2), 3, f"`{name}` ({claimed} since ...) but {name}.schema.json is {actual}")
        if of not in bound:
            at(m.start(3), 3, f"`{name}`'s parenthetical names {of}.{minor}, of neither major {FROZEN} "
                              f"nor IPC_MAJOR {major}")
        elif minor > bound[of]:
            at(m.start(3), 3, f"`{name}`'s parenthetical names {of}.{minor}, above major {of}'s {bound[of]}")

    # Rule 4.
    for m in NAMED.finditer(text):
        name, claimed = m.group(1), m.group(2)
        if not os.path.isfile(os.path.join(root, SCHEMAS, f"{name}.schema.json")):
            at(m.start(), 4, f"`ipc/{name}` names no schema file {SCHEMAS}/{name}.schema.json")
            continue
        actual, why = schema_version(name)
        if actual is None:
            at(m.start(), 4, f"`ipc/{name}`: {why}")
        elif semver(claimed) > semver(actual):
            at(m.start(2), 4, f"`ipc/{name}` {claimed} is above {name}.schema.json's {actual}")

    # Rule 5.
    for of, last in sorted(bound.items()):
        for m in re.finditer(MINOR_OF.format(major=of), text):
            if int(m.group(1)) > last:
                at(m.start(), 5, f"{of}.{m.group(1)} is above major {of}'s highest minor {last}")

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
    print(f"check_ipc_prose_versions: OK — {CONTRACT}'s prose agrees with IPC_MAJOR, IPC_MAX_MINOR and the ipc schemas.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
