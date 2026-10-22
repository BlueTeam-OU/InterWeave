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
    "bare 1.0.0 (MIT) publishes no licence file and notice_derived.json does not list it" "$SANDBOX/bare.txt" app
metadata broken
{ cat "$SANDBOX/compiled.txt"; echo "nolicence v1.0.0"; } > "$SANDBOX/nolicence.txt"
refused "a compiled crate with no licence expression and no licence file" \
    "nolicence 1.0.0 has no licence expression and no licence file" "$SANDBOX/nolicence.txt" app

# --- Derived notices (ADR-0058 decision 2, A 2026-10-22) ------------------
# Six crates that publish no licence file, one per case and edge. The
# script and its pinned texts are copied into the sandbox, so a text can
# be tampered with there.
D="$SANDBOX/derived"
mkdir -p "$D/tool"
cp "$UNDER_TEST" "$D/tool/"
cp -r "$SCRIPT_DIR/licence_texts" "$D/tool/"
src() {  # src <crate> <path> <text>: a source file of a sandbox crate.
    mkdir -p "$(dirname "$D/crates/$1/$2")"
    printf '%s\n' "$3" > "$D/crates/$1/$2"
    printf '[package]\n' > "$D/crates/$1/Cargo.toml"
}
MITHDR='// Copyright 2017 Example Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software, to deal in the Software without restriction.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND.

pub fn f() {}'
src hdr src/lib.rs "$MITHDR"
src hdrtext src/lib.rs '// Copyright 2022 The Example Authors. All rights reserved.
// Licensed under the MIT license (found in the LICENSE-MIT file).

pub fn f() {}'
src std src/lib.rs 'pub fn f() {}'
src spdxonly src/lib.rs '// SPDX-License-Identifier: CC0-1.0
pub fn f() {}'
src deep src/lib.rs 'mod inner;'
src deep src/inner/mod.rs "$MITHDR"
src docc src/lib.rs '//! Copyright 2020 Doc Writer: a doc comment, not a header.
pub fn f() {}'
python3 - "$D" > "$D/metadata.json" <<'PY'
import json, sys
root = sys.argv[1] + "/crates"
def pkg(name, licence, authors=()):
    return {"id": f"{name}-id", "name": name, "version": "1.0.0", "license": licence,
            "license_file": None, "authors": list(authors),
            "manifest_path": f"{root}/{name}/Cargo.toml",
            "targets": [{"kind": ["lib"], "src_path": f"{root}/{name}/src/lib.rs"}]}
derived = [pkg("hdr", "MIT"), pkg("hdrtext", "MIT OR Apache-2.0"),
           pkg("std", "MIT", ["Ann Author <ann@example.invalid>"]),
           pkg("spdxonly", "CC0-1.0"), pkg("deep", "MIT"), pkg("docc", "BSD-3-Clause")]
app = {"id": "app-id", "name": "app", "version": "1.0.0", "license": "Apache-2.0",
       "license_file": None, "manifest_path": f"{root}/app/Cargo.toml", "targets": []}
normal = [{"kind": None, "target": None}]
nodes = [{"id": "app-id", "deps": [{"name": p["name"], "pkg": p["id"], "dep_kinds": normal} for p in derived]}]
nodes += [{"id": p["id"], "deps": []} for p in derived]
print(json.dumps({"packages": [app] + derived, "workspace_members": ["app-id"], "resolve": {"nodes": nodes}}))
PY
printf 'app v1.0.0\nhdr v1.0.0\nhdrtext v1.0.0\nstd v1.0.0\nspdxonly v1.0.0\ndeep v1.0.0\ndocc v1.0.0\n' > "$D/compiled.txt"
# list <json lines>: the derived list, one crate object per argument.
list() {
    local IFS=,
    printf '{"crates": [%s]}\n' "$*" > "$D/list.json"
}
entry() {  # entry <name> <licence> <case> [file]
    if [[ -n "${4-}" ]]; then
        printf '{"name": "%s", "version": "1.0.0", "license": "%s", "case": "%s", "file": "%s"}' "$1" "$2" "$3" "$4"
    else
        printf '{"name": "%s", "version": "1.0.0", "license": "%s", "case": "%s"}' "$1" "$2" "$3"
    fi
}
GOOD=("$(entry hdr MIT header src/lib.rs)" "$(entry hdrtext 'MIT OR Apache-2.0' header+text src/lib.rs)"
      "$(entry std MIT standard)" "$(entry spdxonly CC0-1.0 standard)"
      "$(entry deep MIT header src/inner/mod.rs)" "$(entry docc BSD-3-Clause standard)")
