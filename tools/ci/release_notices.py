#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_notices.py
#
# >>> help
# Write the third-party notices a release archive ships beside its
# programs: every crate compiled into them, with its licence and the
# licence texts the crate itself publishes.
#
#   tools/ci/release_notices.py <metadata.json> <compiled.txt> <out.md> <package>...
#
# <compiled.txt> says WHICH crates: `cargo tree --locked -e normal
# --target <target> -p <package>... --prefix none -f '{p}'`, the crates
# the shipped build compiles for those packages, one `name vVERSION` per
# line. <metadata.json> (`cargo metadata --locked --format-version 1
# --filter-platform <target>`) says only WHAT each is -- its licence and
# its files -- because its resolve unifies features across every
# workspace member, the Android app's included, and so reaches crates
# these programs do not contain (Skia's bindings among them). Normal
# dependencies: a build-dependency runs at build time and is not in the
# program, and a dev-dependency is not built at all; a proc macro is a
# normal dependency and is listed, since its expansion is compiled in.
# Workspace members are this repository's own and are covered by its
# LICENSE; a vendored crate under third_party/ is NOT a member and is
# listed like any other.
#
# The texts are each crate's own files whose names begin LICENSE, LICENCE,
# COPYING, NOTICE, COPYRIGHT or UNLICENSE (and the files of a directory so
# named, as a LICENSES/ directory holds them), or its `license-file`; a text
# several crates share is printed once and named by each. A crate that
# publishes no licence file is listed with its licence expression and
# marked so, never given a text it does not carry.
#
# REFUSED, writing nothing and removing an earlier run's <out.md>, when a named package is not a workspace member,
# a compiled crate is not one the metadata's normal walk from the
# packages reaches (the two disagree about the build), or a crate carries
# no licence expression and no licence file.
#
# Exit codes:
#   0  written; the counts are printed
#   1  a refusal above
#   2  usage
# <<< help

import hashlib
import json
import os
import sys
from pathlib import Path

PREFIXES = ("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT", "UNLICENSE")


def help_text() -> str:
    lines = Path(__file__).read_text().splitlines()
    start = lines.index("# >>> help") + 1
    end = lines.index("# <<< help")
    return "\n".join(line[2:] for line in lines[start:end]) + "\n"


def compiled_set(text: str) -> set:
    """The (name, version) pairs of `cargo tree --prefix none -f '{p}'`."""
    pairs = set()
    for line in text.splitlines():
        words = line.split()
        if not words:
            continue
        if len(words) < 2 or not words[1].startswith("v"):
            raise ValueError(f"not a cargo tree line: {line!r}")
        pairs.add((words[0], words[1][1:]))
    return pairs


def shipped(metadata: dict, roots: list, compiled: set) -> list:
    """The third-party packages the build compiles, sorted."""
    packages = {p["id"]: p for p in metadata["packages"]}
    members = set(metadata["workspace_members"])
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    by_name = {packages[i]["name"]: i for i in members}
    missing = [r for r in roots if r not in by_name]
    if missing:
        raise ValueError("not a workspace member: " + ", ".join(missing))
    seen, stack = set(), [by_name[r] for r in roots]
    while stack:
        node = stack.pop()
        if node in seen:
            continue
        seen.add(node)
        for dep in nodes[node]["deps"]:
            if any(kind["kind"] is None for kind in dep["dep_kinds"]):
                stack.append(dep["pkg"])
    walked = {(packages[i]["name"], packages[i]["version"]): i for i in seen}
    stray = sorted(compiled - set(walked))
    if stray:
        raise ValueError(
            "compiled but not reached from the packages in the metadata: "
            + ", ".join(f"{n} {v}" for n, v in stray)
        )
    third = [packages[walked[pair]] for pair in compiled if walked[pair] not in members]
    return sorted(third, key=lambda p: (p["name"], p["version"]))


def licence_files(package: dict) -> list:
    root = Path(package["manifest_path"]).parent
    files = set()
    if package.get("license_file"):
        files.add(root / package["license_file"])
    for entry in os.listdir(root):
        if not entry.upper().startswith(PREFIXES):
            continue
        path = root / entry
        if path.is_file():
            files.add(path)
        elif path.is_dir():
            # A REUSE-style LICENSES/ directory, one text per file.
            files.update(f for f in path.iterdir() if f.is_file())
    return sorted(files)


def render(third: list) -> tuple:
    texts, order, lines, bare = {}, [], [], 0
    for package in third:
        expr = package.get("license") or ""
        files = licence_files(package)
        if not expr and not files:
            raise ValueError(f"{package['name']} {package['version']} has no licence expression and no licence file")
        refs = []
        for path in files:
            text = path.read_text(errors="replace").strip() + "\n"
            digest = hashlib.sha256(text.encode()).hexdigest()
            if digest not in texts:
                texts[digest] = {"text": text, "label": f"T{len(texts) + 1}", "users": []}
                order.append(digest)
            texts[digest]["users"].append(f"{package['name']} {package['version']} ({path.name})")
            refs.append(texts[digest]["label"])
        line = f"- `{package['name']}` {package['version']}: {expr or 'see its licence file'}"
        if refs:
            line += " — text " + ", ".join(refs)
        else:
            bare += 1
            line += " — publishes no licence file"
            if package.get("repository"):
                line += f"; source {package['repository']}"
        lines.append(line)
    out = [
        "# Third-party notices",
        "",
        "These programs are InterWeave's own, under the Apache License 2.0",
        "(`LICENSE`), and they contain the following crates, each under its",
        "own licence. A licence text that several crates share is printed",
        "once, below the list, and named by each.",
        "",
        *lines,
        "",
        "## Licence texts",
    ]
    for digest in order:
        entry = texts[digest]
        out += ["", f"### {entry['label']}", "", "Used by " + "; ".join(entry["users"]) + ".", "",
                "```text", entry["text"].rstrip("\n"), "```"]
    return "\n".join(out) + "\n", len(order), bare


def run(argv: list) -> int:
    if len(argv) == 2 and argv[1] in ("-h", "--help"):
        sys.stdout.write(help_text())
        return 0
    if len(argv) < 5:
        sys.stderr.write("usage: release_notices.py <metadata.json> <compiled.txt> <out.md> <package>...\n")
        return 2
    try:
        metadata = json.loads(Path(argv[1]).read_text())
        compiled = compiled_set(Path(argv[2]).read_text())
        third = shipped(metadata, argv[4:], compiled)
        page, distinct, bare = render(third)
    except (OSError, ValueError, KeyError) as err:
        sys.stderr.write(f"release_notices: {err}\n")
        return 1
    publish(Path(argv[3]), page)
    print(f"{len(third)} crates, {distinct} distinct licence texts, {bare} crates with no licence file")
    return 0

def publish(path: Path, text: str) -> None:
    """Writes `text` at `path` whole or not at all: a temporary file beside
    it, renamed over it once written, so a failed write leaves no part of a
    page there."""
    tmp = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    try:
        tmp.write_text(text)
        os.replace(tmp, path)
    finally:
        tmp.unlink(missing_ok=True)


def main(argv: list) -> int:
    # A run that fails once its output is named -- a refusal, a failed
    # write, an exception -- leaves no file of that name, an earlier run's
    # included, so what is there after a failure is never taken for this
    # run's.
    out = Path(argv[3]) if len(argv) >= 5 else None
    status = 1
    try:
        status = run(argv)
    finally:
        if status != 0 and out is not None:
            out.unlink(missing_ok=True)
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv))
