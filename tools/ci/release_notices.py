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
#   tools/ci/release_notices.py [--json KEY] <metadata.json> <compiled.txt> <out> <package>...
#
# With --json KEY, <out> is {"KEY": the notices} for release_page.py: the
# page's own title and opening paragraph dropped (the release page
# introduces the list in its own words) and every heading one level down, so the
# text sits under a section of the release page (ADR-0058 decision 5, the
# APK's notices); a line inside a fenced licence text is never touched.
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
# several crates share is printed once and named by each.
#
# A crate that publishes none has its notice DERIVED (ADR-0058 decision 2,
# A 2026-10-22), as tools/ci/notice_derived.json lists it by name and
# version (NOTICE_DERIVED_JSON names another list):
#   header       every distinct leading comment block with a copyright
#                line among the files compiled in (the root module's
#                first, then src/ and build.rs in sorted order; tests/,
#                examples/ and benches/ are not read), shipped whole --
#                among them a full permission notice of its licence;
#   header+text  those headers, which name its licence without its text,
#                followed by the standard text of the licence's first term;
# and, beside either, the standard text of every licence a header names
# that the notice does not otherwise carry, as the entry's extra_texts
# list it (a header naming BSD with no BSD text attached is refused).
# Each header-case entry also names the root header's file and the count
# of distinct headers, so a header a version adds is looked at again.
#   standard     no copyright header anywhere: the standard text of the
#                first term, under the crate's `authors` (or "the <crate>
#                authors"), after one sentence saying so.
# The standard texts are tools/ci/licence_texts/, SPDX's, each pinned.
#
# REFUSED, writing nothing and removing an earlier run's <out.md>, when a
# named package is not a workspace member, the compiled list does not name
# every named package (an empty list, or one taken for other programs,
# would otherwise give a page with no crates), a compiled crate is not one the
# metadata's normal walk from the packages reaches (the two disagree about
# the build), or a crate publishes no licence file and the list does not
# name it (an MIT or BSD crate's copyright notice must travel with every
# copy, so a crate is never listed by its licence expression alone), or a
# listed crate disagrees with its entry: a different licence expression, a
# header case whose header is missing, in another file or counted
# otherwise, a licence a header names with no text for it, a standard case
# whose crate has a copyright header after all, a licence file it now
# publishes (remove the entry), or a standard text that is not its pin.
#
# Exit codes:
#   0  written; the counts are printed
#   1  a refusal above
#   2  usage
# <<< help

import hashlib
import json
import os
import re
import sys
from pathlib import Path

PREFIXES = ("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT", "UNLICENSE")

# ADR-0058 decision 3: the reviewed list of crates whose notice is
# derived, keyed by name and version so a bump is looked at again.
# NOTICE_DERIVED_JSON names another list (the self-test's).
DERIVED = Path(__file__).with_name("notice_derived.json")
CASES = ("header", "header+text", "standard")

# The standard texts of decision 2.iii, SPDX license-list-data v3.27.0
# verbatim, each pinned: a changed file is refused, not shipped. The
# second field is the copyright placeholder the text carries, filled
# with the crate's authors; a text without one gets a copyright line
# above it.
TEXTS = Path(__file__).with_name("licence_texts")
STANDARD = {
    "Apache-2.0": ("074e6e32c86a4c0ef8b3ed25b721ca23aca83df277cd88106ef7177c354615ff", None),
    "BSD-3-Clause": ("5a93d5831e1297ab10fe643e1a631e83be392896da14ee2951285a79012df69d", "<year> <owner>"),
    "CC0-1.0": ("a2010f343487d3f7618affe54f789f5487602331c0a8d03f49e9a7c547cf0499", None),
    "MIT": ("b05785f9f18e6716bab63424b11454513b9943a222595b70411009202fc592b5", "<year> <copyright holders>"),
}


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
    # cargo tree prints each root's own line, so a list without one was not
    # made for these programs; the walk below only bounds the list from
    # above, and an empty list would pass it with nothing to ship.
    absent = [
        r for r in roots
        if (packages[by_name[r]]["name"], packages[by_name[r]]["version"]) not in compiled
    ]
    if absent:
        raise ValueError("the compiled list does not name: " + ", ".join(absent))
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


