#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_skia_notices.sh
#
# Self-test for skia_notices.py, offline.
#
# A sandbox pin names stand-in texts read from a sandbox directory
# (SKIA_NOTICES_SOURCE); the happy path, a C source whose licence header
# is taken, is the positive control for the refusals. The repository's
# own pin is parsed with the same parser.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/skia_notices.py"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(realpath -- "$(mktemp -d)")"
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

SRC="$SANDBOX/src"
mkdir -p "$SRC"
printf 'The A licence.\n' > "$SRC/A-LICENSE"
printf 'The B licence, part one.\n' > "$SRC/B-ONE"
printf '/*\n * Copyright (C) 2010 Someone\n * Some rights kept.\n */\n\nint code;\n' > "$SRC/c.c"
# A stand-in FTL.TXT: the credit sentence between its three-quote marks.
printf 'The licence.\n\n   """\n    Portions of this software are copyright © <year> Someone\n    (https://example.invalid).  Some rights kept.\n   """\n\nMore.\n' > "$SRC/FTL.TXT"
sha() { sha256sum "$SRC/$1" | cut -d' ' -f1; }

write_pin() {
    {
        echo "# a comment"
        echo "Alpha | LICENSE | https://example.invalid/a/A-LICENSE | ${1:-$(sha A-LICENSE)}"
        echo "Beta | ONE | gitiles:https://example.invalid/b/+/abc/B-ONE | $(sha B-ONE)"
        echo "Gamma | c.c (its licence header) | ndk:1.0/sources/c.c | $(sha c.c)"
        echo "Delta | docs/FTL.TXT | https://example.invalid/d/docs/FTL.TXT | $(sha FTL.TXT)"
        echo "credit-year | 2026"
        [[ -n "${2-}" ]] && echo "$2"
    } > "$SANDBOX/pin"
}
run() {
    SKIA_NOTICES_PIN="$SANDBOX/pin" SKIA_NOTICES_SOURCE="$SRC" \
        python3 "$UNDER_TEST" "$SANDBOX/out.json" >"$SANDBOX/stdout" 2>"$SANDBOX/err"
}

echo "skia_notices.py:"
write_pin
if run; then
    got="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d["freetype_credit"]); print("--"); print(d["third_party_notice_texts"])' "$SANDBOX/out.json")"
    want="$(cat <<'EOF'
Portions of this software are copyright © 2026 Someone (https://example.invalid).  Some rights kept.
--
### Alpha

From `LICENSE`:

```text
The A licence.
```

### Beta

From `ONE`:

```text
The B licence, part one.
```

### Gamma

From `c.c (its licence header)`:

```text
Copyright (C) 2010 Someone
Some rights kept.
```

### Delta

From `docs/FTL.TXT`:

```text
The licence.

   """
    Portions of this software are copyright © <year> Someone
    (https://example.invalid).  Some rights kept.
   """

More.
```
EOF
)"
    [[ "$got" == "$want" ]] && pass "writes FTL.TXT's credit with the pin's year, and every text, a C file's header alone" \
        || fail "the written notices are not the expected ones" "$(diff <(echo "$want") <(echo "$got"))"
else
    fail "a complete pin was refused" "$(cat "$SANDBOX/err")"
fi

rm -f "$SANDBOX/out.json"
write_pin "$(printf '0%.0s' {1..64})"
if run; then fail "a text whose SHA-256 is not the pin's was accepted"
elif [[ -e "$SANDBOX/out.json" ]]; then fail "a refused text still wrote the notices"
elif ! grep -qF "Alpha LICENSE has SHA-256 $(sha A-LICENSE), not the pin's" "$SANDBOX/err"; then
    fail "a changed text was refused without naming it" "$(cat "$SANDBOX/err")"
else pass "a changed text is refused, nothing written"; fi

write_pin "" "credit-year | 2025"
run; rc=$?
[[ $rc -eq 2 ]] && grep -qF '2 credit-year lines' "$SANDBOX/err" \
    && pass "two credit-year lines are a malformed pin (2)" || fail "two credit-year lines exited $rc"
cp "$SRC/FTL.TXT" "$SRC/FTL.keep"
printf 'The licence, with no credit sentence in it.\n' > "$SRC/FTL.TXT"
write_pin
run; rc=$?
[[ $rc -eq 1 && ! -e "$SANDBOX/out.json" ]] && grep -qF 'no single credit sentence' "$SANDBOX/err" \
    && pass "an FTL.TXT with no credit sentence is refused (1), nothing written" \
    || fail "an FTL.TXT with no credit sentence exited $rc" "$(cat "$SANDBOX/err")"
mv "$SRC/FTL.keep" "$SRC/FTL.TXT"
write_pin "" "Delta | x | https://example.invalid/x"
run; rc=$?
[[ $rc -eq 2 ]] && pass "a line without a SHA-256 is a malformed pin (2)" || fail "a line without a SHA-256 exited $rc"
write_pin
rm "$SRC/B-ONE"
run; rc=$?
[[ $rc -eq 1 ]] && pass "a source that cannot be read is refused (1)" || fail "a missing source exited $rc"

echo "the repository's pin:"
summary="$(python3 - "$UNDER_TEST" "$SCRIPT_DIR/skia-notices.pin" <<'EOF'
import importlib.util, sys
spec = importlib.util.spec_from_file_location("n", sys.argv[1]); n = importlib.util.module_from_spec(spec)
spec.loader.exec_module(n)
entries, credit = n.parse(open(sys.argv[2]).read())
print(len(entries), credit.isdigit() and any(n.endswith("FTL.TXT") for _, n, _, _ in entries))
EOF
)"
read -r count has_ft <<<"$summary"
if [[ "${count:-0}" -gt 0 && "$has_ft" == True ]]; then
    pass "tools/ci/skia-notices.pin parses: $count texts, an FTL.TXT and a credit year"
else
    fail "tools/ci/skia-notices.pin does not parse as expected" "$summary"
fi

if (( failures )); then
    echo "test_skia_notices.sh: $failures failure(s)" >&2
    exit 1
fi
echo "test_skia_notices.sh: all assertions passed"
