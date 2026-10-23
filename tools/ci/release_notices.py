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
# list it (a header naming BSD with no BSD text attached is refused, and
# so is one naming a licence whose text is not committed under
# tools/ci/licence_texts/, or a name the scan reads and does not
# recognise: a "<name> license" phrase in any case, an SPDX id, an
# id-shaped variant such as BSD-4-Clause). That scan is a tripwire and
# closes nothing (ADR-0058 A 2026-10-22 (iii)): it cannot read a licence
# named without the word, a version or a variant written apart from its
# name ("MIT No Attribution"), or a name beside a known one ("Frob and MIT
# licenses").
# Each header-case entry also names the root header's file and the SHA-256
# of each distinct header's text, whitespace folded (runs of whitespace one
# space, the ends trimmed). A header whose digest the entry does not list
# is refused, naming the crate, the file and the digest, and so is a listed
# digest no header carries: the list is where a person has read every
# header text that ships, and its extra_texts say what that reading found.
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
                f"{path.name}: {key[0]} {key[1]} names a file and its headers' digests exactly when its case is a header"
            )
        if header_case and not (
            isinstance(e["headers"], list)
            and e["headers"]
            and all(isinstance(d, str) and re.fullmatch(r"[0-9a-f]{64}", d) for d in e["headers"])
            and len(set(e["headers"])) == len(e["headers"])
        ):
            raise ValueError(
                f"{path.name}: {key[0]} {key[1]}'s headers is not a list of distinct SHA-256 digests"
                " (ADR-0058 A 2026-10-22 (iii): one per header text read, not a count)"
            )
        if "extra_texts" in e and not header_case:
            raise ValueError(f"{path.name}: {key[0]} {key[1]} has extra_texts, which only a header case ships")
        for t in e.get("extra_texts", []):
            if t not in STANDARD:
                raise ValueError(f"{path.name}: {key[0]} {key[1]} asks for {t}, whose standard text is not committed")
        derived[key] = e
    return derived


# A header carries its licence's text only when it holds the committed
# standard text whole, from where the grant begins (the copyright line
# above it is the header's own; Apache's title is its start) to where
# the terms end (Apache's appendix, a how-to with its own placeholder,
# is not the terms),
# compared with whitespace and case folded: a notice cut anywhere, even
# after its last marker, does not count. A term with no committed text,
# or a header that rewords it (a BSD header naming its organisation), is
# header+text -- the standard text ships beside it.
BODY = {
    "MIT": ("Permission is hereby granted", None),
    "BSD-3-Clause": ("Redistribution and use", None),
    "Apache-2.0": ("Apache License", "END OF TERMS AND CONDITIONS"),
}


def folded(text: str) -> str:
    return " ".join(text.split()).lower()


# A licence a header may name, by the words it is named with in any
# case, and the standard texts any one of which answers it (decision
# 2.ii: a header naming a licence the notice does not otherwise carry
# brings its text). A licence with no committed text is listed too: its
# name is recognised, and since extra_texts holds only committed texts,
# a header naming it refuses the run until its text is committed.
NAMED = tuple(
    (re.compile(words, re.I), answers)
    for words, answers in (
        (r"\bBSD\b", ("BSD-2-Clause", "BSD-3-Clause")),
        (r"\bApache\b", ("Apache-2.0",)),
        (r"\bMIT\b", ("MIT",)),
        (r"\bISC\b", ("ISC",)),
        (r"\bCC0\b|\bpublic domain dedication\b", ("CC0-1.0",)),
        (r"\bzlib\b", ("Zlib",)),
        (r"\bMPL\b|\bMozilla Public License\b", ("MPL-2.0",)),
        (
            r"\b(?:L|A)?GPL\b|\bGeneral Public License\b",
            ("GPL-2.0-only", "GPL-3.0-only", "LGPL-2.1-only", "LGPL-3.0-only", "AGPL-3.0-only"),
        ),
        (r"\bUnlicense\b", ("Unlicense",)),
        (r"\bBoost Software License\b|\bBSL-1\.0\b", ("BSL-1.0",)),
        (r"\bUnicode License\b|\bUnicode-(?:DFS-2016|3\.0)\b", ("Unicode-3.0", "Unicode-DFS-2016")),
    )
)

# Words that say a header names SOME licence. A header that says so and
# names none NAMED recognises is refused rather than read as naming
# nothing: the list is closed, and what it does not know is never
# waved through (`a header naming an unrecognised licence is refused`).
LICENCE_WORDS = re.compile(
    r"\blicen[cs](?:e|ed|es|ing)\b|\bpublic domain\b|\bSPDX-License-Identifier\b", re.I
)


