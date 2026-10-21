#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_release_linux_archive.sh
#
# Self-test for release_linux_archive.sh and release_notices.py.
#
# The three programs are stand-in scripts (--bin-dir) and the dependency
# graph a small `cargo metadata` document over sandbox crates, so nothing
# is compiled. The archive is unpacked and its own install.sh run into a
# sandbox home -- the positive control that the layout is the one the
# script expects -- with systemctl a stub that finds no user manager, so
# the test never touches the account's real one. Two builds from the same
# inputs must be the same bytes.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/release_linux_archive.sh"
NOTICES="$SCRIPT_DIR/release_notices.py"
[[ -f "$UNDER_TEST" && -f "$NOTICES" ]] || { echo "test: scripts not found" >&2; exit 1; }

# The first build's umask, so the second's (077) is another one.
umask 022
failures=0
SANDBOX="$(realpath -- "$(mktemp -d)")"
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

BIN="$SANDBOX/bin"
mkdir -p "$BIN"
for p in transport-daemon transportctl human-desktop; do
    printf '#!/bin/sh\necho %s\n' "$p" > "$BIN/$p"
    chmod 0700 "$BIN/$p"
done

# crate <dir> <file>...: a sandbox crate directory with those files.
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
crate daemon
crate ctl
crate desktop
crate libfoo LICENSE-MIT LICENSE-APACHE
crate libbar LICENSE-MIT
cp "$SANDBOX/crates/libfoo/LICENSE-MIT" "$SANDBOX/crates/libbar/LICENSE-MIT"
crate macro LICENSES/MIT.txt
crate bare
crate buildonly LICENSE
crate devonly LICENSE
crate unified LICENSE

M="$SANDBOX/metadata.json"
python3 - "$SANDBOX" > "$M" <<'PY'
import json, sys
root = sys.argv[1] + "/crates"
def pkg(name, dir, license, repo=None):
    return {"id": f"{name}-id", "name": name, "version": "1.0.0", "license": license,
            "license_file": None, "repository": repo,
            "manifest_path": f"{root}/{dir}/Cargo.toml"}
packages = [
    pkg("interweave-transport-daemon", "daemon", "Apache-2.0"),
    pkg("interweave-transportctl", "ctl", "Apache-2.0"),
    pkg("interweave-human-desktop", "desktop", "Apache-2.0"),
    pkg("libfoo", "libfoo", "MIT OR Apache-2.0"),
    pkg("libbar", "libbar", "MIT"),
    pkg("a-macro", "macro", "MIT"),
    pkg("bare", "bare", "MIT", "https://example.invalid/bare"),
    pkg("buildonly", "buildonly", "MIT"),
    pkg("devonly", "devonly", "MIT"),
    pkg("unified", "unified", "MIT"),
]
normal = [{"kind": None, "target": None}]
def dep(name, kinds=normal):
    return {"name": name, "pkg": f"{name}-id", "dep_kinds": kinds}
nodes = [
    {"id": "interweave-transport-daemon-id", "deps": [dep("libfoo"), dep("buildonly", [{"kind": "build", "target": None}])]},
    {"id": "interweave-transportctl-id", "deps": [dep("libbar"), dep("devonly", [{"kind": "dev", "target": None}])]},
    # unified: reached in the metadata's unified resolve, compiled by no
    # build of these three (as the Android app's features reach Skia).
    {"id": "interweave-human-desktop-id", "deps": [dep("a-macro"), dep("bare"), dep("unified")]},
] + [{"id": f"{n}-id", "deps": []} for n in ("libfoo", "libbar", "a-macro", "bare", "buildonly", "devonly", "unified")]
print(json.dumps({"packages": packages, "workspace_members": [p["id"] for p in packages[:3]],
                  "resolve": {"nodes": nodes}}))
PY

