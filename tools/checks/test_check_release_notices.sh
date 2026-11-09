#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/test_check_release_notices.sh
#
# Self-test for check_release_notices.sh.
#
# The guard runs with --root on a sandbox holding its two release sources
# (the archive script's `target=`/`packages=` lines and release.yml's
# release_crate_notices.sh call) and a stub release_notices.py that
# records its arguments and answers with the exit RN_EXIT_<target> names;
# a stub `cargo` on PATH records each call and fails when CARGO_FAIL
# matches it. The cases that matter most: a package added to the archive
# is graphed without editing the guard, each build is resolved with the
# release scripts' own cargo calls, a refusal is exit 1 naming the build,
# and a source that cannot be read is exit 2, never a pass.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/check_release_notices.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_check_release_notices: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
WS="$SANDBOX/ws"
mkdir -p "$WS/tools/ci" "$WS/.github/workflows" "$SANDBOX/bin"
LOG="$SANDBOX/calls"

pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# archive <packages…>: the release script's two lines the guard reads.
archive() {
    { echo '#!/usr/bin/env bash'
      echo 'target=x86_64-unknown-linux-gnu'
      echo "packages=($*)"
      echo 'python3 "$here/release_notices.py" "$metadata" "$compiled" "$out" "${packages[@]}"'; } \
        > "$WS/tools/ci/release_linux_archive.sh"
}
# release [<call line>…]: release.yml's notices step, the call as given.
release() {
    { echo 'jobs:'
      echo '  apk:'
      echo '    steps:'
      echo '      - name: The native library'"'"'s crate notices'
      echo '        run: |'
      if (( $# )); then printf '%s\n' "$@"
      else echo '          bash tools/ci/release_crate_notices.sh aarch64-linux-android apkpkg \'; fi
      echo '            "$VERSION" "$RUNNER_TEMP/out" "$BASE"'; } > "$WS/.github/workflows/release.yml"
}

cat > "$WS/tools/ci/release_notices.py" <<'PY'
import os, sys
metadata, compiled, out, *packages = sys.argv[1:]
target = open(metadata).read().strip()
with open(os.environ["LOG"], "a") as log:
    log.write("notices " + target + " " + " ".join(packages) + " | " + open(compiled).read().strip() + "\n")
code = int(os.environ.get("RN_EXIT_" + target.replace("-", "_"), "0"))
if code:
    sys.stderr.write(f"release_notices: refused for {target}\n")
else:
    open(out, "w").write("page\n")
    print(f"{len(packages)} crates, 1 distinct licence texts")
sys.exit(code)
PY
# cargo: `metadata` prints its --filter-platform (the stub tool reads it
# as the target); `tree` prints its own arguments as the compiled list.
cat > "$SANDBOX/bin/cargo" <<'SH'
#!/usr/bin/env bash
echo "cargo $*" >> "$LOG"
if [[ -n "${CARGO_FAIL-}" && "$*" == *"$CARGO_FAIL"* ]]; then echo "cargo: failed" >&2; exit 101; fi
case "$1" in
metadata) while (( $# )); do [[ "$1" == --filter-platform ]] && { echo "$2"; exit 0; }; shift; done; exit 101 ;;
tree) echo "tree $*" ;;
*) exit 101 ;;
esac
SH
chmod +x "$SANDBOX/bin/cargo"

run() {
    : > "$LOG"
    LOG="$LOG" PATH="$SANDBOX/bin:$PATH" bash "$UNDER_TEST" --root "$WS" >"$SANDBOX/out" 2>"$SANDBOX/err"
}
expect() {  # expect <code> <what> [<needle in stdout+stderr>]
    local code="$1" what="$2" needle="${3-}" got=0
    run || got=$?
    if (( got != code )); then fail "$what: exit $got, not $code" "$(cat "$SANDBOX/out" "$SANDBOX/err")"
    elif [[ -n "$needle" ]] && ! grep -qF -- "$needle" "$SANDBOX/out" "$SANDBOX/err"; then
        fail "$what: does not say $needle" "$(cat "$SANDBOX/out" "$SANDBOX/err")"
    else pass "$what"; fi
}

echo "check_release_notices.sh:"
archive daemon desktop
release
expect 0 "both builds' notices written: exit 0" "the APK's library (aarch64-linux-android: apkpkg): 1 crates"
grep -qxF 'cargo metadata --locked --format-version 1 --filter-platform x86_64-unknown-linux-gnu' "$LOG" \
    && grep -qxF "cargo tree --locked -e normal --target x86_64-unknown-linux-gnu --prefix none -f {p} -p daemon -p desktop" "$LOG" \
    && grep -qxF "notices x86_64-unknown-linux-gnu daemon desktop | tree tree --locked -e normal --target x86_64-unknown-linux-gnu --prefix none -f {p} -p daemon -p desktop" "$LOG" \
    && pass "the archive's graph is the release script's cargo calls, its packages handed to the tool" \
    || fail "the archive's graph is not resolved as the release resolves it" "$(cat "$LOG")"
grep -qxF "cargo tree --locked -e normal --target aarch64-linux-android --prefix none -f {p} -p apkpkg" "$LOG" \
    && grep -qF "notices aarch64-linux-android apkpkg | " "$LOG" \
    && pass "the APK's graph is release_crate_notices.sh's, for the target and package release.yml names" \
    || fail "the APK's graph is not resolved as the release resolves it" "$(cat "$LOG")"

archive daemon desktop extra
expect 0 "a package added to the archive is graphed without editing the guard"
grep -qF -- '-p daemon -p desktop -p extra' "$LOG" || fail "the added package was not graphed" "$(cat "$LOG")"
archive daemon desktop

RN_EXIT_aarch64_linux_android=1 expect 1 "a refusal for the APK's library is exit 1" \
    "FAIL: the APK's library (aarch64-linux-android: apkpkg): release_notices: refused for aarch64-linux-android"
grep -q '^notices x86_64' "$LOG" \
    && pass "a refusal of one build does not stop the other being checked" \
    || fail "the archive was not checked beside the refused APK"
RN_EXIT_x86_64_unknown_linux_gnu=1 expect 1 "a refusal for the Linux archive is exit 1" \
    "FAIL: the Linux archive (x86_64-unknown-linux-gnu: daemon desktop)"
RN_EXIT_x86_64_unknown_linux_gnu=3 expect 2 "the tool's exit other than 0 or 1 is no answer: exit 2" \
    "release_notices.py did not answer for the Linux archive"

CARGO_FAIL="metadata" expect 2 "cargo metadata failing is exit 2" "cargo metadata for x86_64-unknown-linux-gnu failed"
CARGO_FAIL="aarch64-linux-android --prefix" expect 2 "cargo tree failing is exit 2" "cargo tree --locked -e normal --target aarch64-linux-android"

{ cat "$WS/tools/ci/release_linux_archive.sh"; echo 'packages=(other)'; } > "$SANDBOX/two"
mv "$SANDBOX/two" "$WS/tools/ci/release_linux_archive.sh"
expect 2 "two packages=(...) lines are a reading to update: exit 2" "has 2 line(s)"
archive daemon desktop
sed -i 's/^target=.*/target="$1"/' "$WS/tools/ci/release_linux_archive.sh"
expect 2 "a target that is not a literal is a reading to update: exit 2" "has 0 line(s) of the shape /^target="
archive daemon desktop
release '          bash tools/ci/release_crate_notices.sh "$TARGET" apkpkg \'
expect 2 "an APK call taking its target from an expression: exit 2" "has 0 line(s) of the shape"
release '          bash tools/ci/release_crate_notices.sh aarch64-linux-android apkpkg \' \
    '          bash tools/ci/release_crate_notices.sh x86_64-linux-android apkpkg \'
expect 2 "a second release_crate_notices.sh call: exit 2" "names release_crate_notices.sh on 2 lines"
release '          echo no notices here'
expect 2 "no release_crate_notices.sh call: exit 2" "names release_crate_notices.sh on 0 lines"
release
rm "$WS/.github/workflows/release.yml"
expect 2 "a missing release.yml: exit 2" "release_crate_notices.sh on 0 lines"
release

bash "$UNDER_TEST" --bogus >/dev/null 2>&1
[[ $? -eq 2 ]] && pass "an unknown argument is usage (2)" || fail "an unknown argument did not exit 2"
bash "$UNDER_TEST" --help | grep -q '^Would every shipped build' \
    && pass "--help prints the help block" || fail "--help does not print the help block"

if (( failures )); then
    echo "test_check_release_notices.sh: $failures failure(s)" >&2
    exit 1
fi
echo "test_check_release_notices.sh: all assertions passed"
