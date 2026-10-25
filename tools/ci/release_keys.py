#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_keys.py
#
# >>> help
# Print one value of the release's pinned keys (packaging/release-keys.toml):
#
#   tools/ci/release_keys.py <keys.toml> <field>
#
# <field> is one of
#   tag.fingerprint               printed as 40 uppercase hex digits
#   tag.public_key                the armoured block, as written
#   archive.minisign_public_key   RW followed by 54 base64 characters
#
# The value is checked for its shape before it is printed, so a caller
# never hands gpg or minisign a half-pasted key. An empty value is a
# refusal of its own: a release with a key not yet pinned fails closed.
#
# Exit codes:
#   0  the value is printed
#   1  the value is empty or not of its shape
#   2  usage, or the file is not readable TOML
# <<< help

import base64
import re
import sys
import tomllib

FIELDS = ("tag.fingerprint", "tag.public_key", "archive.minisign_public_key")
ARMOUR_BEGIN = "-----BEGIN PGP PUBLIC KEY BLOCK-----"
ARMOUR_END = "-----END PGP PUBLIC KEY BLOCK-----"


def refuse(message: str) -> None:
    print(f"release_keys: {message}", file=sys.stderr)
    sys.exit(1)


def check(field: str, value: str) -> str:
    if field == "tag.fingerprint":
        fpr = value.upper()
        # A v4 key's fingerprint. Short and long key ids are refused: they
        # name a key only as far as nobody has made a colliding one.
        if not re.fullmatch(r"[0-9A-F]{40}", fpr):
            refuse(f"tag.fingerprint {value!r} is not 40 hex digits")
        return fpr
    if field == "tag.public_key":
        text = value.strip()
        # One block: a second BEGIN would let the pin carry a key the
        # fingerprint does not name; release_verify_tag.sh checks the
        # keyring it imports to, too.
        if not (text.startswith(ARMOUR_BEGIN) and text.endswith(ARMOUR_END)) or text.count(ARMOUR_BEGIN) != 1:
            refuse("tag.public_key is not one armoured PGP public key block")
        return text + "\n"
    # minisign: "Ed", an 8-byte key id and a 32-byte Ed25519 key, base64.
    if not re.fullmatch(r"RW[A-Za-z0-9+/]{54}", value):
        refuse(f"archive.minisign_public_key {value!r} is not a minisign public key")
    raw = base64.b64decode(value, validate=True)
    if len(raw) != 42 or raw[:2] != b"Ed":
        refuse(f"archive.minisign_public_key {value!r} is not an Ed25519 minisign key")
    return value


def main(argv: list[str]) -> int:
    if len(argv) == 2 and argv[1] in ("-h", "--help"):
        text = open(__file__, encoding="utf-8").read()
        block = text.split("# >>> help\n", 1)[1].split("# <<< help\n", 1)[0]
        print("".join(line[2:] if line.startswith("# ") else line[1:] for line in block.splitlines(True)), end="")
        return 0
    if len(argv) != 3 or argv[2] not in FIELDS:
        print("usage: release_keys.py <keys.toml> " + "|".join(FIELDS), file=sys.stderr)
        return 2
    try:
        with open(argv[1], "rb") as f:
            keys = tomllib.load(f)
    except (OSError, tomllib.TOMLDecodeError) as e:
        print(f"release_keys: cannot read {argv[1]}: {e}", file=sys.stderr)
        return 2
    table, name = argv[2].split(".")
    value = keys.get(table, {}).get(name, "")
    if not isinstance(value, str):
        refuse(f"{argv[2]} is not a string")
    if not value.strip():
        refuse(f"{argv[1]} pins no {argv[2]}: a release fails closed until the owner's key is pinned")
    print(check(argv[2], value.strip()), end="" if argv[2] == "tag.public_key" else "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
