#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_release_crate_notices.sh
#
# Self-test for release_crate_notices.sh.
#
# cargo is a stub on PATH that records what it was asked and prints a
# small graph over sandbox crates, so nothing is resolved or compiled;
# release_notices.py is the tree's own, so the notices file is the one the
# release would carry and the values file the one its page would take.
# The overrides (RELEASE_METADATA_JSON, RELEASE_COMPILED_LIST) are
# exercised too, and every refusal must leave none of the three outputs,
# an earlier run's included.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/release_crate_notices.sh"
[[ -f "$UNDER_TEST" && -f "$SCRIPT_DIR/release_notices.py" ]] || { echo "test: scripts not found" >&2; exit 1; }

failures=0
SANDBOX="$(realpath -- "$(mktemp -d)")"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_release_crate_notices: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

crate() {
    local d="$SANDBOX/crates/$1"; shift
    mkdir -p "$d"
    printf '[package]\n' > "$d/Cargo.toml"
    local f
    for f in "$@"; do printf 'the text of %s\n' "$f" > "$d/$f"; done
}
crate app
crate libfoo LICENSE-MIT

M="$SANDBOX/metadata.json"
python3 - "$SANDBOX" > "$M" <<'PY'
import json, sys
root = sys.argv[1] + "/crates"
def pkg(name, dir, license):
    return {"id": f"{name}-id", "name": name, "version": "1.0.0", "license": license,
            "license_file": None, "repository": None, "manifest_path": f"{root}/{dir}/Cargo.toml"}
packages = [pkg("interweave-human-android", "app", "Apache-2.0"), pkg("libfoo", "libfoo", "MIT")]
nodes = [{"id": "interweave-human-android-id",
          "deps": [{"name": "libfoo", "pkg": "libfoo-id", "dep_kinds": [{"kind": None, "target": None}]}]},
         {"id": "libfoo-id", "deps": []}]
print(json.dumps({"packages": packages, "workspace_members": ["interweave-human-android-id"],
                  "resolve": {"nodes": nodes}}))
PY
C="$SANDBOX/compiled.txt"
printf 'interweave-human-android v1.0.0 (/somewhere/apps/human-android)\nlibfoo v1.0.0\n' > "$C"

STUB="$SANDBOX/bin"
mkdir -p "$STUB"
cat > "$STUB/cargo" <<EOF
#!/bin/sh
printf '%s\n' "\$*" >> "$SANDBOX/cargo-args"
case "\$1" in
metadata) cat "$M" ;;
tree) cat "\${STUB_TREE:-$C}" ;;
*) echo "cargo stand-in: \$*" >&2; exit 1 ;;
esac
EOF
chmod +x "$STUB/cargo"

D="$SANDBOX/out"
ASSET="$D/THIRD-PARTY-NOTICES-android.md"
J="$D/crate-notices.json"
URL="https://example.invalid/releases/download/v1.2.3"
run() { PATH="$STUB:$PATH" bash "$UNDER_TEST" "$@" >"$SANDBOX/stdout" 2>"$SANDBOX/err"; }
# plant: an earlier run's three outputs, which a refusal must remove.
plant() { mkdir -p "$D"; for f in "$ASSET" "$ASSET.sha256" "$J"; do echo stale > "$f"; done; }
none_left() { [[ ! -e "$ASSET" && ! -e "$ASSET.sha256" && ! -e "$J" ]]; }

echo "release_crate_notices.sh:"
: > "$SANDBOX/cargo-args"
if run aarch64-linux-android interweave-human-android 1.2.3 "$D" "$URL"; then
    (cd "$D" && sha256sum -c --quiet THIRD-PARTY-NOTICES-android.md.sha256) \
        && pass "the notices file and a .sha256 that checks against it are written" \
        || fail "the .sha256 does not check against the notices file" "$(ls -A "$D")"
    got="$(python3 - "$J" "$ASSET" <<'PY'