# A licence named in a header beside one NAMED knows is checked too, once
# the committed texts the header carries whole are set aside (their own
# "Grant of Copyright License" names no licence):
#   - the words next before each "license" or "licence", in any case,
#     must name a licence NAMED knows, unless they are generic ("found in
#     the LICENSE file", "Grant of Patent License");
#   - each SPDX id, on an SPDX-License-Identifier line in any case, and
#     each id-shaped variant of a known name in prose ("BSD-4-Clause",
#     "MIT-0", "Apache-1.1"), must be an id NAMED answers exactly.
# A word scan cannot close this: a licence named with no licence word
# ("and Frob", "GPLv3"), or a version written apart from its name
# ("Apache License, Version 1.1"), is beyond what it can read, and the
# reviewed list's reading of each header is what covers them.
# Not a file's or a URL's "LICENSE": LICENSE-APACHE, LICENSE.chromium,
# .../licenses/MIT.
# No "/" inside a word: "MIT/Frob License" is read as "Frob License".
NAME_PHRASE = re.compile(r"((?:[\w.+\"'()-]+\s+){1,4})(?<![/\w])licen[cs]es?\b(?![-./]\w)", re.I)
CONNECTIVES = {
    "a", "additional", "all", "an", "and", "any", "applicable", "by", "compatible", "different",
    "each", "entire", "following", "for", "found", "from", "full", "in", "its", "of", "or",
    "original", "other", "our", "permissive", "respective", "same", "see", "separate", "such",
    "that", "the", "their", "these", "this", "to", "under", "with", "your",
    # A grant's adjectives: "a perpetual, irrevocable, unconditional License".
    "irrevocable", "limited", "non-exclusive", "non-sublicensable", "non-transferable",
    "perpetual", "royalty-free", "unconditional", "worldwide",
}
# Words that, standing alone before "License", name no licence.
GENERIC = {"copyright", "patent", "public", "software", "grant", "no"}
SPDX_LINE = re.compile(r"spdx-license-identifier\s*:?[ \t]*([^\n]*)", re.I)
# A variant carries a number (BSD-4-Clause, MIT-0); "BSD-style" names none.
# It ends on a letter, digit or "+", never a sentence's full stop.
ID_SHAPED = re.compile(r"\b(?:A?GPL|LGPL|BSD|MIT|Apache|CC0|MPL|ISC|Zlib|BSL|Unicode)-(?=[\w.+-]*\d)[\w.+-]*[\w+]", re.I)



def known_ids() -> dict:
    """Every id NAMED answers with, and every committed text, by lower case."""
    ids = {t for _, answers in NAMED for t in answers} | set(STANDARD)
    return {t.lower(): t for t in ids}


def unrecognised_names(block: str) -> list:
    """The licence names in `block` NAMED does not recognise."""
    text = " ".join(block.split())
    lowered = text.lower()
    for term, (start, end) in BODY.items():
        if carries_text(block, term):
            body = folded(standard_text(term)[0])
            body = body[body.index(folded(start)):]
            if end:
                body = body[: body.index(folded(end)) + len(folded(end))]
            at = lowered.index(body)
            text, lowered = text[:at] + text[at + len(body):], lowered[:at] + lowered[at + len(body):]
    found = []
    for m in NAME_PHRASE.finditer(text):
        words = m.group(1).split()
        # A name does not run across a sentence or out of a parenthesis:
        # "Trademarks. This License", "(... this section) patent license".
        while any(w[-1] in ".:;,()" for w in words[:-1]):
            words = words[1:]
        words = [w.strip("\"'(),.:;") for w in words]
        run = []
        for w in reversed(words):
            if w.lower() in CONNECTIVES or not w:
                break
            run.insert(0, w)
        if not run or all(w.lower() in GENERIC for w in run):
            continue
        if not any(n.search(" ".join(run) + " License") for n, _ in NAMED):
            found.append(" ".join(run) + " License")
    ids = known_ids()
    found += [t for t in id_tokens(text, block) if t.lower() not in ids]
    return list(dict.fromkeys(found))


def id_tokens(text: str, block: str) -> list:
    """Every id the header writes: its SPDX lines' and the id-shaped words
    in its prose, a sentence's punctuation trimmed."""
    tokens = []
    for m in SPDX_LINE.finditer(block):
        # An expression wrapped onto the header's next line -- cut where an
        # id is due ("MIT OR" / "Apache-2.0"), or with the next line going
        # on with it (wraps_on) -- goes on into it, as a reflow of the same
        # words reads; one line at a time, so a copyright line or a licence
        # body after it is not read as ids. A blank line ends it: cut short
        # there, or with no line after it, it is refused as cut short.
        line = m.group(1)
        rest = block[m.end():].splitlines()[1:]
        ids = spdx_expression(line)
        while rest and rest[0].strip() and (ids[-1] == CUT_SHORT or wraps_on(rest[0])):
            line, rest = f"{line} {rest[0]}", rest[1:]
            ids = spdx_expression(line)
        tokens += ids
    tokens += [m.group(0) for m in ID_SHAPED.finditer(text)]
    return [t.rstrip(".") for t in tokens if t.rstrip(".")]


