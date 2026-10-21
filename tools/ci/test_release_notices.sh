#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_release_notices.sh
#
# Self-test for release_notices.py.
#
# A small `cargo metadata` document over sandbox crates, and the `cargo
# tree` list of what the build compiles. The metadata's resolve reaches
# more than the build compiles -- a crate only the unified features reach,
# a build-dependency, a dev-dependency -- and the notices must list the
# compiled ones alone. The complete fill is the positive control for the
# refusals. release_linux_archive.sh's self-test runs it inside the
# archive too.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/release_notices.py"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(realpath -- "$(mktemp -d)")"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_release_notices: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# crate <dir> <file>...: a crate directory holding those files.
crate() {
    local d="$SANDBOX/crates/$1"; shift
    mkdir -p "$d"
    printf '[package]\n' > "$d/Cargo.toml"
    local f
    for f in "$@"; do
        mkdir -p "$(dirname "$d/$f")"
        printf 'the text of %s\n' "$(basename "$f")" > "$d/$f"
    done
}
crate app
crate shared LICENSE-MIT
crate other LICENSE-MIT
cp "$SANDBOX/crates/shared/LICENSE-MIT" "$SANDBOX/crates/other/LICENSE-MIT"
crate reuse LICENSES/Apache-2.0.txt
crate bare
crate unified LICENSE
crate buildonly LICENSE
crate nolicence

metadata() {
    python3 - "$SANDBOX" "${1-}" > "$SANDBOX/metadata.json" <<'PY'
import json, sys
root, broken = sys.argv[1] + "/crates", sys.argv[2]
def pkg(name, licence, repo=None):
    return {"id": f"{name}-id", "name": name, "version": "1.0.0", "license": licence,
            "license_file": None, "repository": repo,
            "manifest_path": f"{root}/{name}/Cargo.toml"}
packages = [pkg("app", "Apache-2.0"), pkg("shared", "MIT"), pkg("other", "MIT"),
            pkg("reuse", "Apache-2.0"), pkg("bare", "MIT", "https://example.invalid/bare"),
            pkg("unified", "MIT"), pkg("buildonly", "MIT"),
            pkg("nolicence", None if broken else "MIT")]
normal = [{"kind": None, "target": None}]
deps = [{"name": n, "pkg": f"{n}-id", "dep_kinds": normal}
        for n in ("shared", "other", "reuse", "bare", "unified", "nolicence")]
deps.append({"name": "buildonly", "pkg": "buildonly-id",
             "dep_kinds": [{"kind": "build", "target": None}]})
nodes = [{"id": "app-id", "deps": deps}] + [{"id": f"{p['name']}-id", "deps": []} for p in packages[1:]]
print(json.dumps({"packages": packages, "workspace_members": ["app-id"],
                  "resolve": {"nodes": nodes}}))
PY
}
cat > "$SANDBOX/compiled.txt" <<'EOF'
app v1.0.0 (/somewhere/app)
shared v1.0.0
other v1.0.0
reuse v1.0.0
shared v1.0.0 (*)
EOF
run() {
    python3 "$UNDER_TEST" "$SANDBOX/metadata.json" "$1" "$SANDBOX/n.md" "${@:2}" \
        >"$SANDBOX/stdout" 2>"$SANDBOX/err"
}