def load_derived(path: Path) -> dict:
    """notice_derived.json as {(name, version): entry}, each entry checked
    for its shape."""
    entries = json.loads(path.read_text())["crates"]
    derived = {}
    for e in entries:
        key = (e["name"], e["version"])
        if key in derived:
            raise ValueError(f"{path.name} lists {key[0]} {key[1]} twice")
        if e["case"] not in CASES:
            raise ValueError(f"{path.name}: {key[0]} {key[1]} has case {e['case']!r}, not one of {', '.join(CASES)}")
        header_case = e["case"] != "standard"
        if header_case != ("file" in e) or header_case != ("headers" in e):
            raise ValueError(
                f"{path.name}: {key[0]} {key[1]} names a file and a header count exactly when its case is a header"
            )
        if header_case and not (isinstance(e["headers"], int) and e["headers"] >= 1):
            raise ValueError(f"{path.name}: {key[0]} {key[1]}'s header count is not a positive number")
        for t in e.get("extra_texts", []):
            if t not in STANDARD:
                raise ValueError(f"{path.name}: {key[0]} {key[1]} asks for {t}, whose standard text is not committed")
        derived[key] = e
    return derived


# What a header must contain to carry its licence's text whole: the
# grant and the disclaimer. A header-case entry must, a header+text one
# must not (that is what the appended standard text is for).
CARRIES = {
    "MIT": ("Permission is hereby granted", "AS IS"),
    "BSD-2-Clause": ("Redistribution and use", "AS IS"),
    "BSD-3-Clause": ("Redistribution and use", "AS IS"),
    "ISC": ("Permission to use, copy, modify", "AS IS"),
}


# A licence a header may name, by the words it is named with, and the
# standard texts any one of which answers it (decision 2.ii: a header
# naming a licence the notice does not otherwise carry brings its text).
NAMED = (
    (re.compile(r"\bBSD\b"), ("BSD-2-Clause", "BSD-3-Clause")),
    (re.compile(r"\bApache License\b|\bApache-2\.0\b"), ("Apache-2.0",)),
    (re.compile(r"\bMIT\b"), ("MIT",)),
    (re.compile(r"\bISC\b"), ("ISC",)),
)


def carries_text(block: str, term: str) -> bool:
    flat = " ".join(block.split())
    return term in CARRIES and all(marker in flat for marker in CARRIES[term])


def elected(expr: str) -> str:
    """Decision 4: under a derived notice, the first term an OR lists --
    and only among OR alternatives: an AND or a WITH names a term that
    is owed as well, so it is refused rather than dropped."""
    if re.search(r"\b(AND|WITH)\b", expr):
        raise ValueError(f"the licence expression {expr!r} is not a plain OR of single terms")
    first = expr.split(" OR ")[0].strip().strip("()").strip()
    if not re.fullmatch(r"[A-Za-z0-9.+-]+", first):
        raise ValueError(f"the licence expression {expr!r} has no single first term")
    return first


def standard_text(term: str) -> tuple:
    if term not in STANDARD:
        raise ValueError(f"no standard text of {term} is committed under {TEXTS.name}/")
    path = TEXTS / f"{term}.txt"
    text = path.read_text()
    digest, placeholder = STANDARD[term]
    if hashlib.sha256(text.encode()).hexdigest() != digest:
        raise ValueError(f"{TEXTS.name}/{path.name} is not the pinned SPDX text")
    return text, placeholder


def comment_block(source: str):
    """The leading comment block of a source file -- `//` lines and `/* */`
    blocks, doc comments excluded -- when it carries a copyright line
    (amendment of 2026-10-22: a bare SPDX tag is a declaration, not a
    notice); otherwise None."""
    lines, out, i = source.splitlines(), [], 0
    while i < len(lines):
        s = lines[i].strip()
        if not s:
            i += 1
            continue
        if s.startswith("//") and not s.startswith(("///", "//!")):
            out.append(s[2:].removeprefix(" "))
            i += 1
            continue
        if s.startswith("/*") and not s.startswith(("/**", "/*!")):
            body = []
            while i < len(lines):
                body.append(lines[i])
                i += 1
                if "*/" in body[-1]:
                    break
            for b in "\n".join(body).strip().removeprefix("/*").removesuffix("*/").splitlines():
                out.append(re.sub(r"^\s*\* ?", "", b).rstrip())
            continue
        break
    block = "\n".join(out).strip()
    # A copyright LINE, not the word: "preserves copyright notices" or a
    # CC0 dedication is no notice.
    return block if re.search(r"^\s*(copyright\b|\(c\)|\u00a9)", block, re.I | re.M) else None


