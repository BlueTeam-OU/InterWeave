#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_page.py
#
# >>> help
# Fill the release page's template for one release (plan section 22,
# ADR-0057 decision 1): every {placeholder} the template names gets the
# value given for it, and the page is written out.
#
#   tools/ci/release_page.py <template> <out.md> <values.json>...
#
# Each <values.json> is one JSON object of strings, and together they
# give one value per placeholder (the release run's own, and the files
# release_android_apk.sh, release_linux_archive.sh and skia_notices.py
# write). The fill REFUSES, writing nothing and removing an earlier
# run's <out.md>:
#   - a placeholder given a value by two of the files;
#   - a placeholder of the template with no value (a page with a literal
#     {apk_sha256} on it would read as published and check nothing);
#   - a value the template has no placeholder for (a misspelt key would
#     otherwise drop its value silently, and the placeholder above
#     catches only the half of a typo that is in the template);
#   - an empty value, or a value that is not a string;
#   - a `version` that is not MAJOR.MINOR.PATCH, the form RELEASE.md
#     gives the version name and the archive and APK names carry;
#   - a hash that is not bound to this release: release_android_apk.sh
#     writes `check:apk_version` beside apk_sha256 and cert_sha256, and
#     release_linux_archive.sh `check:linux_version` beside linux_sha256,
#     each the version it built. A hash on the page whose check is
#     missing, or names another version, is refused, so a values file
#     left from another release cannot put its hash under this
#     release's file name. A `check:` key is not a placeholder.
#
# The template's HTML comments are its notes to the release run, not
# text for a reader, and are dropped before the fill, the SPDX line
# included. A value is inserted verbatim and never read for placeholders
# itself: a licence text may carry braces of its own.
#
# Exit codes:
#   0  the page is written
#   1  a refusal above (named on stderr)
#   2  usage
# <<< help

import json
import os
import re
import sys
from pathlib import Path

PLACEHOLDER = re.compile(r"\{([a-z0-9_]+)\}")
COMMENT = re.compile(r"<!--.*?-->\n?", re.DOTALL)
VERSION = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")
# Which check binds which hash to the version it was made for.
BOUND = {
    "apk_sha256": "check:apk_version",
    "cert_sha256": "check:apk_version",
    "linux_sha256": "check:linux_version",
}


def help_text() -> str:
    lines = Path(__file__).read_text().splitlines()
    start = lines.index("# >>> help") + 1
    end = lines.index("# <<< help")
    return "\n".join(line[2:] for line in lines[start:end]) + "\n"


def fill(template: str, values: dict) -> str:
    """The page, or ValueError naming the first refusal."""
    body = COMMENT.sub("", template).lstrip("\n")
    names = set(PLACEHOLDER.findall(body))
    checks = {k: v for k, v in values.items() if k.startswith("check:")}
    values = {k: v for k, v in values.items() if not k.startswith("check:")}
    missing = sorted(names - values.keys())
    if missing:
        raise ValueError("no value for " + ", ".join("{%s}" % n for n in missing))
    unknown = sorted(values.keys() - names)
    if unknown:
        raise ValueError("the template has no placeholder for " + ", ".join(unknown))
    for name, value in sorted(values.items()):
        if not isinstance(value, str) or not value.strip():
            raise ValueError(f"the value of {name} is empty or not a string")
    if "version" in values and not VERSION.fullmatch(values["version"]):
        raise ValueError(f"version {values['version']!r} is not MAJOR.MINOR.PATCH")
    for name, check in sorted(BOUND.items()):
        if name not in values:
            continue
        if check not in checks:
            raise ValueError(f"{name} comes with no {check}: it is not bound to a release")
        if checks[check] != values.get("version"):
            raise ValueError(
                f"{name} was made for {checks[check]!r}, not this release's {values.get('version')!r}"
            )
    unused = sorted(set(checks) - set(BOUND.values()))
    if unused:
        raise ValueError("unknown check " + ", ".join(unused))
    return PLACEHOLDER.sub(lambda m: values[m.group(1)], body)


def run(argv: list) -> int:
    if len(argv) == 2 and argv[1] in ("-h", "--help"):
        sys.stdout.write(help_text())
        return 0
    if len(argv) < 4:
        sys.stderr.write("usage: release_page.py <template> <out.md> <values.json>...\n")
        return 2
    template_path, out_path = Path(argv[1]), Path(argv[2])
    values, source = {}, {}
    for values_path in (Path(a) for a in argv[3:]):
        try:
            part = json.loads(values_path.read_text())
        except (OSError, ValueError) as err:
            sys.stderr.write(f"release_page: cannot read {values_path}: {err}\n")
            return 1
        if not isinstance(part, dict):
            sys.stderr.write(f"release_page: {values_path} is not one JSON object\n")
            return 1
        for name in part:
            if name in values:
                sys.stderr.write(f"release_page: {name} is given by both {source[name]} and {values_path}\n")
                return 1
            source[name] = values_path
        values.update(part)
    try:
        page = fill(template_path.read_text(), values)
    except OSError as err:
        sys.stderr.write(f"release_page: cannot read {template_path}: {err}\n")
        return 1
    except ValueError as err:
        sys.stderr.write(f"release_page: {err}\n")
        return 1
    publish(out_path, page)
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
    out = Path(argv[2]) if len(argv) >= 4 else None
    status = 1
    try:
        status = run(argv)
    finally:
        if status != 0 and out is not None:
            out.unlink(missing_ok=True)
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv))
