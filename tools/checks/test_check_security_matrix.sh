#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/test_check_security_matrix.sh
#
# Self-test for check_security_matrix.py. Builds throwaway repository
# roots under $TMPDIR, each with a tests/SECURITY-MATRIX.md and the test
# files it names, and asserts the check's verdict on each. A passing root
# is the positive control beside every failing one, so a check that fails
# everything cannot pass this test.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK="$SCRIPT_DIR/check_security_matrix.py"

pass=0
fail=0
ok()  { printf '  ✓ %s\n' "$1"; pass=$((pass + 1)); }
bad() { printf '  ✗ %s\n' "$1" >&2; fail=$((fail + 1)); }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Captured, never piped into `grep -q` -- pipefail plus an early-exiting
# grep turns correct output into a failed pipeline.
run()      { python3 "$CHECK" --root "$@" 2>&1; }
run_code() { python3 "$CHECK" --root "$@" >/dev/null 2>&1; printf '%s' "$?"; }

HEAD='| id | guards | step | tests | limit |
|---|---|---|---|---|'

# root <name> <rows...>: a repository root whose matrix holds the rows, and
# a test file defining `a_guard` and `b_guard`.
root() {
    local dir="$TMP/$1"; shift
    mkdir -p "$dir/tests/security/tests"
    cat > "$dir/tests/security/tests/flood.rs" <<'RS'
#[test]
fn a_guard() {}

#[test]
/// A doc comment between the attribute and the fn.

#[ignore = "a second attribute, past a blank line"]
fn b_guard() {}

#[test] fn one_line_guard() {}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[expect(
    clippy::many_single_char_names,
    reason = "a multi-line attribute, an unbalanced ] in its string"
)]
async fn tokio_guard() {}

fn helper_guard() {}

#[test]
fn mentions() { let _ = "fn string_guard()"; } // fn trailing_guard()

#[test]
// fn swapped_guard() {}
fn the_real_one() {}

/*
#[test]
fn disabled_guard() {}
/* a nested comment */
#[test]
fn nested_disabled_guard() {}
*/

// A quote as a character literal: read as a string opener, it would close
// at the quote inside the comment and leave the disabled test as code.
const Q: char = '"'; /* "
#[test]
fn char_hidden_guard() {}
*/

// An escaped quote: ended there, the string would let its /* open a
// comment that hides the live test below.
const E: &str = "a\" /* ";
#[test]
fn escape_guard() {}

// An odd quote inside: read as a plain string, the raw string would end
// there and leave the test below it as code.
const RAW: &str = r#"say "hi
#[test]
fn raw_guard() {}
"#;

// fn commented_out() {}
/// fn doc_only() names a test in a doc comment
RS
    { printf '# Security matrix\n\n%s\n' "$HEAD"; printf '%s\n' "$@"; } > "$dir/tests/SECURITY-MATRIX.md"
    printf '%s' "$dir"
}

T='`tests/security/tests/flood.rs::a_guard`'
U='`tests/security/tests/flood.rs::b_guard`'
K='`tests/security/tests/flood.rs::tokio_guard`'
O='`tests/security/tests/flood.rs::one_line_guard`'
X='`tests/security/tests/flood.rs::escape_guard`'

echo "check_security_matrix — every named test exists"