# What `cargo tree -e normal -p <the three>` prints: the roots, the
# crates their build compiles, a repeat marked (*).
C="$SANDBOX/compiled.txt"
cat > "$C" <<'EOF'
interweave-transport-daemon v1.0.0 (/somewhere/apps/transport-daemon)
libfoo v1.0.0
interweave-transportctl v1.0.0 (/somewhere/apps/transportctl)
libbar v1.0.0
interweave-human-desktop v1.0.0 (/somewhere/apps/human-desktop)
a-macro v1.0.0
libfoo v1.0.0 (*)
EOF

cat > "$BIN/systemctl" <<'EOF'
#!/bin/sh
exit 1
EOF
chmod +x "$BIN/systemctl"

OUT="$SANDBOX/out"
build() {
    rm -rf "$OUT"
    SOURCE_DATE_EPOCH=1760000000 RELEASE_METADATA_JSON="$M" RELEASE_COMPILED_LIST="$C" \
        bash "$UNDER_TEST" --bin-dir "$BIN" "$@" >"$SANDBOX/stdout" 2>"$SANDBOX/err"
}

echo "release_linux_archive.sh:"
NAME=interweave-0.3.0-x86_64-unknown-linux-gnu
A="$OUT/$NAME.tar.gz"
if build 0.3.0 "$OUT"; then
    listing="$(tar -tzvf "$A" | awk '{print $1, $2, $6}' | sort -k3)"
    want="$(sort -k3 <<EOF
drwxr-xr-x 0/0 $NAME/
-rw-r--r-- 0/0 $NAME/LICENSE
-rw-r--r-- 0/0 $NAME/README.md
-rw-r--r-- 0/0 $NAME/THIRD-PARTY-NOTICES.md
drwxr-xr-x 0/0 $NAME/bin/
-rwxr-xr-x 0/0 $NAME/bin/human-desktop
-rwxr-xr-x 0/0 $NAME/bin/transport-daemon
-rwxr-xr-x 0/0 $NAME/bin/transportctl
-rwxr-xr-x 0/0 $NAME/install.sh
drwxr-xr-x 0/0 $NAME/systemd/
-rw-r--r-- 0/0 $NAME/systemd/interweave-client@.service
-rw-r--r-- 0/0 $NAME/systemd/interweave-daemon@.service
-rwxr-xr-x 0/0 $NAME/uninstall.sh
EOF
)"
    if [[ "$listing" == "$want" ]]; then
        pass "the archive holds exactly the layout, owned by 0/0 with release modes"
    else
        fail "the archive's listing is not the expected one" "$(diff <(echo "$want") <(echo "$listing"))"
    fi
    if (cd "$OUT" && sha256sum -c --quiet "$NAME.tar.gz.sha256"); then
        pass "the .sha256 file checks against the archive"
    else
        fail "the .sha256 file does not check"
    fi
    want_json="{\"linux_archive\": \"$NAME.tar.gz\", \"linux_sha256\": \"$(sha256sum "$A" | cut -d' ' -f1)\", \"check:linux_version\": \"0.3.0\"}"
    [[ "$(cat "$OUT/linux-values.json")" == "$want_json" ]] \
        && pass "linux-values.json names the archive and its hash" \
        || fail "linux-values.json is not the expected one" "$(cat "$OUT/linux-values.json")"
    stamps="$(python3 -c 'import sys, tarfile; print(sorted({m.mtime for m in tarfile.open(sys.argv[1])}))' "$A")"
    [[ "$stamps" == "[1760000000]" ]] \
        && pass "every entry's mtime is SOURCE_DATE_EPOCH" \
        || fail "the entries' mtimes are $stamps, not SOURCE_DATE_EPOCH alone"
    first="$(sha256sum "$A" | cut -d' ' -f1)"
    touch "$BIN/transportctl"
    # The second build under another umask: the builder's is not an input.
    (umask 077 && build 0.3.0 "$OUT")
    [[ "$(sha256sum "$A" | cut -d' ' -f1)" == "$first" ]] \
        && pass "two builds from the same inputs, under umasks 022 and 077, are the same bytes" \
        || fail "a second build of the same inputs, under umask 077, differs" "$(tar -tzvf "$A" | head -4)"

    X="$SANDBOX/x"
    mkdir -p "$X" "$SANDBOX/home"
    tar -xzf "$A" -C "$X"
    if HOME="$SANDBOX/home" XDG_CONFIG_HOME="$SANDBOX/home/.config" PATH="$BIN:$PATH" \
        sh "$X/$NAME/install.sh" >"$SANDBOX/install.out" 2>&1 \
        && [[ "$("$SANDBOX/home/.local/bin/transportctl")" == transportctl ]] \
        && [[ -f "$SANDBOX/home/.config/systemd/user/interweave-daemon@.service" ]]; then
        pass "the unpacked archive's install.sh installs its programs and units"
    else
        fail "the unpacked archive's install.sh did not install" "$(cat "$SANDBOX/install.out")"
    fi

    N="$X/$NAME/THIRD-PARTY-NOTICES.md"
    for line in '- `libfoo` 1.0.0: MIT OR Apache-2.0 — text T' '- `libbar` 1.0.0: MIT — text T' \
        '- `a-macro` 1.0.0: MIT — text T'; do
        grep -qF -- "$line" "$N" || { fail "the notices lack: $line"; lacking=1; }
    done
    [[ -n "${lacking-}" ]] || pass "the notices list every crate reached by a normal dependency, a proc macro included"
    if grep -qE 'buildonly|devonly|unified|interweave-' "$N"; then
        fail "the notices list a build-only, dev-only, uncompiled or workspace crate" "$(grep -E 'buildonly|devonly|unified|interweave-' "$N")"
    else
        pass "a build-dependency, a dev-dependency, a crate the build does not compile and the workspace's own crates are not listed"
    fi
    shared="$(grep -c '^the text of LICENSE-MIT$' "$N")"
    [[ "$shared" == 1 ]] && grep -qF 'Used by libbar 1.0.0 (LICENSE-MIT); libfoo 1.0.0 (LICENSE-MIT).' "$N" \
        && pass "a text two crates share is printed once and names both" \
        || fail "a shared text is printed $shared times or not attributed to both"
    grep -qx 'the text of MIT.txt' "$N" \
        && pass "a LICENSES/ directory's texts are included" \
        || fail "a LICENSES/ directory's text is missing"