def headers(package: dict) -> list:
    """[(path, text)] of every distinct copyright header among the files
    compiled in -- the root module, src/ and build.rs; tests/, examples/
    and benches/ are not -- deduplicated by text, the root module's
    first and the rest in sorted order (decision 2.ii, A 2026-10-22)."""
    root = Path(package["manifest_path"]).parent
    lib = next(
        (Path(t["src_path"]) for t in package["targets"]
         if set(t["kind"]) & {"lib", "rlib", "dylib", "proc-macro"}),
        None,
    )
    compiled = set((root / "src").rglob("*.rs")) | ({root / "build.rs"} if (root / "build.rs").is_file() else set())
    candidates = ([lib] if lib else []) + sorted(f for f in compiled if f != lib)
    found, seen = [], set()
    for f in candidates:
        block = comment_block(f.read_text(errors="replace"))
        if block and block not in seen:
            seen.add(block)
            found.append((f.relative_to(root).as_posix(), block))
    return found


def without_placeholder(text: str, placeholder) -> str:
    """A standard text under a header that has its own copyright line:
    the text's placeholder line goes rather than shipping unfilled."""
    if not placeholder:
        return text
    return "\n".join(l for l in text.splitlines() if placeholder not in l).lstrip("\n") + "\n"


def derived_notices(package: dict, entry: dict) -> list:
    """The notice texts of a crate that publishes no licence file, as its
    entry says and as its own source confirms: [(text, what it is)]."""
    name, version = package["name"], package["version"]
    expr = package.get("license") or ""
    if entry["license"] != expr:
        raise ValueError(f"{name} {version} declares {expr!r}; notice_derived.json lists {entry['license']!r}")
    found = headers(package)
    if entry["case"] in ("header", "header+text"):
        if not found:
            raise ValueError(f"{name} {version} is listed as {entry['case']} but has no copyright header")
        if found[0][0] != entry["file"]:
            raise ValueError(f"{name} {version}'s header is in {found[0][0]}; notice_derived.json names {entry['file']}")
        if len(found) != entry["headers"]:
            raise ValueError(
                f"{name} {version} has {len(found)} distinct copyright header(s); notice_derived.json says"
                f" {entry['headers']}: look at them again"
            )
        term = elected(expr)
        carried = any(carries_text(text, term) for _, text in found)
        if entry["case"] == "header" and not carried:
            raise ValueError(
                f"{name} {version}'s headers do not carry the {term} text: list it as header+text"
            )
        if entry["case"] == "header+text" and carried:
            raise ValueError(
                f"{name} {version}'s headers carry the {term} text already: list it as header"
            )
        extra = entry.get("extra_texts", [])
        # A licence a header names that is not one of the crate's own
        # terms is answered by a text the header carries or one the entry
        # attaches; otherwise its text would never ship.
        own = set(re.findall(r"[A-Za-z0-9.+-]+", expr)) - {"OR", "AND", "WITH"}
        for path, text in found:
            for words, answers in NAMED:
                if not words.search(text) or own & set(answers):
                    continue
                if not (set(answers) & set(extra) or any(carries_text(text, a) for a in answers)):
                    raise ValueError(
                        f"{name} {version}'s header in {path} names {answers[-1]}, whose text it does not carry:"
                        f" add it to the entry's extra_texts"
                    )
        notices = [(text + "\n", f"header of {path}") for path, text in found]
        if entry["case"] == "header+text":
            text, placeholder = standard_text(term)
            notices.append((without_placeholder(text, placeholder), f"standard {term} text"))
        for t in extra:
            text, placeholder = standard_text(t)
            notices.append((without_placeholder(text, placeholder), f"standard {t} text"))
        return notices
    if found:
        raise ValueError(
            f"{name} {version} has a copyright header in {found[0][0]}: list it as header or header+text"
        )
    term = elected(expr)
    text, placeholder = standard_text(term)
    holder = ", ".join(package.get("authors") or []) or f"the {name} authors"
    if placeholder:
        text = text.replace(placeholder, holder)
    else:
        text = f"Copyright (c) {holder}\n\n{text}"
    sentence = (
        f"{name} {version} publishes no notice of its own: this is the standard text of {term},"
        " the licence it declares, under its authors as its metadata names them."
    )
    return [(sentence + "\n\n" + text, f"standard {term} text")]


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