import hashlib, json, sys
v = json.load(open(sys.argv[1]))
sha = hashlib.sha256(open(sys.argv[2], "rb").read()).hexdigest()
print(sorted(v), v.get("crate_notices"), v.get("crate_notices_sha256") == sha, v.get("check:crate_notices_version"))
PY
)"
    want="['check:crate_notices_version', 'crate_notices', 'crate_notices_sha256'] $URL/THIRD-PARTY-NOTICES-android.md True 1.2.3"
    [[ "$got" == "$want" ]] \
        && pass "crate-notices.json: the file's URL, its SHA-256 and the version that binds them, and nothing else" \
        || fail "crate-notices.json is not the expected one" "$got"
    body="$(cat "$ASSET")"
    if [[ "$body" == *libfoo* && "$body" == *"the text of LICENSE-MIT"* && "$body" != *interweave-human-android* ]]; then
        pass "the notices file names the compiled crate with its text, and not the workspace's own package"
    else
        fail "the notices file is not the package's compiled crates" "$body"
    fi
    args="$(cat "$SANDBOX/cargo-args")"
    if grep -qxF 'metadata --locked --format-version 1 --filter-platform aarch64-linux-android' <<<"$args" \
        && grep -qxF 'tree --locked -e normal --target aarch64-linux-android --prefix none -f {p} -p interweave-human-android' <<<"$args"; then
        pass "cargo is asked for the given target and package, locked, normal edges"
    else
        fail "cargo was not asked for that target and package" "$args"
    fi
else
    fail "a complete set of inputs was refused" "$(cat "$SANDBOX/err")"
fi

# The overrides: no cargo call at all.
: > "$SANDBOX/cargo-args"
RELEASE_METADATA_JSON="$M" RELEASE_COMPILED_LIST="$C" run aarch64-linux-android interweave-human-android 1.2.3 "$D" "$URL" \
    && [[ ! -s "$SANDBOX/cargo-args" ]] && [[ -s "$ASSET" && -s "$J" ]] \
    && pass "with RELEASE_METADATA_JSON and RELEASE_COMPILED_LIST, cargo is not run" \
    || fail "the overrides still ran cargo, or wrote nothing" "$(cat "$SANDBOX/cargo-args" "$SANDBOX/err")"

# A refusal of release_notices.py: a compiled list that does not name
# the package.
printf 'libfoo v1.0.0\n' > "$SANDBOX/no-root.txt"
plant
STUB_TREE="$SANDBOX/no-root.txt" run aarch64-linux-android interweave-human-android 1.2.3 "$D" "$URL"
status=$?
[[ $status == 1 ]] && none_left && grep -qF 'could not be written' "$SANDBOX/err" \
    && pass "release_notices.py's refusal is exit 1, named, and leaves none of the three, an earlier run's included" \
    || fail "a refusal: exit $status, or an output was left" "$(ls -A "$D"; cat "$SANDBOX/err")"

plant
run aarch64-linux-android interweave-human-android 1.2 "$D" "$URL"
status=$?
[[ $status == 1 ]] && none_left && grep -qF 'is not MAJOR.MINOR.PATCH' "$SANDBOX/err" \
    && pass "a version that is not MAJOR.MINOR.PATCH is refused, leaving none of the three" \
    || fail "version 1.2: exit $status, or an output was left" "$(ls -A "$D"; cat "$SANDBOX/err")"

# cargo failing.
cat > "$STUB/cargo" <<'EOF'
#!/bin/sh
echo "error: failed to load manifest" >&2; exit 101
EOF
plant
run aarch64-linux-android interweave-human-android 1.2.3 "$D" "$URL"
status=$?
[[ $status == 1 ]] && none_left && grep -qF 'cargo metadata failed' "$SANDBOX/err" \
    && pass "cargo failing is exit 1, named, with none of the three left" \
    || fail "cargo failing: exit $status" "$(ls -A "$D"; cat "$SANDBOX/err")"

# cargo tree failing after metadata succeeded: its own message, exit 1,
# never cargo's status.
cat > "$STUB/cargo" <<EOF
#!/bin/sh
case "\$1" in
metadata) cat "$M" ;;
*) echo "error: tree failed" >&2; exit 101 ;;
esac
EOF
plant
run aarch64-linux-android interweave-human-android 1.2.3 "$D" "$URL"
status=$?
[[ $status == 1 ]] && none_left && grep -qF 'cargo tree failed' "$SANDBOX/err" \
    && pass "cargo tree failing is exit 1, named, with none of the three left" \
    || fail "cargo tree failing: exit $status" "$(ls -A "$D"; cat "$SANDBOX/err")"

bash "$UNDER_TEST" aarch64-linux-android interweave-human-android 1.2.3 "$D" >/dev/null 2>&1; status=$?
[[ $status == 2 ]] && pass "a missing argument is a usage error" || fail "four arguments: exit $status, not 2"
help_out="$(bash "$UNDER_TEST" --help 2>/dev/null)"
[[ "$help_out" == *"A 2026-10-22 (ii)"* ]] && pass "--help prints the help block" || fail "--help" "$help_out"

echo
if (( failures )); then
    echo "test_release_crate_notices: $failures failure(s)" >&2
    exit 1
fi
echo "test_release_crate_notices: all passed"