echo "release_notices.py:"
metadata
if run "$SANDBOX/compiled.txt" app; then
    N="$SANDBOX/n.md"
    lacking=""
    for line in '- `shared` 1.0.0: MIT — text T' '- `other` 1.0.0: MIT — text T' \
        '- `reuse` 1.0.0: Apache-2.0 — text T'; do
        grep -qF -- "$line" "$N" || { fail "the notices lack: $line"; lacking=1; }
    done
    [[ -n "$lacking" ]] || pass "every compiled crate is listed with its licence"
    if grep -qE '`(unified|buildonly|app)`' "$N"; then
        fail "a crate the build does not compile, or the workspace's own, is listed" "$(grep -E 'unified|buildonly|`app`' "$N")"
    else
        pass "a crate only the unified resolve reaches, a build-dependency and the workspace's own are not listed"
    fi
    [[ "$(grep -c '^the text of LICENSE-MIT$' "$N")" == 1 ]] \
        && grep -qF 'Used by other 1.0.0 (LICENSE-MIT); shared 1.0.0 (LICENSE-MIT).' "$N" \
        && pass "a text two crates share is printed once and names both" \
        || fail "a shared text is not printed once for both"
    grep -qx 'the text of Apache-2.0.txt' "$N" \
        && pass "a LICENSES/ directory's text is included" \
        || fail "a LICENSES/ directory's text is missing"
    grep -qx '3 crates, 2 distinct licence texts' "$SANDBOX/stdout" \
        && pass "the counts are printed" || fail "the counts are not as expected" "$(cat "$SANDBOX/stdout")"
else
    fail "a consistent metadata and compiled list were refused" "$(cat "$SANDBOX/err")"
fi

refused() {
    local what="$1" needle="$2"; shift 2
    # An earlier run's notices, which a refusal must remove.
    printf 'an earlier run\n' > "$SANDBOX/n.md"
    if run "$@"; then fail "$what: accepted"
    elif [[ -e "$SANDBOX/n.md" ]]; then fail "$what: refused but left notices"
    elif ! grep -qF -- "$needle" "$SANDBOX/err"; then fail "$what: refused without saying $needle" "$(cat "$SANDBOX/err")"
    else pass "$what: refused, an earlier run's notices removed"; fi
}
cp "$SANDBOX/compiled.txt" "$SANDBOX/stray.txt"
echo "buildonly v1.0.0" >> "$SANDBOX/stray.txt"
refused "a compiled crate the metadata's normal walk does not reach" \
    "compiled but not reached from the packages in the metadata: buildonly 1.0.0" "$SANDBOX/stray.txt" app
printf 'not a tree line\n' > "$SANDBOX/garbled.txt"
refused "a compiled list that is not cargo tree's" "not a cargo tree line" "$SANDBOX/garbled.txt" app
refused "a package that is not a workspace member" "not a workspace member: nobody" "$SANDBOX/compiled.txt" nobody
# A list cargo tree did not make for the named package: empty, or the same
# crates without the root's own line (the full list above is the control).
: > "$SANDBOX/empty.txt"
refused "an empty compiled list" "the compiled list does not name: app" "$SANDBOX/empty.txt" app
grep -v '^app ' "$SANDBOX/compiled.txt" > "$SANDBOX/rootless.txt"
refused "a compiled list without the named package's own line" \
    "the compiled list does not name: app" "$SANDBOX/rootless.txt" app
# A crate with a licence expression and no licence file: its notice is
# not in its files, so it is refused, not listed without it (the passing
# run above, with the same crate defined but not compiled, is the control).
{ cat "$SANDBOX/compiled.txt"; echo "bare v1.0.0"; } > "$SANDBOX/bare.txt"
refused "a compiled crate that publishes no licence file" \
    "bare 1.0.0 (MIT) publishes no licence file, so its notice cannot be shipped" "$SANDBOX/bare.txt" app
metadata broken
{ cat "$SANDBOX/compiled.txt"; echo "nolicence v1.0.0"; } > "$SANDBOX/nolicence.txt"
refused "a compiled crate with no licence expression and no licence file" \
    "nolicence 1.0.0 has no licence expression and no licence file" "$SANDBOX/nolicence.txt" app

python3 "$UNDER_TEST" "$SANDBOX/metadata.json" >/dev/null 2>&1
[[ $? -eq 2 ]] && pass "a missing argument is a usage error (2)" || fail "a missing argument did not exit 2"

if (( failures )); then
    echo "test_release_notices.sh: $failures failure(s)" >&2
    exit 1
fi
echo "test_release_notices.sh: all assertions passed"
