#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_check_glibc_floor.sh
#
# Self-test for check_glibc_floor.py, against ELF objects linked here
# with cc against a stub libc.so.6 whose version nodes the test chooses,
# so what each object needs is the test's and not the host's glibc: an
# object needing GLIBC_2.43 is refused at 2.35 and named with that
# version, and passes at 2.43 (the control that the refusal is the
# floor's); one needing only 2.35 passes at 2.35; GLIBC_ABI_DT_RELR is
# ordered as 2.36, refused at 2.35 and passed at 2.36; GLIBC_PRIVATE and
# an unknown GLIBC_ABI_ marker are refused at any floor; a weak symbol's
# version counts; an object needing no versions passes; a non-ELF
# file and a malformed floor are usage (exit 2).
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/check_glibc_floor.py"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }
command -v cc >/dev/null || { echo "test_check_glibc_floor: no cc on PATH" >&2; exit 1; }
command -v readelf >/dev/null || { echo "test_check_glibc_floor: no readelf on PATH" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_check_glibc_floor: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# The stub C library: one function per version node.
cat > "$SANDBOX/libc.map" <<'EOF'
GLIBC_2.2.5 { global: v_2_2_5; local: *; };
GLIBC_2.35 { global: v_2_35; } GLIBC_2.2.5;
GLIBC_ABI_DT_RELR { global: v_relr; } GLIBC_2.35;
GLIBC_2.43 { global: v_2_43; } GLIBC_ABI_DT_RELR;
GLIBC_PRIVATE { global: v_private; } GLIBC_2.43;
GLIBC_ABI_SOMETHING_NEW { global: v_unknown; } GLIBC_PRIVATE;
EOF
fns=(v_2_2_5 v_2_35 v_relr v_2_43 v_private v_unknown)
for f in "${fns[@]}"; do echo "void $f(void){}"; done > "$SANDBOX/libc.c"
cc -shared -nostdlib -fPIC -Wl,-soname,libc.so.6 -Wl,--version-script="$SANDBOX/libc.map" \
    -o "$SANDBOX/libc.so.6" "$SANDBOX/libc.c" || { echo "test_check_glibc_floor: the stub libc did not link" >&2; exit 1; }

# obj <name> <fn>... [weak <fn>...]: a shared object calling each <fn>
# of the stub, those after `weak` through a weak reference.
obj() {
    local name="$1"; shift
    local attr="" decls="" calls=""
    for f in "$@"; do
        if [[ "$f" == weak ]]; then attr="__attribute__((weak)) "; continue; fi
        decls+="${attr}void $f(void);"$'\n'
        calls+="  $f();"$'\n'
    done
    printf '%svoid use(void) {\n%s}\n' "$decls" "$calls" > "$SANDBOX/$name.c"
    cc -shared -nostdlib -fPIC -o "$SANDBOX/$name" "$SANDBOX/$name.c" "$SANDBOX/libc.so.6" \
        || { echo "test_check_glibc_floor: $name did not link" >&2; exit 1; }
}
obj new.so v_2_2_5 v_2_43
obj floor.so v_2_2_5 v_2_35
obj relr.so v_relr
obj private.so v_private
obj unknown.so v_unknown
# A strong reference beside the weak one, as a real program has: a
# toolchain that links with --as-needed (Ubuntu's gcc does by default)
# drops a library only weak references use, and the weak one is then
# left unversioned -- Rust's std reaches pidfd_spawnp weakly from a
# binary that needs libc for everything else.
obj weak.so v_2_2_5 weak v_2_43
obj none.so

# The fixtures are what the test says they are: readelf, not the script,
# is asked, so a stub that linked without its versions fails here.
readelf -V -W "$SANDBOX/new.so" | grep -q 'Name: GLIBC_2.43' \
    && pass "fixture: new.so needs GLIBC_2.43" || fail "fixture: new.so does not need GLIBC_2.43"
readelf -W --dyn-syms "$SANDBOX/weak.so" | grep -q 'WEAK .*v_2_43@GLIBC_2.43' \
    && pass "fixture: weak.so's v_2_43@GLIBC_2.43 is a weak symbol" \
    || fail "fixture: weak.so's v_2_43 is not a weak GLIBC_2.43 symbol" "$(readelf -W --dyn-syms "$SANDBOX/weak.so")"
readelf -V -W "$SANDBOX/none.so" | grep -q 'Version needs' \
    && fail "fixture: none.so has version needs" || pass "fixture: none.so needs no version"

# run <floor> <file>...: sets out, err and status.
run() {
    status=0
    out="$(python3 "$UNDER_TEST" "$@" 2>"$SANDBOX/err")" || status=$?
    err="$(cat "$SANDBOX/err")"
}
expect() {  # expect <label> <status> [<stderr fragment>]
    if [[ $status == "$2" && ( -z "${3-}" || "$err" == *"$3"* ) ]]; then pass "$1"
    else fail "$1" "status $status, wanted $2; stdout: $out; stderr: $err"; fi
}

run 2.35 "$SANDBOX/new.so"
expect "GLIBC_2.43 is refused at 2.35, naming it" 1 "needs GLIBC_2.43, above the floor 2.35"
[[ "$out" == "$SANDBOX/new.so: GLIBC_2.43" ]] && pass "its highest version is printed" \
    || fail "its highest version is printed" "$out"
run 2.43 "$SANDBOX/new.so"
expect "control: the same object passes at 2.43" 0
run 2.35 "$SANDBOX/floor.so"
expect "GLIBC_2.35 passes at 2.35" 0
run 2.34 "$SANDBOX/floor.so"
expect "GLIBC_2.35 is refused at 2.34" 1 "needs GLIBC_2.35"
run 2.35 "$SANDBOX/relr.so"
expect "GLIBC_ABI_DT_RELR is refused at 2.35" 1 "needs GLIBC_ABI_DT_RELR, above the floor 2.35"
run 2.36 "$SANDBOX/relr.so"
expect "GLIBC_ABI_DT_RELR passes at 2.36" 0
run 9.99 "$SANDBOX/private.so"
expect "GLIBC_PRIVATE is refused at any floor" 1 "needs GLIBC_PRIVATE, which has no version to compare"
run 9.99 "$SANDBOX/unknown.so"
expect "an unknown GLIBC_ABI_ marker is refused at any floor" 1 "needs GLIBC_ABI_SOMETHING_NEW, which has no version"
run 2.35 "$SANDBOX/weak.so"
expect "a weak symbol's GLIBC_2.43 is refused at 2.35" 1 "needs GLIBC_2.43"
run 2.35 "$SANDBOX/none.so"
expect "an object needing no version passes" 0
[[ "$out" == "$SANDBOX/none.so: none" ]] && pass "and prints none" || fail "and prints none" "$out"
run 2.35 "$SANDBOX/floor.so" "$SANDBOX/new.so"
expect "one refused file among passing ones fails the run" 1 "new.so needs GLIBC_2.43"
run 2.35 "$SANDBOX/libc.c"
expect "a non-ELF file is exit 2" 2 "not readable as ELF"
run 2.x "$SANDBOX/floor.so"
expect "a malformed floor is usage" 2 "usage"
run 2.35
expect "no file is usage" 2 "usage"

if (( failures > 0 )); then
    echo "test_check_glibc_floor: $failures failed" >&2
    exit 1
fi
echo "test_check_glibc_floor: all passed"