good="$(root good "| S21-01 | floods | S1 | $T; $U; $K; $O; $X | — |" "| TM-01 | probing | S2 | $T; owed: S3 lands the rest | — |" "| C-01 | debug line | S4 | owed (S21-05's tests) | — |" "| C-02 | spoofing | S4 | owed | — |" "| C-03 | replay | S4 | — | — |" "C-04 | no leading pipe | S4 | $U | — |")"
out="$(run "$good")"; code="$(run_code "$good")"
[[ "$code" == 0 ]] && ok "a matrix whose tests exist passes" || bad "control: expected 0, got $code: $out"
[[ "$out" == *"7 named test(s) exist"* ]] && ok "  counting the named tests, a row without its leading pipe included" || bad "count: $out"
[[ "$out" == *"C-01 names no test"* && "$out" == *"C-02 names no test"* && "$out" == *"C-03 names no test"* ]] \
    && ok "  and reporting each empty row (owed (...), bare owed, —) without failing" || bad "empty report: $out"
[[ "$(run_code "$good" --strict)" == 1 ]] && ok "--strict fails the same empty row" || bad "--strict did not fail"
full="$(root full "| S21-01 | floods | S1 | $T | — |")"
[[ "$(run_code "$full" --strict)" == 0 ]] && ok "--strict passes a matrix with no empty row" || bad "--strict control failed: $(run "$full" --strict)"

expect_fail() {  # <label> <needle> <rows...>
    local label="$1" needle="$2"; shift 2
    local dir; dir="$(root "case$((pass + fail))" "$@")"
    local out code
    out="$(run "$dir")"; code="$(run_code "$dir")"
    if [[ "$code" == 1 && "$out" == *"$needle"* ]]; then ok "$label"; else bad "$label: exit $code, wanted 1 and '$needle': $out"; fi
}

expect_fail "a deleted test file fails" "does not exist" "| S21-01 | floods | S1 | \`tests/security/tests/gone.rs::a_guard\` | — |"
expect_fail "a renamed test fails" "defines no test fn renamed_guard" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::renamed_guard\` | — |"
expect_fail "a name only in a // comment does not vouch" "defines no test fn commented_out" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::commented_out\` | — |"
expect_fail "a name only in a /// doc comment does not vouch" "defines no test fn doc_only" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::doc_only\` | — |"
expect_fail "a helper without a test attribute does not vouch" "defines no test fn helper_guard" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::helper_guard\` | — |"
expect_fail "a name inside a string does not vouch" "defines no test fn string_guard" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::string_guard\` | — |"
expect_fail "a name in a trailing comment does not vouch" "defines no test fn trailing_guard" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::trailing_guard\` | — |"
expect_fail "a name in a // line between #[test] and another fn does not vouch" "defines no test fn swapped_guard" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::swapped_guard\` | — |"
expect_fail "a test disabled in a /* */ block does not vouch" "defines no test fn disabled_guard" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::disabled_guard\` | — |"
expect_fail "a test after a nested /* */ inside the block does not vouch" "defines no test fn nested_disabled_guard" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::nested_disabled_guard\` | — |"
expect_fail "a disabled test after a quote character literal does not vouch" "defines no test fn char_hidden_guard" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::char_hidden_guard\` | — |"
expect_fail "a test inside a raw string does not vouch" "defines no test fn raw_guard" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::raw_guard\` | — |"
expect_fail "a prefix of a real name does not vouch" "defines no test fn a_gu" "| S21-01 | floods | S1 | \`tests/security/tests/flood.rs::a_gu\` | — |"
expect_fail "a path outside the repository fails" "not a path inside the repository" "| S21-01 | floods | S1 | \`../../etc/passwd::root\` | — |"
expect_fail "a repeated id fails" "id S21-01 repeats" "| S21-01 | floods | S1 | $T | — |" "| S21-01 | again | S1 | $U | — |"
expect_fail "an id outside the three families fails" "is not S21-NN, TM-NN or C-NN" "| SX-01 | floods | S1 | $T | — |"
expect_fail "a row without five cells fails" "has 4 cells" "| S21-01 | floods | $T | — |"
expect_fail "an item that is neither path::fn nor owed fails" "neither \`path::fn\` nor an owed marker" "| S21-01 | floods | S1 | $T; see the plan | — |"
expect_fail "a matrix with no id row fails" "no row with an S21-NN" ""
for id in '`S21-01`' '**S21-01**' 'S21–01' 's21-01' 'S21-01a'; do
    expect_fail "a malformed id ($id) fails, its row not skipped" "id '$id' is not" "| $id | floods | S1 | $T | — |"
done

nomatrix="$TMP/nomatrix"; mkdir -p "$nomatrix"
out="$(run "$nomatrix")"; code="$(run_code "$nomatrix")"
[[ "$code" == 1 && "$out" == *"the security matrix the Stage 18 gate reads is not there"* && "$out" != *Traceback* ]] \
    && ok "an absent matrix fails by name, not by a crash" || bad "absent matrix: exit $code: $out"

echo "test_check_security_matrix: $pass passed, $fail failed"
[[ "$fail" == 0 ]]
