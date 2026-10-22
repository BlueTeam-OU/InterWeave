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
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR
// ANY CLAIM, DAMAGES OR OTHER LIABILITY ARISING FROM THE SOFTWARE.

pub fn f() {}'
src hdr src/lib.rs "$MITHDR"
src hdrtext src/lib.rs '// Copyright 2022 The Example Authors.
// Licensed under the MIT license (found in the LICENSE-MIT file).

pub fn f() {}'
src std src/lib.rs 'pub fn f() {}'
src spdxonly src/lib.rs '// SPDX-License-Identifier: CC0-1.0
pub fn f() {}'
src deep src/lib.rs 'mod inner;'
src deep src/inner/mod.rs "$MITHDR"
src docc src/lib.rs '//! Copyright 2020 Doc Writer: a doc comment, not a header.
pub fn f() {}'
src prose src/lib.rs '// This module preserves copyright notices when it rewrites files.
pub fn f() {}'
# A header-free crate whose example carries one: examples/ is not
# compiled in, so it stays standard.
src std examples/demo.rs "$MITHDR"
# Shaped like libp2p-core: the root module's header, a second holder's
# in a file that sorts before it, and one in tests/ that is not compiled.
src core src/lib.rs "$MITHDR"
src core src/a_first.rs '// Copyright 2023 Second Holder Labs.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR
// ANY CLAIM, DAMAGES OR OTHER LIABILITY ARISING FROM THE SOFTWARE.
pub fn g() {}'
src core tests/smoke.rs '// Copyright 2019 Test Writer.
fn main() {}'
# A header with the grant and AS IS but not the notice condition or the
# liability clause is not the whole MIT notice (it ships as header+text).
# Two near-whole MIT notices, each missing one clause: the notice
# condition, then the liability clause.
src trunc src/lib.rs "$(printf '%s\n' "$MITHDR" | grep -v 'The above copyright notice\|all copies or substantial')"
src trunc2 src/lib.rs "$(printf '%s\n' "$MITHDR" | sed 's/ IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR//; /ANY CLAIM, DAMAGES/d')"
# One header naming the crate's own licence and a foreign one (as the
# Chromium-derived accesskit header names MIT and BSD): the BSD text is
# owed even though MIT is answered.
src mixed src/lib.rs "$MITHDR"
src mixed src/m.rs '// Copyright 2019 Mixed Holder. Licensed under the MIT license.
// Derived from Browser, Copyright 2018 The Browser Authors, under a
// BSD-style license found in the LICENSE.browser file.
pub fn m() {}'
# Apache-first, with the whole MIT notice in its header: MIT satisfies
# the OR (decision 4), so it is a header case.
src apfirst src/lib.rs "$MITHDR"
# MIT OR Apache-2.0, a whole MIT root header, and a second holder's
# header naming Apache alone: the Apache text is owed.
src apx src/lib.rs "$MITHDR"
src apx src/other.rs '// Copyright 2021 Apache Holder.
// Licensed under the Apache License, Version 2.0 (found in the LICENSE-APACHE file).
pub fn k() {}'
# A REUSE header: SPDX-FileCopyrightText is a copyright line.
# (The tag is split so this file declares no licence of its own.)
src reusehdr src/lib.rs "$(printf '%s\n%s\n%s' '// SPDX-FileCopyrightText: 2024 Jane Doe' \
    '// SPDX-License-''Identifier: MIT' 'pub fn f() {}')"
# An MIT crate with a second holder's BSD-style header that only names
# its licence: the BSD text is owed beside it.
src zbsd src/lib.rs "$MITHDR"
src zbsd src/z.rs '// Copyright 2018 The Browser Authors.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE.browser file.
pub fn h() {}'
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
           pkg("spdxonly", "CC0-1.0"), pkg("deep", "MIT"), pkg("docc", "BSD-3-Clause"),
           pkg("prose", "MIT"), pkg("core", "MIT"), pkg("zbsd", "MIT"),
           pkg("trunc", "MIT"), pkg("apfirst", "Apache-2.0 OR MIT"),
           pkg("apx", "MIT OR Apache-2.0"), pkg("reusehdr", "MIT"),
           pkg("trunc2", "MIT"), pkg("mixed", "MIT")]
