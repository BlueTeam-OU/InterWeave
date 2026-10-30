#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/test_check_shipped_test_hooks.sh
#
# Self-test for check_shipped_test_hooks.sh.
#
# The guard runs with --root on a sandbox holding its two release sources
# (the archive script's `target=`/`packages=` lines and the cargo-ndk
# call), and a stub `cargo` on PATH answers `cargo metadata` from a
# hand-built member list and `cargo tree` from one file per build, keyed
# by --target (the matcher's control, asked with --features, by
# "control"). The cases that matter most: a package added to the archive
# is checked without editing the guard, a feature on a NON-declaring crate
# fails too, a feature whose name only contains `test-hooks` passes, and
# each positive control turns a blind graph into exit 2, not a pass.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/check_shipped_test_hooks.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[ -n "$SANDBOX" ] && [ -d "$SANDBOX" ] || { echo "test_check_shipped_test_hooks: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
WS="$SANDBOX/ws"
mkdir -p "$WS/tools/ci" "$WS/apps/human-android/android/app" "$SANDBOX/bin" "$SANDBOX/trees"

pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# archive <packages…>: the release script's two lines the guard reads.
archive() {
    { echo '#!/usr/bin/env bash'
      echo 'target=x86_64-unknown-linux-gnu'
      echo "packages=($*)"
      echo 'programs=(transport-daemon)'
      echo 'build() {'
      echo '    build_args=(build --release --locked --target "$target")'
      echo '    for p in "${packages[@]}"; do build_args+=(-p "$p"); done'
      echo '    (cd "$root" && cargo "${build_args[@]}") || die "cargo build failed"'
      echo '}'; } > "$WS/tools/ci/release_linux_archive.sh"
}

# gradle [<release profile line>]: the cargo-ndk call, as build.gradle.kts has it.
gradle() {
    local profile="${1:-            if (variant == \"Release\") listOf(\"--release\") else listOf(\"--features\", \"dev-stand-ins,e2e-cases\")}"
    { echo '        val profile ='
      echo "$profile"
      echo '        // --locked: the reviewed lockfile.'
      echo '        commandLine('
      echo '            listOf('
      echo '                "cargo", "ndk", "-t", "arm64-v8a", "-P", "30",'
      echo '                "-o", rustJniLibs(variant).get().asFile.path,'
      echo '                "build", "--locked", "-p", "apkpkg",'
      echo '            ) + profile,'
      echo '        )'; } > "$WS/apps/human-android/android/app/build.gradle.kts"
}

# members "<name>[:feature,…]"…: the members cargo metadata reports, with the features each declares.
members() {
    python3 - "$SANDBOX/meta.json" "$WS" "$@" <<'PYEOF'
import json, sys
out, ws, *rows = sys.argv[1:]
pk = []
for r in rows:
    n, _, f = r.partition(":")
    pk.append({"id": n, "name": n, "manifest_path": f"{ws}/{n}/Cargo.toml",
               "features": {x: [] for x in f.split(",") if x}})
json.dump({"workspace_root": ws, "workspace_members": [p["id"] for p in pk], "packages": pk}, open(out, "w"))
PYEOF
}

# tree <key> "<name>|<features>"…: what `cargo tree -f '{p}|{f}'` prints for that build.
tree() {
    local key="$1" r; shift
    : > "$SANDBOX/trees/$key"
    for r in "$@"; do echo "${r%%|*} v0.2.0 (/ws/${r%%|*})|${r#*|}" >> "$SANDBOX/trees/$key"; done
}

cat > "$SANDBOX/bin/cargo" <<EOF
#!/usr/bin/env bash
echo "    Updating crates.io index" >&2
case "\$1" in
  metadata)
    [[ -e "$SANDBOX/metadata-fails" ]] && { echo "error: failed to load manifest" >&2; exit 101; }
    cat "$SANDBOX/meta.json" ;;
  tree)
    printf '%s\n' "\$*" >> "$SANDBOX/tree-args"
    [[ -e "$SANDBOX/tree-fails" ]] && { echo "error: package ID specification matched no packages" >&2; exit 101; }
    key=""; while (( \$# )); do
      case "\$1" in --target) key="\$2"; shift ;; --features) key=control; shift ;; esac; shift
    done
    cat "$SANDBOX/trees/\$key" ;;
esac
EOF
chmod +x "$SANDBOX/bin/cargo"

expect() {
    local want="$1" name="$2" says="${3:-}" out got
    out="$(PATH="$SANDBOX/bin:$PATH" bash "$UNDER_TEST" --root "$WS" 2>&1)"; got=$?
    if [[ "$got" -ne "$want" ]]; then fail "$name — wanted exit $want, got $got" "$out"; return; fi
    if [[ -n "$says" && "$out" != *"$says"* ]]; then fail "$name — output lacks: $says" "$out"; return; fi
    pass "$name (exit $got)"
}