CUT_SHORT = "an SPDX expression cut short"
# An id among the words after the prose that ends an expression: a name,
# a hyphen and a version, a LicenseRef-, or an id this tool knows.
ID_WORD = re.compile(r"[A-Za-z][A-Za-z0-9.+]*-[A-Za-z0-9.+-]*\d[A-Za-z0-9.+-]*|LicenseRef-[A-Za-z0-9.+-]+")


def wraps_on(next_line: str) -> bool:
    """Whether the line after a complete SPDX expression goes on with it:
    it opens, parentheses aside, on an operator in ANY case -- glued to
    its word or not ("OR(LicenseRef-Frob)", ") AND X", "/LicenseRef-Frob",
    "or curl") -- or is a line of parentheses alone, which closes the
    expression. SPDX operators and ids are case-insensitive, so no case
    or shape of the next word can tell an id from prose: every guess at
    prose was a wrap whose id was dropped with no error, where the same
    words on one line refuse it (`or ( LicenseRef-Frob )`, `with
    LLVM-exception`, `or WTFPL`, `or wtfpl`, `or curl`: #386's reviews,
    two rounds). So a sentence opening on the word ("With contributions
    from ...") is read on and refused, which a reviewer sees, rather than
    an id passing unread, which nothing would."""
    # Every parenthesis and space before the first word, not one run then
    # the spaces: ") ) OR X" is the operator line ") OR X" is.
    head = next_line.lstrip("() \t")
    if not head or head[0] in "/|+":
        return True
    return bool(re.match(r"(AND|OR|WITH)(?![A-Za-z0-9.+-])", head, re.I))


def spdx_expression(line: str) -> list:
    """The ids of the SPDX expression that opens `line`: an id, then an
    operator (AND, OR, WITH in any case, or the legacy "/", "|", "+")
    before each further id, parentheses aside. Where an operator is due,
    it ends only at a word with no letter or digit, a parenthesis standing
    alone excepted ("MIT -- see LICENSE",
    "MIT */") or after an id that ends a sentence ("MIT. See the notice");
    any other word there is read as an id, so an unknown one refuses the
    run rather than being dropped. A word that is not a clean id
    ("MIT/Frob") is returned as it stands, so the lookup refuses it rather
    than reading less than the line names; and after the expression ends, an id-shaped word left
    on the line is read too: a name, a hyphen and a version ("MIT --
    EPL-2.0 for x.c"), a LicenseRef-, or an id this tool knows (ISC), each
    without the punctuation that follows it in prose. An expression cut
    where an id is due ("MIT OR" at the line's end) is returned with
    CUT_SHORT last, which id_tokens reads on into the next line and the
    lookup refuses where none follows."""
    ids, want_id, ended = [], True, False
    words = line.split()
    for at, word in enumerate(words):
        # A parenthesis standing alone groups, wherever it stands: it is
        # neither an id nor the prose that ends the expression ("( MIT ) OR X").
        if re.fullmatch(r"[()]+", word):
            continue
        bare = word.rstrip(".").strip("()").rstrip(".")
        if want_id:
            if not bare:
                continue
            ids.append(bare if re.fullmatch(r"[A-Za-z0-9.+-]+", bare) else f"{word} (not an SPDX id)")
            want_id, ended = False, word.endswith(".")
        elif bare.upper() in ("AND", "OR", "WITH") or word in ("/", "|", "+"):
            want_id = True
        elif ended or not re.search(r"[A-Za-z0-9]", word):
            known = known_ids()
            for w in words[at:]:
                w = w.strip("()\"'").rstrip(".,;:").strip("()\"'").rstrip(".,;:")
                if ID_WORD.fullmatch(w) or w.lower() in known:
                    ids.append(w)
            break
        else:
            ids.append(bare if re.fullmatch(r"[A-Za-z0-9.+-]+", bare) else f"{word} (not an SPDX id)")
            ended = word.endswith(".")
    if want_id:
        ids.append(CUT_SHORT)
    return ids