drun() {
    NOTICE_DERIVED_JSON="$D/list.json" python3 "$D/tool/release_notices.py" "$D/metadata.json" \
        "$D/compiled.txt" "$SANDBOX/n.md" app >"$SANDBOX/stdout" 2>"$SANDBOX/err"
}
list "${GOOD[@]}"
if drun; then
    N="$SANDBOX/n.md"
    grep -qF -- '- `hdr` 1.0.0: MIT (derived: header) — text T' "$N" \
        && grep -qx 'Permission is hereby granted, free of charge, to any person obtaining a' "$N" \
        && pass "a full notice in the root module's header ships as the header" \
        || fail "the header notice is missing" "$(cat "$N")"
    grep -qF 'Used by deep 1.0.0 (header of src/inner/mod.rs); hdr 1.0.0 (header of src/lib.rs).' "$N" \
        && pass "a header found in another source file is the same notice, named by its file" \
        || fail "the header of a nested source file was not found" "$(grep -n 'Used by' "$N")"
    grep -qF -- '- `hdrtext` 1.0.0: MIT OR Apache-2.0 (derived: header+text) — text T' "$N" \
        && grep -qx 'Licensed under the MIT license (found in the LICENSE-MIT file).' "$N" \
        && grep -qF 'Used by hdrtext 1.0.0 (standard MIT text)' "$N" \
        && ! grep -qF '<copyright holders>' "$N" \
        && pass "a header naming its licence ships with the first term's standard text, no placeholder line" \
        || fail "header+text did not ship the header and the MIT text" "$(cat "$N")"
    grep -qF 'std 1.0.0 publishes no notice of its own: this is the standard text of MIT' "$N" \
        && grep -qx 'Copyright (c) Ann Author <ann@example.invalid>' "$N" \
        && ! grep -qF '<copyright holders>' "$N" \
        && pass "a crate with no header gets the standard text under its authors" \
        || fail "the standard text is not under the crate's authors" "$(cat "$N")"
    grep -qF 'spdxonly 1.0.0 publishes no notice of its own: this is the standard text of CC0-1.0' "$N" \
        && grep -qx 'Copyright (c) the spdxonly authors' "$N" \
        && pass "an SPDX tag alone is no notice: the standard text, under the crate's name" \
        || fail "an SPDX-only crate was not given the standard text" "$(cat "$N")"
    grep -qF 'docc 1.0.0 publishes no notice of its own: this is the standard text of BSD-3-Clause' "$N" \
        && grep -qx 'Copyright (c) the docc authors. ' "$N" \
        && pass "a doc comment is not a header" \
        || fail "a doc comment was taken for a header" "$(cat "$N")"
else
    fail "the derived list was refused" "$(cat "$SANDBOX/err")"
fi

