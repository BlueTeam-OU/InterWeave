#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_release_page.sh
#
# Self-test for release_page.py.
#
# A small template stands for the shape (comments, placeholders, a value
# carrying braces); then the repository's own template is filled with a
# value for every placeholder it names -- the positive control -- and
# refused with each one left out in turn, so a placeholder the release
# run does not supply can never reach a published page.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"
UNDER_TEST="$SCRIPT_DIR/release_page.py"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }
PY="${PYTHON:-python3}"

failures=0
SANDBOX="$(realpath -- "$(mktemp -d)")"
trap 'rm -rf "$SANDBOX"' EXIT

pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

run() { "$PY" "$UNDER_TEST" "$@" 2>"$SANDBOX/err"; }

T="$SANDBOX/t.md.in"
cat > "$T" <<'EOF'
<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- A note to the run: fills each {placeholder}. -->
# InterWeave {version}

SHA-256: `{apk_sha256}`

{notices}
EOF

echo "release_page.py:"

cat > "$SANDBOX/v.json" <<'EOF'
{"version": "1.2.3", "apk_sha256": "ab12", "notices": "a text with {braces} of its own"}
EOF
if run "$T" "$SANDBOX/out.md" "$SANDBOX/v.json"; then
    want=$'# InterWeave 1.2.3\n\nSHA-256: `ab12`\n\na text with {braces} of its own'
    if [[ "$(cat "$SANDBOX/out.md")" == "$want" ]]; then
        pass "fills every placeholder, drops the comments, leaves a value's braces"
    else
        fail "the filled page is not the expected one" "$(cat "$SANDBOX/out.md")"
    fi
else
    fail "a complete set of values was refused" "$(cat "$SANDBOX/err")"
fi

refused() {
    local what="$1" json="$2" needle="$3"
    rm -f "$SANDBOX/out.md"
    printf '%s\n' "$json" > "$SANDBOX/v.json"
    if run "$T" "$SANDBOX/out.md" "$SANDBOX/v.json"; then
        fail "$what: accepted"
    elif [[ -e "$SANDBOX/out.md" ]]; then
        fail "$what: refused but wrote a page"
    elif ! grep -qF -- "$needle" "$SANDBOX/err"; then
        fail "$what: refused without naming $needle" "$(cat "$SANDBOX/err")"
    else
        pass "$what: refused, nothing written"
    fi
}

refused "a placeholder with no value" \
    '{"version": "1.2.3", "notices": "n"}' '{apk_sha256}'
refused "a value with no placeholder" \
    '{"version": "1.2.3", "apk_sha256": "ab", "notices": "n", "apk_sha265": "ab"}' 'apk_sha265'
refused "an empty value" \
    '{"version": "1.2.3", "apk_sha256": " ", "notices": "n"}' 'apk_sha256'
refused "a value that is not a string" \
    '{"version": "1.2.3", "apk_sha256": 12, "notices": "n"}' 'apk_sha256'
refused "a version that is not MAJOR.MINOR.PATCH" \
    '{"version": "v1.2", "apk_sha256": "ab", "notices": "n"}' "'v1.2'"
refused "a version with a leading zero" \
    '{"version": "1.02.3", "apk_sha256": "ab", "notices": "n"}' "'1.02.3'"

printf '[]' > "$SANDBOX/v.json"
if run "$T" "$SANDBOX/out.md" "$SANDBOX/v.json"; then
    fail "a JSON array was accepted as the values"
else
    pass "values that are not one JSON object are refused"
fi

# The release run's values come in several files: merged, and a key
# two of them give is refused.
printf '{"version": "1.2.3"}' > "$SANDBOX/v1.json"
printf '{"apk_sha256": "ab12", "notices": "n"}' > "$SANDBOX/v2.json"
if run "$T" "$SANDBOX/out.md" "$SANDBOX/v1.json" "$SANDBOX/v2.json" && grep -qx '# InterWeave 1.2.3' "$SANDBOX/out.md"; then
    pass "values from several files are merged"
else
    fail "values in several files were not merged" "$(cat "$SANDBOX/err")"
fi
printf '{"version": "1.2.4"}' > "$SANDBOX/v3.json"
rm -f "$SANDBOX/out.md"
if run "$T" "$SANDBOX/out.md" "$SANDBOX/v1.json" "$SANDBOX/v2.json" "$SANDBOX/v3.json"; then
    fail "a key two files give was accepted"
elif [[ ! -e "$SANDBOX/out.md" ]] && grep -qF 'version is given by both' "$SANDBOX/err"; then
    pass "a key two files give is refused, nothing written"
else
    fail "a key two files give was refused without saying so, or wrote a page" "$(cat "$SANDBOX/err")"
fi

run "$T" "$SANDBOX/out.md" >/dev/null
[[ $? -eq 2 ]] && pass "a missing argument is a usage error (2)" \
    || fail "a missing argument did not exit 2"

# The repository's template: every placeholder it names is filled, and
# each one left out is refused.
REAL="$ROOT/apps/human-android/release/release-page.md.in"
if [[ -f "$REAL" ]]; then
    names="$("$PY" - "$REAL" <<'EOF'
import re, sys
text = open(sys.argv[1]).read()
body = re.sub(r"<!--.*?-->\n?", "", text, flags=re.DOTALL)
print("\n".join(sorted(set(re.findall(r"\{([a-z0-9_]+)\}", body)))))
EOF
)"
    if [[ -z "$names" ]]; then
        fail "the repository's template names no placeholder"
    else
        all_values() {
            local skip="$1" n first=1
            printf '{'
            for n in $names; do
                [[ "$n" == "$skip" ]] && continue
                (( first )) || printf ','
                first=0
                if [[ "$n" == version ]]; then printf '"version":"0.1.0"'
                else printf '"%s":"value-of-%s"' "$n" "$n"; fi
            done
            printf '}'
        }
        all_values "" > "$SANDBOX/v.json"
        if run "$REAL" "$SANDBOX/out.md" "$SANDBOX/v.json" \
            && ! grep -qE '\{[a-z0-9_]+\}|<!--' "$SANDBOX/out.md"; then
            pass "the repository's template fills completely ($(wc -w <<<"$names") placeholders)"
        else
            fail "the repository's template did not fill completely" "$(cat "$SANDBOX/err")"
        fi
        for n in $names; do
            all_values "$n" > "$SANDBOX/v.json"
            if run "$REAL" "$SANDBOX/out.md" "$SANDBOX/v.json"; then
                fail "the repository's template filled without {$n}"
            fi
        done
        pass "the repository's template is refused with any one placeholder unfilled"
    fi
else
    fail "no template at $REAL"
fi

if (( failures )); then
    echo "test_release_page.sh: $failures failure(s)" >&2
    exit 1
fi
echo "test_release_page.sh: all assertions passed"