def render(third: list, derived: dict) -> tuple:
    texts, order, lines = {}, [], []
    for package in third:
        name, version = package["name"], package["version"]
        expr = package.get("license") or ""
        files = licence_files(package)
        entry = derived.get((name, version))
        if files and entry:
            raise ValueError(
                f"{name} {version} publishes a licence file now: remove its entry from notice_derived.json"
            )
        if not expr and not files:
            raise ValueError(f"{name} {version} has no licence expression and no licence file")
        if not files and not entry:
            raise ValueError(
                f"{name} {version} ({expr}) publishes no licence file and notice_derived.json does not"
                " list it, so its notice cannot be shipped (ADR-0058 decision 3)"
            )
        if files:
            notices = [(path.read_text(errors="replace").strip() + "\n", path.name) for path in files]
            how = ""
        else:
            notices = derived_notices(package, entry)
            how = f" (derived: {entry['case']})"
        refs = []
        for text, what in notices:
            digest = hashlib.sha256(text.encode()).hexdigest()
            if digest not in texts:
                texts[digest] = {"text": text, "label": f"T{len(texts) + 1}", "users": []}
                order.append(digest)
            texts[digest]["users"].append(f"{name} {version} ({what})")
            refs.append(texts[digest]["label"])
        lines.append(f"- `{name}` {version}: {expr or 'see its licence file'}{how} — text " + ", ".join(refs))
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
    return "\n".join(out) + "\n", len(order)


def as_section(page: str) -> str:
    """The page from its list on -- without its title and the paragraph
    under it, which speak of "these programs" where the release page
    says in its own sentence whose crates these are -- each heading one
    level down, fenced text left as it is."""
    lines = page.splitlines()
    start = next(i for i, l in enumerate(lines) if l.startswith("- `"))
    out, fenced = [], False
    for line in lines[start:]:
        if line.startswith("```"):
            fenced = not fenced
        elif not fenced and line.startswith("#"):
            line = "#" + line
        out.append(line)
    return "\n".join(out).strip("\n") + "\n"


def run(argv: list) -> int:
    if len(argv) == 2 and argv[1] in ("-h", "--help"):
        sys.stdout.write(help_text())
        return 0
    key = None
    if len(argv) > 1 and argv[1] == "--json":
        if len(argv) < 3:
            sys.stderr.write("usage: release_notices.py [--json KEY] <metadata.json> <compiled.txt> <out> <package>...\n")
            return 2
        key, argv = argv[2], argv[:1] + argv[3:]
    if len(argv) < 5:
        sys.stderr.write("usage: release_notices.py [--json KEY] <metadata.json> <compiled.txt> <out> <package>...\n")
        return 2
    try:
        metadata = json.loads(Path(argv[1]).read_text())
        compiled = compiled_set(Path(argv[2]).read_text())
        third = shipped(metadata, argv[4:], compiled)
        derived = load_derived(Path(os.environ.get("NOTICE_DERIVED_JSON") or DERIVED))
        page, distinct = render(third, derived)
    except (OSError, ValueError, KeyError) as err:
        sys.stderr.write(f"release_notices: {err}\n")
        return 1
    if key is not None:
        page = json.dumps({key: as_section(page)}) + "\n"
    publish(Path(argv[3]), page)
    print(f"{len(third)} crates, {distinct} distinct licence texts")
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
    rest = argv[2:] if len(argv) > 2 and argv[1] == "--json" else argv
    out = Path(rest[3]) if len(rest) >= 5 else None
    status = 1
    try:
        status = run(argv)
    finally:
        if status != 0 and out is not None:
            out.unlink(missing_ok=True)
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv))