LINUX=x86_64-unknown-linux-gnu
APK=aarch64-linux-android
clean() {
    tree $LINUX "daemon|" "comp|default" "cli|" "serde|derive,std"
    tree $APK "apkpkg|" "platform|" "embedded|" "comp|"
    tree control "comp|test-hooks" "serde|std"
}

echo "test_check_shipped_test_hooks"

archive daemon cli
gradle
members daemon cli apkpkg "comp:default,test-hooks" "platform:test-hooks" embedded
clean
rm -f "$SANDBOX/tree-args"
expect 0 "shipped graphs without test-hooks pass, the declarers named" "2 member(s) declare test-hooks: comp platform"
args="$(cat "$SANDBOX/tree-args")"
lin="$(grep -- "--target $LINUX" <<<"$args")"
apk="$(grep -- "--target $APK" <<<"$args")"
[[ "$lin" == *"-p daemon -p cli"* && "$apk" == *"-p apkpkg"* ]] \
    && pass "  and each build is one tree of the packages the release names, for its target" \
    || fail "the builds were not asked as the release names them" "$args"
grep -E -- "--features|--all-features|--no-default-features" <<<"$lin$apk" >/dev/null \
    && fail "a shipped build was asked with other features than the release's" "$args" \
    || pass "  under the release's own features"
[[ "$lin" == *"-e normal --prefix none -f {p}|{f}"* && "$apk" == *"-e normal --prefix none -f {p}|{f}"* ]] \
    && pass "  with -e normal, --prefix none and the resolved-feature format" \
    || fail "cargo tree was not asked for normal edges and {p}|{f}" "$args"
grep -- "-p comp --features test-hooks" <<<"$args" >/dev/null \
    && pass "  and the control is the first declarer built with test-hooks on" \
    || fail "the control was not run on the first declarer" "$args"

tree $APK "apkpkg|" "platform|e2e-cases,test-hooks" "embedded|test-hooks" "comp|test-hooks"
expect 1 "test-hooks in the APK's graph fails, naming every package" "FAIL: the APK's library ($APK: apkpkg) turns test-hooks on in: comp embedded platform"
clean
tree $LINUX "daemon|" "comp|default,test-hooks" "cli|"
expect 1 "test-hooks on the archive's composition fails" "FAIL: the Linux archive ($LINUX: daemon cli) turns test-hooks on in: comp"
tree $LINUX "daemon|" "comp|" "cli|" "embedded|test-hooks"
expect 1 "test-hooks on a crate that declares none in metadata fails too" "turns test-hooks on in: embedded"
tree $LINUX "daemon|test-hooks" "cli|"
expect 1 "a shipped root's own test-hooks fails" "turns test-hooks on in: daemon"
tree $LINUX "daemon|" "cli|"
echo 'odd v0.1.0 (/ws/a|b)|test-hooks' >> "$SANDBOX/trees/$LINUX"
expect 1 "the features are read after the LAST '|', a path holding one included" "turns test-hooks on in: odd"
tree $LINUX "daemon|" "cli|no-test-hooks,test-hooks-extra" "comp|test_hooks"
expect 0 "a feature that only contains the name passes"
clean

archive daemon cli newprog
tree $LINUX "daemon|" "cli|" "newprog|" "comp|test-hooks"
expect 1 "a package added to the archive is checked without editing the guard" "($LINUX: daemon cli newprog) turns test-hooks on in: comp"
tree $LINUX "daemon|" "cli|"
expect 2 "an archive package missing from its graph is exit 2 (positive control)" "the graph for $LINUX names no newprog"
archive daemon cli
: > "$SANDBOX/trees/$APK"
expect 2 "an empty APK graph is exit 2 (positive control)" "names no apkpkg"
clean

tree control "comp|default" "serde|std"
expect 2 "a control the matcher cannot see is exit 2" "is not matched"
clean
members daemon cli apkpkg comp platform
expect 2 "no member declaring test-hooks is exit 2" "no workspace member declares test-hooks"
members daemon cli apkpkg "comp:default,test-hooks" "platform:test-hooks" embedded

gradle '            if (variant == "Release") listOf("--release", "--features", "x") else listOf()'
expect 2 "a release profile that gains features is exit 2 until it is read" "release profile is"
gradle
sed -i 's/arm64-v8a/riscv64/' "$WS/apps/human-android/android/app/build.gradle.kts"
expect 2 "an ABI the guard cannot map is exit 2" "the ABI 'riscv64'"
gradle
sed -i '/^packages=/d' "$WS/tools/ci/release_linux_archive.sh"
expect 2 "no packages= line is exit 2" "has 0 line(s) of the shape"
archive daemon cli
echo 'packages=(other)' >> "$WS/tools/ci/release_linux_archive.sh"
expect 2 "two packages= lines are exit 2" "has 2 line(s) of the shape"
archive daemon cli
sed -i 's/^packages=(daemon cli)$/packages=("${base[@]}" cli)/' "$WS/tools/ci/release_linux_archive.sh"
expect 2 "a packages= line outside the read subset is exit 2" "has 0 line(s) of the shape"
archive daemon cli