else
    fail "a complete set of inputs was refused" "$(cat "$SANDBOX/err")"
fi

refused() {
    local what="$1" needle="$2"; shift 2
    if build "$@"; then fail "$what: accepted"
    elif ls "$OUT"/*.tar.gz >/dev/null 2>&1; then fail "$what: refused but left an archive"
    elif ! grep -qF -- "$needle" "$SANDBOX/err"; then fail "$what: refused without saying $needle" "$(cat "$SANDBOX/err")"
    else pass "$what: refused, no archive"; fi
}
# The last write fails, after the archive and its hash are moved: an
# `mv` on PATH that refuses the values file. Nothing of the run is left.
STUB="$SANDBOX/stub"
mkdir -p "$STUB"
cat > "$STUB/mv" <<EOF
#!/bin/sh
for last; do :; done
case "\$last" in *-values.json) echo "mv: refused by the test" >&2; exit 1 ;; esac
exec $(command -v mv) "\$@"
EOF
chmod +x "$STUB/mv"
rm -rf "$OUT"
if PATH="$STUB:$PATH" SOURCE_DATE_EPOCH=1760000000 RELEASE_METADATA_JSON="$M" RELEASE_COMPILED_LIST="$C" \
    bash "$UNDER_TEST" --bin-dir "$BIN" 0.3.0 "$OUT" >/dev/null 2>"$SANDBOX/err"; then
    fail "a run whose values file cannot be moved succeeded"
elif ! grep -q 'refused by the test' "$SANDBOX/err"; then
    fail "the stub was not reached at the last move" "$(cat "$SANDBOX/err")"
elif [[ -e "$A" || -e "$A.sha256" ]]; then
    fail "a run that failed at its last move left an archive or its hash" "$(ls -A "$OUT")"
else
    pass "a run that fails at its last move leaves no archive and no hash"
fi
rm -rf "$OUT"
mkdir -p "$OUT"
printf 'a previous run\n' > "$A"
mv "$BIN/human-desktop" "$BIN/human-desktop.gone"
SOURCE_DATE_EPOCH=1760000000 RELEASE_METADATA_JSON="$M" RELEASE_COMPILED_LIST="$C" \
    bash "$UNDER_TEST" --bin-dir "$BIN" 0.3.0 "$OUT" >/dev/null 2>&1
mv "$BIN/human-desktop.gone" "$BIN/human-desktop"
[[ ! -e "$A" ]] && pass "a refused run leaves no previous run's archive of the same name" \
    || fail "a refused run left a previous run's archive"

# The scratch directory cannot be made (a full or unwritable TMPDIR): a
# failure after the version check like any other, so a previous run's
# three outputs are removed. The control: all three present before.
FAILTMP="$SANDBOX/failtmp"
mkdir -p "$FAILTMP"
printf '#!/bin/sh\necho "mktemp: refused by the test" >&2\nexit 1\n' > "$FAILTMP/mktemp"
chmod +x "$FAILTMP/mktemp"
rm -rf "$OUT"
mkdir -p "$OUT"
for f in "$A" "$A.sha256" "$OUT/linux-values.json"; do printf 'a previous run\n' > "$f"; done
[[ -e "$A" && -e "$A.sha256" && -e "$OUT/linux-values.json" ]] || fail "the control: a previous run's outputs"
if PATH="$FAILTMP:$PATH" SOURCE_DATE_EPOCH=1760000000 RELEASE_METADATA_JSON="$M" RELEASE_COMPILED_LIST="$C" \
    bash "$UNDER_TEST" --bin-dir "$BIN" 0.3.0 "$OUT" >/dev/null 2>"$SANDBOX/err"; then
    fail "a run with no scratch directory succeeded"
elif ! grep -q 'refused by the test' "$SANDBOX/err"; then
    fail "the stub was not reached at mktemp" "$(cat "$SANDBOX/err")"
elif [[ -e "$A" || -e "$A.sha256" || -e "$OUT/linux-values.json" ]]; then
    fail "a run with no scratch directory left a previous run's outputs" "$(ls -A "$OUT")"
else
    pass "a run that cannot make its scratch directory leaves none of a previous run's outputs"
fi

# Without --bin-dir the script builds, and takes the executables from the
# directory cargo reports, not one it computes. A cargo stand-in builds
# fresh programs into a directory of its own and reports it; the run starts
# from another directory holding decoys at <cwd>/target, where a relative
# CARGO_TARGET_DIR=target read from the caller's side would look. The
# archive must hold the fresh bytes. (That real cargo reports a relative
# CARGO_TARGET_DIR or a config target-dir in `target_directory` is cargo's
# behaviour, read on the host, not this stand-in's.)
CARGO_STUB="$SANDBOX/cargo-stub"
BUILT="$SANDBOX/cargo-built"
CALLER="$SANDBOX/caller"
mkdir -p "$CARGO_STUB" "$CALLER/target/x86_64-unknown-linux-gnu/release"
cat > "$CARGO_STUB/cargo" <<EOF
#!/bin/sh
case "\$1" in
build)
    mkdir -p "$BUILT/x86_64-unknown-linux-gnu/release"
    for p in transport-daemon transportctl human-desktop; do
        printf '#!/bin/sh\n# fresh %s\n' "\$p" > "$BUILT/x86_64-unknown-linux-gnu/release/\$p"
        chmod 0755 "$BUILT/x86_64-unknown-linux-gnu/release/\$p"
    done ;;
metadata) printf '{"target_directory": "%s"}\n' "$BUILT" ;;
*) echo "cargo stand-in: \$*" >&2; exit 1 ;;
esac
EOF
chmod +x "$CARGO_STUB/cargo"
for p in transport-daemon transportctl human-desktop; do
    printf '#!/bin/sh\n# decoy %s\n' "$p" > "$CALLER/target/x86_64-unknown-linux-gnu/release/$p"
    chmod 0755 "$CALLER/target/x86_64-unknown-linux-gnu/release/$p"
done
rm -rf "$OUT"
if ! (cd "$CALLER" && PATH="$CARGO_STUB:$PATH" CARGO_TARGET_DIR=target SOURCE_DATE_EPOCH=1760000000 \
    RELEASE_METADATA_JSON="$M" RELEASE_COMPILED_LIST="$C" \
    bash "$UNDER_TEST" 0.3.0 "$OUT" >/dev/null 2>"$SANDBOX/err"); then
    fail "a build run from another directory failed" "$(cat "$SANDBOX/err")"
else
    Y="$SANDBOX/y"
    rm -rf "$Y"
    mkdir -p "$Y"
    tar -xzf "$A" -C "$Y"
    if grep -q '# fresh transport-daemon' "$Y/$NAME/bin/transport-daemon" \
        && ! grep -rq '# decoy' "$Y/$NAME/bin"; then
        pass "a build's executables are taken from the directory cargo reports, not one computed beside the caller"
    else
        fail "the archive's programs are not the ones this build made" "$(head -n 2 "$Y/$NAME"/bin/*)"
    fi
fi

refused "a version that is not MAJOR.MINOR.PATCH" "not MAJOR.MINOR.PATCH" 0.3 "$OUT"
mv "$BIN/human-desktop" "$BIN/human-desktop.gone"
refused "a missing program" "no executable $BIN/human-desktop" 0.3.0 "$OUT"
mv "$BIN/human-desktop.gone" "$BIN/human-desktop"
python3 - "$M" <<'PY'
import json, sys
m = json.load(open(sys.argv[1])); m["packages"][4]["license"] = None
import os; os.remove(m["packages"][4]["manifest_path"].replace("Cargo.toml", "LICENSE-MIT"))
json.dump(m, open(sys.argv[1], "w"))
PY
refused "a crate with no licence expression and no licence file" "libbar 1.0.0 has no licence" 0.3.0 "$OUT"

echo "release_notices.py:"
cp "$C" "$SANDBOX/c2.txt"
echo "buildonly v1.0.0" >> "$SANDBOX/c2.txt"
python3 "$NOTICES" "$M" "$SANDBOX/c2.txt" "$SANDBOX/n.md" interweave-transport-daemon \
    interweave-transportctl interweave-human-desktop >/dev/null 2>"$SANDBOX/err"
[[ $? -eq 1 ]] && grep -qF 'compiled but not reached from the packages in the metadata: buildonly 1.0.0' "$SANDBOX/err" \
    && [[ ! -e "$SANDBOX/n.md" ]] \
    && pass "a compiled crate the metadata's normal walk does not reach is refused" \
    || fail "a compiled crate outside the walk was not refused" "$(cat "$SANDBOX/err")"
python3 "$NOTICES" "$M" "$C" "$SANDBOX/n.md" no-such-member >/dev/null 2>"$SANDBOX/err"
[[ $? -eq 1 ]] && grep -qF 'not a workspace member: no-such-member' "$SANDBOX/err" && [[ ! -e "$SANDBOX/n.md" ]] \
    && pass "a package that is not a workspace member is refused" \
    || fail "an unknown package was not refused" "$(cat "$SANDBOX/err")"

if (( failures )); then
    echo "test_release_linux_archive.sh: $failures failure(s)" >&2
    exit 1
fi
echo "test_release_linux_archive.sh: all assertions passed"