def owed_ids(block: str) -> list:
    """The known licence ids the header writes, in an SPDX line or its
    prose, each owed its own text: BSD-2-Clause is not answered by the
    BSD-3-Clause text."""
    ids = known_ids()
    text = " ".join(block.split())
    return [ids[t.lower()] for t in id_tokens(text, block) if t.lower() in ids]


def carries_text(block: str, term: str) -> bool:
    if term not in BODY:
        return False
    text = standard_text(term)[0]
    start, end = BODY[term]
    body = text[text.index(start):]
    if end:
        body = body[: body.index(end) + len(end)]
    return folded(body) in folded(block)


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
    line = r"^\s*(copyright\b|\(c\)|\u00a9|spdx-filecopyrighttext:|portions copyright\b)"
    return block if re.search(line, block, re.I | re.M) else None


def header_digest(text: str) -> str:
    """The SHA-256 of a header's text with its whitespace folded: runs of
    whitespace one space, the ends trimmed. Case is kept: a changed letter
    is a text a person reads again."""
    return hashlib.sha256(" ".join(text.split()).encode()).hexdigest()


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
        # Decision 3 (A 2026-10-22 (iii)): every header that ships is one a
        # person read, by its digest; a word-scan pass below proves nothing.
        listed = set(entry["headers"])
        for path, text in found:
            if header_digest(text) not in listed:
                raise ValueError(
                    f"{name} {version}'s header in {path} has digest {header_digest(text)}, which"
                    f" notice_derived.json does not list: read the header, record what it names in"
                    f" extra_texts, and add the digest"
                )
        for stale in sorted(listed - {header_digest(text) for _, text in found}):
            raise ValueError(
                f"{name} {version}'s entry lists digest {stale}, which no header carries: remove it"
            )
        term = elected(expr)
        # Decision 4: an OR is satisfied by any term whose text the crate
        # publishes whole; failing every one, header+text adds the first.
        own = [t.strip("()") for t in expr.split(" OR ")]
        carried = {t for t in own for _, text in found if carries_text(text, t)}
        if entry["case"] == "header" and not carried:
            raise ValueError(
                f"{name} {version}'s headers carry no whole text of {expr}: list it as header+text"
            )
        if entry["case"] == "header+text" and carried:
            raise ValueError(
                f"{name} {version}'s headers carry the {sorted(carried)[0]} text already: list it as header"
            )
        extra = entry.get("extra_texts", [])
        # Every licence any header names has its text in what ships --
        # carried by a header, the standard text header+text adds, or an
        # extra text -- the crate's own terms included, and an "at your
        # option" header's every alternative too (decision 2.ii as written:
        # a header naming a licence whose text the notice does not
        # otherwise carry brings that text). One comment block can hold
        # several holders' statements and the tool cannot tell them apart,
        # so it asks for every one; a text too many costs nothing.
        shipped = carried | set(extra) | ({term} if entry["case"] == "header+text" else set())
        for path, text in found:
            named = [a for words, a in NAMED if words.search(text)]
            unknown = unrecognised_names(text)
            if unknown or (not named and LICENCE_WORDS.search(text)):
                said = f" ({', '.join(unknown)})" if unknown else ""
                raise ValueError(
                    f"{name} {version}'s header in {path} names a licence this tool does not recognise{said}:"
                    f" add its words to NAMED and its text under {TEXTS.name}/"
                )
            # An SPDX id is owed its own text, never a sibling's: BSD-2-Clause
            # is not answered by the BSD-3-Clause text.
            for answers in named + [(t,) for t in owed_ids(text)]:
                if not (set(answers) & shipped or any(carries_text(text, x) for x in answers)):
                    if not set(answers) & set(STANDARD):
                        # extra_texts takes committed texts only, so the
                        # remedy is the text first (`a header naming zlib`).
                        raise ValueError(
                            f"{name} {version}'s header in {path} names {' or '.join(answers)}, whose standard"
                            f" text is not committed: commit and pin it under {TEXTS.name}/, then add it to"
                            f" the entry's extra_texts"
                        )
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
        "InterWeave's own code is under the Apache License 2.0 (the file",
        "`LICENSE` in its source code). The InterWeave software that comes",
        "with this file also contains the Rust libraries (called crates)",
        "listed below, each under its own licence. Each line names a crate,",
        "its version and its licence, and gives the labels of its licence",
        "texts (T1, T2 and so on). The licence texts follow the list, under",
        "the heading \"Licence texts\". A text that several crates share is",
        "printed once.",
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
        derived = load_derived(Path(os.environ.get("NOTICE_DERIVED_JSON") or DERIVED))
        page, distinct = render(third, derived)
    except (OSError, ValueError, KeyError) as err:
        sys.stderr.write(f"release_notices: {err}\n")
        return 1
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
