#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/skia_notices.py
#
# >>> help
# Fetch the licence texts the APK's native Skia library owes and write
# them for the release page (ADR-0057 decisions 3 and 4):
#
#   tools/ci/skia_notices.py <out.json>
#
# <out.json> is {"freetype_credit": ..., "third_party_notice_texts": ...},
# the two placeholders release_page.py fills from it. Every text named in
# tools/ci/skia-notices.pin is fetched and REFUSED, with nothing written,
# when its SHA-256 is not the pin's: a text that changed upstream is read
# again and the pin renewed by hand, never published unread. The pin
# must also hold exactly one credit line.
#
# Environment: SKIA_NOTICES_PIN (the pin; default beside this script);
# SKIA_NOTICES_SOURCE, a directory in which a source's text is the file
# named by the source's last path segment, read instead of the network
# (the self-test's); ANDROID_NDK_HOME or ANDROID_HOME for an ndk: source.
#
# Exit codes:
#   0  written
#   1  a refusal above, or a source that cannot be read
#   2  usage, or a malformed pin
# <<< help

import base64
import hashlib
import json
import os
import re
import sys
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent


class Malformed(Exception):
    pass


def help_text() -> str:
    lines = Path(__file__).read_text().splitlines()
    start = lines.index("# >>> help") + 1
    end = lines.index("# <<< help")
    return "\n".join(line[2:] for line in lines[start:end]) + "\n"


def parse(pin: str) -> tuple:
    entries, credits = [], []
    for number, raw in enumerate(pin.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        fields = [f.strip() for f in line.split(" | ")]
        if fields[0] == "credit" and len(fields) == 2 and fields[1]:
            credits.append(fields[1])
        elif len(fields) == 4 and all(fields) and re.fullmatch(r"[0-9a-f]{64}", fields[3]):
            entries.append(tuple(fields))
        else:
            raise Malformed(f"pin line {number} is not 'component | file | source | sha256' or 'credit | text'")
    if len(credits) != 1:
        raise Malformed(f"the pin holds {len(credits)} credit lines, not one")
    if not entries:
        raise Malformed("the pin names no text")
    return entries, credits[0]


def ndk_path(source: str) -> Path:
    version, _, rel = source[len("ndk:"):].partition("/")
    if os.environ.get("ANDROID_NDK_HOME"):
        base = Path(os.environ["ANDROID_NDK_HOME"]).parent
    elif os.environ.get("ANDROID_HOME"):
        base = Path(os.environ["ANDROID_HOME"]) / "ndk"
    else:
        raise OSError("an ndk: source needs ANDROID_NDK_HOME or ANDROID_HOME")
    return base / version / rel


def read(source: str) -> bytes:
    local = os.environ.get("SKIA_NOTICES_SOURCE")
    if local:
        return (Path(local) / source.rstrip("/").split("/")[-1]).read_bytes()
    if source.startswith("ndk:"):
        return ndk_path(source).read_bytes()
    gitiles = source.startswith("gitiles:")
    url = source[len("gitiles:"):] + "?format=TEXT" if gitiles else source
    with urllib.request.urlopen(url, timeout=60) as response:
        body = response.read()
    return base64.b64decode(body) if gitiles else body


def header(text: str) -> str:
    """The first C comment of a source file: its licence header."""
    match = re.match(r"\s*/\*(.*?)\*/", text, re.DOTALL)
    if not match:
        raise ValueError("no leading comment to take a licence header from")
    lines = [re.sub(r"^\s*\* ?", "", l).rstrip() for l in match.group(1).splitlines()]
    return "\n".join(lines).strip("\n")


def build(entries: list) -> str:
    out, current = [], None
    for component, name, source, sha in entries:
        data = read(source)
        got = hashlib.sha256(data).hexdigest()
        if got != sha:
            raise ValueError(f"{component} {name} has SHA-256 {got}, not the pin's {sha}: read it and renew the pin")
        text = data.decode("utf-8", errors="replace")
        if source.startswith("ndk:"):
            text = header(text)
        if component != current:
            out += [f"### {component}", ""]
            current = component
        out += [f"From `{name}`:", "", "```text", text.strip("\n"), "```", ""]
    return "\n".join(out).rstrip("\n")


def main(argv: list) -> int:
    if len(argv) == 2 and argv[1] in ("-h", "--help"):
        sys.stdout.write(help_text())
        return 0
    if len(argv) != 2:
        sys.stderr.write("usage: skia_notices.py <out.json>\n")
        return 2
    pin = Path(os.environ.get("SKIA_NOTICES_PIN", HERE / "skia-notices.pin"))
    try:
        entries, credit = parse(pin.read_text())
    except (OSError, Malformed) as err:
        sys.stderr.write(f"skia_notices: {err}\n")
        return 2
    try:
        texts = build(entries)
    except (OSError, ValueError) as err:
        sys.stderr.write(f"skia_notices: {err}\n")
        return 1
    Path(argv[1]).write_text(json.dumps({"freetype_credit": credit, "third_party_notice_texts": texts}) + "\n")
    print(f"{len(entries)} texts checked against {pin}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