app = {"id": "app-id", "name": "app", "version": "1.0.0", "license": "Apache-2.0",
       "license_file": None, "manifest_path": f"{root}/app/Cargo.toml", "targets": []}
normal = [{"kind": None, "target": None}]
nodes = [{"id": "app-id", "deps": [{"name": p["name"], "pkg": p["id"], "dep_kinds": normal} for p in derived]}]
nodes += [{"id": p["id"], "deps": []} for p in derived]
print(json.dumps({"packages": [app] + derived, "workspace_members": ["app-id"], "resolve": {"nodes": nodes}}))
PY
printf 'app v1.0.0\nhdr v1.0.0\nhdrtext v1.0.0\nstd v1.0.0\nspdxonly v1.0.0\ndeep v1.0.0\ndocc v1.0.0\nprose v1.0.0\ncore v1.0.0\nzbsd v1.0.0\ntrunc v1.0.0\napfirst v1.0.0\napx v1.0.0\nreusehdr v1.0.0\ntrunc2 v1.0.0\nmixed v1.0.0\n' > "$D/compiled.txt"
# list <json lines>: the derived list, one crate object per argument.
list() {
    local IFS=,
    printf '{"crates": [%s]}\n' "$*" > "$D/list.json"
}
entry() {  # entry <name> <licence> <case> [file [headers [extra-texts-json]]]
    if [[ -n "${4-}" ]]; then
        printf '{"name": "%s", "version": "1.0.0", "license": "%s", "case": "%s", "file": "%s", "headers": %s%s}' \
            "$1" "$2" "$3" "$4" "${5:-1}" "${6:+, \"extra_texts\": $6}"
    else
        printf '{"name": "%s", "version": "1.0.0", "license": "%s", "case": "%s"}' "$1" "$2" "$3"
    fi
}
GOOD=("$(entry hdr MIT header src/lib.rs)" "$(entry hdrtext 'MIT OR Apache-2.0' header+text src/lib.rs)"
      "$(entry std MIT standard)" "$(entry spdxonly CC0-1.0 standard)"
      "$(entry deep MIT header src/inner/mod.rs)" "$(entry docc BSD-3-Clause standard)"
      "$(entry prose MIT standard)" "$(entry core MIT header src/lib.rs 2)"
      "$(entry zbsd MIT header src/lib.rs 2 '["BSD-3-Clause"]')"
      "$(entry trunc MIT header+text src/lib.rs)" "$(entry apfirst 'Apache-2.0 OR MIT' header src/lib.rs)"
      "$(entry apx 'MIT OR Apache-2.0' header src/lib.rs 2 '["Apache-2.0"]')"
      "$(entry reusehdr MIT header+text src/lib.rs)" "$(entry trunc2 MIT header+text src/lib.rs)"
      "$(entry mixed MIT header src/lib.rs 2 '["BSD-3-Clause"]')")
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
    grep -F 'Used by ' "$N" | grep -F 'deep 1.0.0 (header of src/inner/mod.rs)' | grep -qF 'hdr 1.0.0 (header of src/lib.rs)' \
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
    grep -qF 'Used by core 1.0.0 (header of src/a_first.rs).' "$N" \
        && [[ "$(sed -n 's/^- `core` 1.0.0: MIT (derived: header) — text \(T[0-9]*\),.*/\1/p' "$N")" == \
              "$(awk '/^### T/{l=$2} /^Copyright 2017 Example Ltd\.$/{print l; exit}' "$N")" ]] \
        && [[ "$(grep -n 'Copyright 2017 Example Ltd.' "$N" | head -1 | cut -d: -f1)" -lt \
              "$(grep -n 'Copyright 2023 Second Holder Labs.' "$N" | head -1 | cut -d: -f1)" ]] \
        && ! grep -qF 'Test Writer' "$N" \
        && pass "every distinct header of the compiled files ships, the root module's first; tests/ is not read" \
        || fail "a crate's several headers did not ship as decision 2.ii says" "$(grep -n 'core\|Holder\|Test Writer' "$N")"
    grep -qF 'Used by zbsd 1.0.0 (header of src/z.rs).' "$N" \
        && grep -F 'Used by ' "$N" | grep -qF 'zbsd 1.0.0 (standard BSD-3-Clause text)' \
        && ! grep -qF '<year> <owner>' "$N" \
        && pass "a second holder's header naming BSD ships with the BSD text, no placeholder line" \
        || fail "the BSD-style header did not bring its text" "$(grep -n 'zbsd' "$N")"
    grep -qF 'std 1.0.0 publishes no notice of its own' "$N" \
        && pass "a header under examples/ is not compiled in and leaves the crate standard" \
        || fail "an example's header changed the crate's case" "$(grep -n 'std 1.0.0' "$N")"
    grep -qF -- '- `apfirst` 1.0.0: Apache-2.0 OR MIT (derived: header)' "$N" \
        && ! grep -qF 'apfirst 1.0.0 (standard' "$N" \
        && pass "an OR is satisfied by the term whose whole text a header carries, first or not" \
        || fail "an Apache-first crate with a whole MIT header was not a header case" "$(grep -n apfirst "$N")"
    grep -qF 'Used by apx 1.0.0 (header of src/other.rs).' "$N" \
        && grep -F 'Used by ' "$N" | grep -qF 'apx 1.0.0 (standard Apache-2.0 text)' \
        && pass "a second holder's header naming the crate's other term brings that term's text" \
        || fail "the second holder's Apache text did not ship" "$(grep -n apx "$N")"
    grep -qF -- '- `trunc` 1.0.0: MIT (derived: header+text)' "$N" \
        && grep -qF -- '- `reusehdr` 1.0.0: MIT (derived: header+text)' "$N" \
        && grep -qx 'SPDX-FileCopyrightText: 2024 Jane Doe' "$N" \
        && pass "a truncated MIT notice and a REUSE header each ship with the MIT text" \
        || fail "a truncated or REUSE header was not shipped with its text" "$(grep -n 'trunc\|reusehdr\|Jane' "$N")"
    grep -qF 'prose 1.0.0 publishes no notice of its own: this is the standard text of MIT' "$N" \
        && pass "a comment that mentions copyright without a copyright line is not a header" \
        || fail "a prose comment was taken for a header" "$(cat "$N")"
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
# The two header cases, checked against what the header carries.
list "$(entry hdr MIT header+text src/lib.rs)" "${GOOD[@]:1}"
drefused "header+text for a header carrying its licence's text" "hdr 1.0.0's headers carry the MIT text already: list it as header"
list "${GOOD[@]:0:1}" "$(entry hdrtext 'MIT OR Apache-2.0' header src/lib.rs)" "${GOOD[@]:2}"
drefused "header for a header that only names its licence" "hdrtext 1.0.0's headers carry no whole text of MIT OR Apache-2.0: list it as header+text"
list "${GOOD[@]:0:6}" "$(entry prose MIT header src/lib.rs)" "${GOOD[@]:7}"
drefused "a header entry for a comment with no copyright line" "prose 1.0.0 is listed as header but has no copyright header"
list "$(entry hdr MIT headers src/lib.rs)" "${GOOD[@]:1}"
drefused "an unknown case" "has case 'headers'"
list "$(entry hdr MIT header)" "${GOOD[@]:1}"
drefused "a header entry without its file" "names a file and a header count exactly when its case is a header"
list "${GOOD[@]:0:7}" "$(entry core MIT header src/lib.rs 1)" "${GOOD[@]:8}"
drefused "a header count that is not the crate's" "core 1.0.0 has 2 distinct copyright header(s); notice_derived.json says 1"
list "${GOOD[@]:0:7}" "$(entry core MIT header src/a_first.rs 2)" "${GOOD[@]:8}"
drefused "a header file that is not the root module's" "core 1.0.0's header is in src/lib.rs; notice_derived.json names src/a_first.rs"
list "${GOOD[@]:0:8}" "$(entry zbsd MIT header src/lib.rs 2)" "${GOOD[@]:9}"
drefused "a header naming BSD without the BSD text" "zbsd 1.0.0's header in src/z.rs names BSD-3-Clause, whose text it does not carry: add it to the entry's extra_texts"
list "${GOOD[@]:0:8}" "$(entry zbsd MIT header src/lib.rs 2 '["ISC"]')" "${GOOD[@]:9}"
drefused "an extra text that is not committed" "zbsd 1.0.0 asks for ISC, whose standard text is not committed"
list "${GOOD[@]:0:9}" "$(entry trunc MIT header src/lib.rs)" "${GOOD[@]:10}"
drefused "a truncated MIT notice listed as header" "trunc 1.0.0's headers carry no whole text of MIT: list it as header+text"
list "${GOOD[@]:0:10}" "$(entry apfirst 'Apache-2.0 OR MIT' header+text src/lib.rs)" "${GOOD[@]:11}"
drefused "header+text for a header carrying one OR term's whole text" "apfirst 1.0.0's headers carry the MIT text already: list it as header"
list "${GOOD[@]:0:11}" "$(entry apx 'MIT OR Apache-2.0' header src/lib.rs 2)" "${GOOD[@]:12}"
drefused "a second holder's header naming the crate's other term, without its text" "apx 1.0.0's header in src/other.rs names Apache-2.0, whose text it does not carry"
list "${GOOD[@]:0:12}" "$(entry reusehdr MIT standard)" "${GOOD[@]:13}"
drefused "a REUSE header listed as standard" "reusehdr 1.0.0 has a copyright header in src/lib.rs"
list "${GOOD[@]:0:2}" '{"name": "std", "version": "1.0.0", "license": "MIT", "case": "standard", "extra_texts": ["BSD-3-Clause"]}' "${GOOD[@]:3}"
drefused "extra_texts on a standard entry" "std 1.0.0 has extra_texts, which only a header case ships"
list "${GOOD[@]:0:13}" "$(entry trunc2 MIT header src/lib.rs)" "${GOOD[@]:14}"
drefused "an MIT notice without its liability clause listed as header" "trunc2 1.0.0's headers carry no whole text of MIT: list it as header+text"
list "${GOOD[@]:0:14}" "$(entry mixed MIT header src/lib.rs 2)"
drefused "a header naming its crate's licence and a foreign one, without the foreign text" "mixed 1.0.0's header in src/m.rs names BSD-3-Clause, whose text it does not carry"
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
drefused "an expression with an AND inside its first term" "is not a plain OR of single terms"
python3 - "$D/metadata.json" <<'PY'
import json, sys
m = json.load(open(sys.argv[1]))
next(p for p in m["packages"] if p["name"] == "std")["license"] = "(MIT OR Apache-2.0) AND Unicode-3.0"
json.dump(m, open(sys.argv[1], "w"))
PY
list "${GOOD[@]:0:2}" "$(entry std '(MIT OR Apache-2.0) AND Unicode-3.0' standard)" "${GOOD[@]:3}"
drefused "an expression owing an AND term beside its OR" "is not a plain OR of single terms"

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
    ! grep -q '^# Third-party notices' "$S" && ! grep -q 'These programs' "$S" \
        && [[ "$(head -c 3 "$S")" == '- `' ]] && grep -qx '### Licence texts' "$S" && grep -qx '#### T1' "$S" \
        && grep -qF -- '- `shared` 1.0.0: MIT — text T' "$S" \
        && pass "--json writes the notices under its key from the list on, its title and opening paragraph dropped, every heading a level down" \
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
