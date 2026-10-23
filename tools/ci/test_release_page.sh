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
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_release_page: no sandbox (mktemp failed)" >&2; exit 1; }
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
{"version": "1.2.3", "apk_sha256": "ab12", "check:apk_version": "1.2.3", "notices": "a text with {braces} of its own"}
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
    # An earlier run's page, which a refusal must remove.
    printf 'an earlier run\n' > "$SANDBOX/out.md"
    printf '%s\n' "$json" > "$SANDBOX/v.json"
    if run "$T" "$SANDBOX/out.md" "$SANDBOX/v.json"; then
        fail "$what: accepted"
    elif [[ -e "$SANDBOX/out.md" ]]; then
        fail "$what: refused but left a page"
    elif ! grep -qF -- "$needle" "$SANDBOX/err"; then
        fail "$what: refused without naming $needle" "$(cat "$SANDBOX/err")"
    else
        pass "$what: refused, an earlier run's page removed"
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
refused "a hash with no check binding it to a release" \
    '{"version": "1.2.3", "apk_sha256": "ab", "notices": "n"}' 'apk_sha256 comes with no check:apk_version'
refused "a hash made for another release" \
    '{"version": "1.2.3", "apk_sha256": "ab", "check:apk_version": "1.2.2", "notices": "n"}' "made for '1.2.2'"
refused "an unknown check" \
    '{"version": "1.2.3", "apk_sha256": "ab", "check:apk_version": "1.2.3", "check:apk_versoin": "1.2.3", "notices": "n"}' 'unknown check check:apk_versoin'
refused "a version with a leading zero" \
    '{"version": "1.02.3", "apk_sha256": "ab", "notices": "n"}' "'1.02.3'"
refused "a version with build metadata" \
    '{"version": "1.2.3+b.1", "apk_sha256": "ab", "notices": "n"}' "'1.2.3+b.1'"
printf '{"version": "1.2.3-rc.1", "apk_sha256": "ab", "check:apk_version": "1.2.3-rc.1", "notices": "n"}' > "$SANDBOX/v.json"
run "$T" "$SANDBOX/out.md" "$SANDBOX/v.json" && grep -qF '# InterWeave 1.2.3-rc.1' "$SANDBOX/out.md" \
    && pass "a pre-release version fills the page" \
    || fail "a pre-release version was refused" "$(cat "$SANDBOX/err")"

# One grammar with release_tag.sh, which the tag job applies first: over a
# table of versions, the page refuses exactly those release_tag.sh refuses
# as a shape (its other refusal, not the workspace's, is not the page's).
disagree=""
for v in 1.2.3 0.0.0 10.20.30 1.2.3-rc.1 1.0.0-alpha.beta 1.0.0-0.3.7 1.0.0-x-y 1.0.0-x.7.z.92 \
    1.0.0--- 1.0.0-0a 1.2 1.2.3.4 01.2.3 1.02.3 1.2.03 1.2.3- 1.2.3-01 1.2.3-rc..1 1.2.3-rc.1. \
    1.2.3+b 1.2.3-rc+b v1.2.3 ' 1.2.3' 1.2.3-r_c 1.2.3-é; do
    tag_shape=ok
    bash "$SCRIPT_DIR/release_tag.sh" "v$v" --manifest >"$SANDBOX/tag-err" 2>&1 \
        || ! grep -qF 'is not v<MAJOR>' "$SANDBOX/tag-err" || tag_shape=refused
    "$PY" - "$UNDER_TEST" "$v" >"$SANDBOX/page-shape" <<'PY'
import importlib.util, sys
spec = importlib.util.spec_from_file_location("release_page", sys.argv[1])
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
print("ok" if m.VERSION.fullmatch(sys.argv[2]) else "refused")
PY
    [[ "$(cat "$SANDBOX/page-shape")" == "$tag_shape" ]] || disagree+=" '$v'(tag:$tag_shape)"
done
[[ -z "$disagree" ]] && pass "the page's version grammar is release_tag.sh's over 26 versions" \
    || fail "the page and release_tag.sh disagree on" "$disagree"

printf '[]' > "$SANDBOX/v.json"
if run "$T" "$SANDBOX/out.md" "$SANDBOX/v.json"; then
    fail "a JSON array was accepted as the values"
else
    pass "values that are not one JSON object are refused"
fi

# The release run's values come in several files: merged, and a key
# two of them give is refused.
# The notices file's link and hash are bound like the APK's hash.
cat > "$SANDBOX/cn.md.in" <<'EOF'
# InterWeave {version}

{crate_notices}

SHA-256: `{crate_notices_sha256}`
EOF
for case in \
    'no check|{"version": "1.2.3", "crate_notices": "[f](u)", "crate_notices_sha256": "ab"}|crate_notices comes with no check:crate_notices_version' \
    'another release|{"version": "1.2.3", "crate_notices": "[f](u)", "crate_notices_sha256": "ab", "check:crate_notices_version": "1.2.2"}|made for '"'"'1.2.2'"'"''; do
    IFS='|' read -r what json needle <<<"$case"
    printf '%s' "$json" > "$SANDBOX/cv.json"
    if run "$SANDBOX/cn.md.in" "$SANDBOX/out.md" "$SANDBOX/cv.json"; then
        fail "the notices file's link and hash with $what: accepted"
    elif grep -qF -- "$needle" "$SANDBOX/err"; then
        pass "the notices file's link and hash with $what: refused"
    else
        fail "the notices file's link and hash with $what: refused without saying $needle" "$(cat "$SANDBOX/err")"
    fi
done
printf '{"version": "1.2.3", "crate_notices": "[f](u)", "crate_notices_sha256": "ab", "check:crate_notices_version": "1.2.3"}' > "$SANDBOX/cv.json"
run "$SANDBOX/cn.md.in" "$SANDBOX/out.md" "$SANDBOX/cv.json" && grep -qx 'SHA-256: `ab`' "$SANDBOX/out.md" \
    && pass "the notices file's link and hash bound to this release are filled" \
    || fail "the bound notices link and hash were refused" "$(cat "$SANDBOX/err")"

printf '{"version": "1.2.3"}' > "$SANDBOX/v1.json"
printf '{"apk_sha256": "ab12", "check:apk_version": "1.2.3", "notices": "n"}' > "$SANDBOX/v2.json"
if run "$T" "$SANDBOX/out.md" "$SANDBOX/v1.json" "$SANDBOX/v2.json" && grep -qx '# InterWeave 1.2.3' "$SANDBOX/out.md"; then
    pass "values from several files are merged"
else
    fail "values in several files were not merged" "$(cat "$SANDBOX/err")"
fi
printf '{"version": "1.2.4"}' > "$SANDBOX/v3.json"
printf 'an earlier run\n' > "$SANDBOX/out.md"
if run "$T" "$SANDBOX/out.md" "$SANDBOX/v1.json" "$SANDBOX/v2.json" "$SANDBOX/v3.json"; then
    fail "a key two files give was accepted"
elif [[ ! -e "$SANDBOX/out.md" ]] && grep -qF 'version is given by both' "$SANDBOX/err"; then
    pass "a key two files give is refused, an earlier run's page removed"
else
    fail "a key two files give was refused without saying so, or left a page" "$(cat "$SANDBOX/err")"
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
            printf ',"check:apk_version":"0.1.0","check:linux_version":"0.1.0","check:crate_notices_version":"0.1.0"}'
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
