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
#   tools/ci/release_page.py <template> <values.json> <out.md>
#
# <values.json> is one JSON object of strings, one per placeholder. The
# fill REFUSES, writing nothing:
#   - a placeholder of the template with no value (a page with a literal
#     {apk_sha256} on it would read as published and check nothing);
#   - a value the template has no placeholder for (a misspelt key would
#     otherwise drop its value silently, and the placeholder above
#     catches only the half of a typo that is in the template);
#   - an empty value, or a value that is not a string;
#   - a `version` that is not MAJOR.MINOR.PATCH, the form RELEASE.md
#     gives the version name and the archive and APK names carry.
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
import re
import sys
from pathlib import Path

PLACEHOLDER = re.compile(r"\{([a-z0-9_]+)\}")
COMMENT = re.compile(r"<!--.*?-->\n?", re.DOTALL)
VERSION = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")


def help_text() -> str:
    lines = Path(__file__).read_text().splitlines()
    start = lines.index("# >>> help") + 1
    end = lines.index("# <<< help")
    return "\n".join(line[2:] for line in lines[start:end]) + "\n"


def fill(template: str, values: dict) -> str:
    """The page, or ValueError naming the first refusal."""
    body = COMMENT.sub("", template).lstrip("\n")
    names = set(PLACEHOLDER.findall(body))
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
    return PLACEHOLDER.sub(lambda m: values[m.group(1)], body)


def main(argv: list) -> int:
    if len(argv) == 2 and argv[1] in ("-h", "--help"):
        sys.stdout.write(help_text())
        return 0
    if len(argv) != 4:
        sys.stderr.write("usage: release_page.py <template> <values.json> <out.md>\n")
        return 2
    template_path, values_path, out_path = (Path(a) for a in argv[1:])
    try:
        values = json.loads(values_path.read_text())
    except (OSError, ValueError) as err:
        sys.stderr.write(f"release_page: cannot read {values_path}: {err}\n")
        return 1
    if not isinstance(values, dict):
        sys.stderr.write(f"release_page: {values_path} is not one JSON object\n")
        return 1
    try:
        page = fill(template_path.read_text(), values)
    except OSError as err:
        sys.stderr.write(f"release_page: cannot read {template_path}: {err}\n")
        return 1
    except ValueError as err:
        sys.stderr.write(f"release_page: {err}\n")
        return 1
    out_path.write_text(page)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