drefused() {
    local what="$1" needle="$2"
    printf 'an earlier run\n' > "$SANDBOX/n.md"
    if drun; then fail "$what: accepted"
    elif [[ -e "$SANDBOX/n.md" ]]; then fail "$what: refused but left notices"
    elif ! grep -qF -- "$needle" "$SANDBOX/err"; then fail "$what: refused without saying $needle" "$(cat "$SANDBOX/err")"
    else pass "$what: refused"; fi
}
# Each beside the passing list above: one entry changed.
list "${GOOD[@]:1}"
drefused "a file-less crate the list does not name" "hdr 1.0.0 (MIT) publishes no licence file and notice_derived.json does not list it"
list "$(entry hdr 'MIT OR Apache-2.0' header src/lib.rs)" "${GOOD[@]:1}"
drefused "an entry whose licence is not the crate's" "hdr 1.0.0 declares 'MIT'; notice_derived.json lists 'MIT OR Apache-2.0'"
list "$(entry hdr MIT header src/main.rs)" "${GOOD[@]:1}"
drefused "a header entry naming another file" "hdr 1.0.0's header is in src/lib.rs; notice_derived.json names src/main.rs"
list "$(entry hdr MIT standard)" "${GOOD[@]:1}"
drefused "a standard entry for a crate with a header" "hdr 1.0.0 has a copyright header in src/lib.rs: list it as header or header+text"
list "${GOOD[@]:0:3}" "$(entry spdxonly CC0-1.0 header src/lib.rs)" "${GOOD[@]:4}"
drefused "a header entry for an SPDX tag alone" "spdxonly 1.0.0 is listed as header but has no copyright header"
list "${GOOD[@]}" "$(entry hdr MIT header src/lib.rs)"
drefused "a crate listed twice" "lists hdr 1.0.0 twice"
list "$(entry hdr MIT headers src/lib.rs)" "${GOOD[@]:1}"
drefused "an unknown case" "has case 'headers'"
list "$(entry hdr MIT header)" "${GOOD[@]:1}"
drefused "a header entry without its file" "names a file exactly when its case is a header"
# A file-less crate now publishing a file: its entry must go.
printf 'the MIT text\n' > "$D/crates/hdr/LICENSE"
list "${GOOD[@]}"
drefused "an entry for a crate that publishes a licence file now" "hdr 1.0.0 publishes a licence file now: remove its entry"
rm "$D/crates/hdr/LICENSE"
# A standard text that is not its pin; restored after.
cp "$D/tool/licence_texts/MIT.txt" "$D/MIT.keep"
printf 'tampered\n' >> "$D/tool/licence_texts/MIT.txt"
drefused "a standard text changed from its pin" "licence_texts/MIT.txt is not the pinned SPDX text"
cp "$D/MIT.keep" "$D/tool/licence_texts/MIT.txt"
drun && pass "the restored text is accepted again" || fail "the restored text was refused" "$(cat "$SANDBOX/err")"
# A licence with no committed standard text, and one with no first term.
python3 - "$D/metadata.json" <<'PY'
import json, sys
m = json.load(open(sys.argv[1]))
next(p for p in m["packages"] if p["name"] == "std")["license"] = "ISC"
json.dump(m, open(sys.argv[1], "w"))
PY
list "${GOOD[@]:0:2}" "$(entry std ISC standard)" "${GOOD[@]:3}"
drefused "a licence whose standard text is not committed" "no standard text of ISC is committed"
python3 - "$D/metadata.json" <<'PY'
import json, sys
m = json.load(open(sys.argv[1]))
next(p for p in m["packages"] if p["name"] == "std")["license"] = "(MIT AND ISC) OR Apache-2.0"
json.dump(m, open(sys.argv[1], "w"))
PY
list "${GOOD[@]:0:2}" "$(entry std '(MIT AND ISC) OR Apache-2.0' standard)" "${GOOD[@]:3}"
drefused "an expression with no single first term" "has no single first term"

# --- --json KEY: the notices as a section of the release page ------------
metadata
printf '# a line of a licence text that starts with a hash\n' >> "$SANDBOX/crates/reuse/LICENSES/Apache-2.0.txt"
if python3 "$UNDER_TEST" --json crate_notices "$SANDBOX/metadata.json" "$SANDBOX/compiled.txt" \
    "$SANDBOX/n.json" app >/dev/null 2>"$SANDBOX/err"; then
    python3 - "$SANDBOX/n.json" > "$SANDBOX/section.md" <<'PY' || fail "--json did not write one key" "$(cat "$SANDBOX/n.json")"
import json, sys
d = json.load(open(sys.argv[1]))
assert list(d) == ["crate_notices"], d
sys.stdout.write(d["crate_notices"])
PY
    S="$SANDBOX/section.md"
    ! grep -q '^# Third-party notices' "$S" && grep -qx '### Licence texts' "$S" && grep -qx '#### T1' "$S" \
        && grep -qF -- '- `shared` 1.0.0: MIT — text T' "$S" \
        && pass "--json writes the notices under its key, the title dropped and every heading a level down" \
        || fail "--json's section is not the page one level down" "$(cat "$S")"
    grep -qx '# a line of a licence text that starts with a hash' "$S" \
        && pass "--json leaves a line inside a fenced licence text as it is" \
        || fail "--json changed a line inside a licence text" "$(grep -n 'starts with a hash' "$S")"
else
    fail "--json refused a consistent run" "$(cat "$SANDBOX/err")"
fi
printf 'an earlier run\n' > "$SANDBOX/n.json"
python3 "$UNDER_TEST" --json crate_notices "$SANDBOX/metadata.json" "$SANDBOX/compiled.txt" \
    "$SANDBOX/n.json" nobody >/dev/null 2>&1
[[ $? -eq 1 && ! -e "$SANDBOX/n.json" ]] \
    && pass "--json's refusal removes an earlier run's output" \
    || fail "--json's refusal left an output or did not exit 1"
python3 "$UNDER_TEST" --json >/dev/null 2>&1
[[ $? -eq 2 ]] && pass "--json without its key is a usage error (2)" || fail "--json without its key did not exit 2"

python3 "$UNDER_TEST" "$SANDBOX/metadata.json" >/dev/null 2>&1
[[ $? -eq 2 ]] && pass "a missing argument is a usage error (2)" || fail "a missing argument did not exit 2"

if (( failures )); then
    echo "test_release_notices.sh: $failures failure(s)" >&2
    exit 1
fi
echo "test_release_notices.sh: all assertions passed"