A_SH="$WS/tools/ci/release_linux_archive.sh"
G_KTS="$WS/apps/human-android/android/app/build.gradle.kts"
sed -i 's|--target "$target")$|--target "$target" --features comp/test-hooks)|' "$A_SH"
expect 2 "a feature on the archive's cargo build is exit 2 until it is read" 'build_args=\(build --release'
archive daemon cli
sed -i '/cargo "\${build_args/i\    build_args+=(--all-features)' "$A_SH"
expect 2 "another build_args line is exit 2" "names build_args on 4 lines"
archive daemon cli
echo 'cargo build --release --features comp/test-hooks -p daemon' >> "$A_SH"
expect 2 "a feature flag anywhere in the archive script is exit 2" "passes a feature flag"
archive daemon cli
sed -i 's|"-p", "apkpkg",$|"-p", "apkpkg", "--features", "platform/test-hooks",|' "$G_KTS"
expect 2 "a feature on the cargo-ndk build line is exit 2" "cargo-ndk call, line"
gradle
sed -i 's|^\( *\)) + profile,$|\1"--features", "platform/test-hooks",\n\1) + profile,|' "$G_KTS"
expect 2 "a line added to the cargo-ndk call is exit 2" "cargo-ndk call, line"
gradle
sed -i 's|"-t", "arm64-v8a",|"-t", "arm64-v8a", "-t", "x86_64",|' "$G_KTS"
expect 2 "a second ABI on the cargo-ndk call is exit 2" "cargo-ndk call, line"
gradle
sed -i 's|) + profile,$|) + profile + listOf("-F", "x"),|' "$G_KTS"
expect 2 "anything appended to the profile is exit 2" "cargo-ndk call, line"
gradle
echo '    tasks.register("x") { args("--all-features") }' >> "$G_KTS"
expect 2 "a feature flag elsewhere in the Gradle file is exit 2" "passes a feature flag on 2 lines"
gradle
gradle '            if (variant == "Release") listOf("--release") else listOf("--features", "a") + listOf("-F", "b")'
expect 2 "a debug profile beyond one listOf is exit 2" "release profile is"
gradle

echo '    tasks.register("y") { args("-Fplatform/test-hooks") }' >> "$G_KTS"
expect 2 "an attached -F<features> in the Gradle file is exit 2" "passes a feature flag on 2 lines"
gradle
echo 'cargo() { command cargo "$@" -F=comp/test-hooks; }' >> "$A_SH"
expect 2 "an attached -F=<features> in the archive script is exit 2" "passes a feature flag"
archive daemon cli
sed -i '/^packages=/a packages+=(extra)' "$A_SH"
expect 2 "a packages+= line is exit 2, not an ungraphed package" "names packages other than"
archive daemon cli
echo 'read -r -a packages <<<"$PKGS"' >> "$A_SH"
expect 2 "a read into packages is exit 2" "names packages other than"
archive daemon cli
sed -i '/^packages=/a packages=("${packages[@]}" extra)' "$A_SH"
expect 2 "a second packages=( line in another shape is exit 2" "names packages other than"
archive daemon cli
sed -i '/^packages=/a packages=(daemon cli extra) ' "$A_SH"
expect 2 "a second packages=( line with a trailing space is exit 2" "names packages other than"
archive daemon cli
echo '    tasks.register("z") { args("-qFplatform/test-hooks") }' >> "$G_KTS"
expect 2 "a clustered -qF<features> in the Gradle file is exit 2" "passes a feature flag on 2 lines"
gradle
echo 'cargo() { command cargo "$@" -rF comp/test-hooks; }' >> "$A_SH"
expect 2 "a clustered -rF <features> in the archive script is exit 2" "passes a feature flag"
archive daemon cli
echo 'notices "${packages[@]}" "$stage"' >> "$A_SH"
expect 0 "a line expanding \"\${packages[@]}\" whole passes"
archive daemon cli

touch "$SANDBOX/tree-fails"
expect 2 "cargo tree failing is exit 2, with cargo's error" "matched no packages"
rm -f "$SANDBOX/tree-fails"
touch "$SANDBOX/metadata-fails"
expect 2 "cargo metadata failing is exit 2, with cargo's error" "failed to load manifest"
rm -f "$SANDBOX/metadata-fails"
expect 0 "the sandbox is clean again (control for the cases above)"

help_out="$(bash "$UNDER_TEST" --help 2>/dev/null)"
[[ "$help_out" == *"POSITIVE CONTROLS"* ]] && pass "--help prints the help block, positive controls included" || fail "--help should print the help block" "$help_out"

echo
if (( failures > 0 )); then
    echo "test_check_shipped_test_hooks: $failures failure(s)" >&2
    exit 1
fi
echo "test_check_shipped_test_hooks: OK — all assertions passed."
