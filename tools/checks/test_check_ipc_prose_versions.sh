#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/test_check_ipc_prose_versions.sh
#
# Self-test for check_ipc_prose_versions.py. Each case copies the
# repository's own contract, version.rs and ipc schemas into a sandbox
# root, mutates one site, and asserts the check fails naming the rule and
# the line. The unmutated copy is the positive control, so a check that
# fails everything cannot pass this test, and the cases run on the text
# the check guards rather than on a shape written to suit it. Three cases
# are #503's misses as they were made: a chain skipping a minor, the
# history ending a minor short, and the parenthetical still reading 1.1.0.
# Major 2 is frozen at 13 (ADR-0067 decision 7), so its cases name 2.N
# literally; the live major's cases are written from IPC_MAJOR and
# IPC_MAX_MINOR as the tree holds them.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECK="$SCRIPT_DIR/check_ipc_prose_versions.py"
CONTRACT=architecture/contracts/LOCAL-IPC.md
VERSION_RS=crates/api/ipc-protocol/src/version.rs
SCHEMAS=architecture/contracts/schemas/ipc

pass=0
fail=0
ok()  { printf '  ✓ %s\n' "$1"; pass=$((pass + 1)); }
bad() { printf '  ✗ %s\n' "$1" >&2; fail=$((fail + 1)); }

TMP="$(mktemp -d)"
[ -n "$TMP" ] && [ -d "$TMP" ] || { echo "test_check_ipc_prose_versions: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$TMP"' EXIT

# Captured, never piped into `grep -q` -- pipefail plus an early-exiting
# grep turns correct output into a failed pipeline.
run()      { python3 "$CHECK" --root "$1" 2>&1; }
run_code() { python3 "$CHECK" --root "$1" >/dev/null 2>&1; printf '%s' "$?"; }

# copy <name>: a sandbox root holding the three inputs as the tree has them.
copy() {
    local dir="$TMP/$1"
    mkdir -p "$dir/$(dirname "$CONTRACT")" "$dir/$(dirname "$VERSION_RS")" "$dir/$SCHEMAS"
    cp "$REPO/$CONTRACT" "$dir/$CONTRACT"
    cp "$REPO/$VERSION_RS" "$dir/$VERSION_RS"
    cp "$REPO/$SCHEMAS"/*.json "$dir/$SCHEMAS/"
    printf '%s' "$dir"
}

# edit <file> <old> <new>: replace the first <old>, and fail the case
# loudly if <old> is absent -- a replace that matches nothing would leave
# the copy unmutated and the case would test the control.
edit() {
    python3 - "$1" "$2" "$3" <<'PYEOF'
import sys
path, old, new = sys.argv[1:]
text = open(path, encoding="utf-8").read()
if old not in text:
    sys.exit(f"edit: {old!r} not in {path}")
open(path, "w", encoding="utf-8").write(text.replace(old, new, 1))
PYEOF
}

# line_of <file> <fixed text>: the first line holding it.
line_of() { grep -nF -m1 -- "$2" "$1" | cut -d: -f1; }

# expect_fail <root> <rule> <line> <what>: exit 1, and a line naming
# the rule at that line of the contract.
expect_fail() {
    local root="$1" rule="$2" at="$3" what="$4" out code
    out="$(run "$root")"; code="$(run_code "$root")"
    if [ "$code" != 1 ]; then bad "$what: want exit 1, got $code: $out"; return; fi
    case "$out" in
        *"$CONTRACT:$at: rule $rule:"*) ok "$what (rule $rule, line $at)" ;;
        *) bad "$what: want '$CONTRACT:$at: rule $rule:' in: $out" ;;
    esac
}

MAX="$(sed -n 's/^pub const IPC_MAX_MINOR: u64 = \([0-9]*\);$/\1/p' "$REPO/$VERSION_RS")"
[ -n "$MAX" ] || { echo "test_check_ipc_prose_versions: no IPC_MAX_MINOR in $VERSION_RS" >&2; exit 1; }
MAJOR="$(sed -n 's/^pub const IPC_MAJOR: u64 = \([0-9]*\);$/\1/p' "$REPO/$VERSION_RS")"
[ -n "$MAJOR" ] || { echo "test_check_ipc_prose_versions: no IPC_MAJOR in $VERSION_RS" >&2; exit 1; }
HIGH="the server's highest minor on major $MAJOR, $MAX since "
PEER_LIST="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["x-contract"]["version"])' "$REPO/$SCHEMAS/peer-list.schema.json")"

printf 'test_check_ipc_prose_versions\n'

echo "the tree's own inputs pass (the control)"
R="$(copy control)"
code="$(run_code "$R")"
if [ "$code" = 0 ]; then ok "unmutated copy exits 0"; else bad "control: want exit 0, got $code: $(run "$R")"; fi

echo "--help names the five rules and the three inputs"
help="$(python3 "$CHECK" --help 2>&1)"
for word in "IPC_MAJOR" "IPC_MAX_MINOR" "x-contract.version" "frozen" "1. highest-minor" "2. version history" \
            "3. method parenthetical" "4. no schema version above" "5. no minor above"; do
    case "$help" in *"$word"*) ok "help says '$word'" ;; *) bad "help lacks '$word'" ;; esac
done

echo "rule 1: the highest-minor sentence"
R="$(copy r1-number)"; F="$R/$CONTRACT"
edit "$F" "$HIGH" "the server's highest minor on major $MAJOR, $((MAX + 1)) since " || exit 1
expect_fail "$R" 1 "$(line_of "$F" "the server's highest minor on")" "the highest minor not IPC_MAX_MINOR"
R="$(copy r1-major)"; F="$R/$CONTRACT"
edit "$F" "$HIGH" "the server's highest minor on major $((MAJOR + 1)), $MAX since " || exit 1
expect_fail "$R" 1 "$(line_of "$F" "the server's highest minor on")" "the sentence naming another major"
R="$(copy r1-gone)"; F="$R/$CONTRACT"
edit "$F" "$HIGH" "the server's top minor on major $MAJOR, $MAX since " || exit 1
out="$(run "$R")"
case "$out" in *"rule 1:"*"found 0 times"*) ok "a vanished sentence fails rule 1" ;; *) bad "vanished sentence: $out" ;; esac
R="$(copy r1-frozen-gone)"; F="$R/$CONTRACT"
edit "$F" "ended at minor 13 — " "ended with minor 13 — " || exit 1
out="$(run "$R")"
case "$out" in *"rule 1:"*"ended at minor N"*"found 0 times"*) ok "a vanished major-2 clause fails rule 1" ;; *) bad "vanished clause: $out" ;; esac
R="$(copy r1-frozen-end)"; F="$R/$CONTRACT"
# Major 2's end is frozen, whatever IPC_MAX_MINOR says; the chain after
# it is left at 13, so only the clause's own number can fail the case.
edit "$F" "ended at minor 13 — 13 since " "ended at minor 14 — 13 since " || exit 1
expect_fail "$R" 1 "$(line_of "$F" "ended at minor 14")" "major 2 ending anywhere but 13"
R="$(copy r1-frozen-skip)"; F="$R/$CONTRACT"
# #503's first miss, in the frozen chain: 13 straight to 11.
edit "$F" "; 12 since " "; twelve since " || exit 1
expect_fail "$R" 1 "$(line_of "$F" "ended at minor 13")" "the major-2 chain skipping 12"
R="$(copy r1-frozen-short)"; F="$R/$CONTRACT"
edit "$F" "; 3 since " "; three since " || exit 1
expect_fail "$R" 1 "$(line_of "$F" "ended at minor 13")" "the major-2 chain stopping above 3"
R="$(copy r1-live-skip)"; F="$R/$CONTRACT"
# The live chain on a later minor: two minors up, the middle reason missing.
edit "$R/$VERSION_RS" "IPC_MAX_MINOR: u64 = $MAX;" "IPC_MAX_MINOR: u64 = $((MAX + 2));" || exit 1
edit "$F" "$HIGH" "the server's highest minor on major $MAJOR, $((MAX + 2)) since a later row; $MAX since " || exit 1
expect_fail "$R" 1 "$(line_of "$F" "the server's highest minor on")" "the major-$MAJOR chain skipping $MAJOR.$((MAX + 1))"
R="$(copy r1-live-grows)"; F="$R/$CONTRACT"
# The positive case for growth: the next minor, its reason and its history.
edit "$R/$VERSION_RS" "IPC_MAX_MINOR: u64 = $MAX;" "IPC_MAX_MINOR: u64 = $((MAX + 1));" || exit 1
edit "$F" "$HIGH" "the server's highest minor on major $MAJOR, $((MAX + 1)) since a later row; $MAX since " || exit 1
edit "$F" "brought 2.13" "brought 2.13, brought $MAJOR.$((MAX + 1))" || exit 1
code="$(run_code "$R")"
if [ "$code" = 0 ]; then ok "the next minor of major $MAJOR, written in, passes"; else bad "growth: $(run "$R")"; fi

echo "rule 2: the version history"
R="$(copy r2-missing)"; F="$R/$CONTRACT"
# #503's second miss: the history ending a minor short.
edit "$F" "brought 2.13" "added 2.13" || exit 1
expect_fail "$R" 2 "$(grep -nF 'brought 2.' "$F" | tail -1 | cut -d: -f1)" "the major-2 history ending at 2.12"
R="$(copy r2-extra)"; F="$R/$CONTRACT"
edit "$F" "brought 2.2" "brought 2.1" || exit 1
expect_fail "$R" 2 "$(line_of "$F" "brought 2.1")" "a history naming 2.1"
R="$(copy r2-live-ahead)"; F="$R/$CONTRACT"
edit "$F" "brought 2.13" "brought 2.13, brought $MAJOR.$((MAX + 1))" || exit 1
expect_fail "$R" 2 "$(line_of "$F" "brought $MAJOR.$((MAX + 1))")" "a history naming a minor above IPC_MAX_MINOR"
R="$(copy r2-live-missing)"; F="$R/$CONTRACT"
# The code moves a minor and the history list does not (rule 1 fails too).
edit "$R/$VERSION_RS" "IPC_MAX_MINOR: u64 = $MAX;" "IPC_MAX_MINOR: u64 = $((MAX + 1));" || exit 1
out="$(run "$R")"
case "$out" in *"rule 2: no \"brought $MAJOR.$((MAX + 1))\""*) ok "a minor missing from the major-$MAJOR history fails rule 2" ;;
    *) bad "live history: $out" ;; esac
R="$(copy r2-other-major)"; F="$R/$CONTRACT"
edit "$F" "brought 2.13" "brought 2.13, brought $((MAJOR + 1)).0" || exit 1
expect_fail "$R" 2 "$(line_of "$F" "brought $((MAJOR + 1)).0")" "a history naming a major beyond IPC_MAJOR"

echo "rule 3: the method parenthetical"
R="$(copy r3-stale)"; F="$R/$CONTRACT"
# #503's third miss (F5): the parenthetical a version behind its schema.
edit "$F" "\`peer-list\` ($PEER_LIST since" "\`peer-list\` (1.1.0 since" || exit 1
expect_fail "$R" 3 "$(line_of "$F" "\`peer-list\` (1.1.0 since")" "peer-list's claim behind its schema"
R="$(copy r3-schema)"; F="$R/$CONTRACT"
# The other direction: the schema moves and the prose does not.
edit "$R/$SCHEMAS/peer-list.schema.json" "\"version\": \"$PEER_LIST\"" '"version": "9.0.0"' || exit 1
expect_fail "$R" 3 "$(line_of "$F" "\`peer-list\` ($PEER_LIST since")" "the schema bumped under unchanged prose"
R="$(copy r3-history)"; F="$R/$CONTRACT"
# A later clause of the parenthetical is history: an old version there passes.
edit "$F" "\`peer-list\` ($PEER_LIST since" "\`peer-list\` ($PEER_LIST since 1.0.0;" || exit 1
code="$(run_code "$R")"
if [ "$code" = 0 ]; then ok "an older version later in the parenthetical passes"; else bad "history clause: $(run "$R")"; fi

R="$(copy r3-minor)"; F="$R/$CONTRACT"
# The parenthetical's own minor bound. Rule 5 reports the same N.N too;
# each case asserts rule 3's line, which only rule 3's branch prints.
# A major-2 parenthetical is bounded by 13, a live one by IPC_MAX_MINOR.
NAMED="$(grep -o 'which 2\.[0-9]* brings' "$F" | sed 's/[^0-9]*2\.\([0-9]*\).*/\1/' | sort -n | tail -1)"
[ -n "$NAMED" ] || { bad "no method parenthetical names a minor"; exit 1; }
edit "$F" "which 2.$NAMED brings" "which 2.14 brings" || exit 1
expect_fail "$R" 3 "$(line_of "$F" "which 2.14 brings")" "the parenthetical naming 2.14"
R="$(copy r3-live)"; F="$R/$CONTRACT"
edit "$F" "which 2.$NAMED brings" "which $MAJOR.$((MAX + 1)) brings" || exit 1
expect_fail "$R" 3 "$(line_of "$F" "which $MAJOR.$((MAX + 1)) brings")" "the parenthetical naming a minor above IPC_MAX_MINOR"
R="$(copy r3-other)"; F="$R/$CONTRACT"
edit "$F" "which 2.$NAMED brings" "which $((MAJOR + 1)).0 brings" || exit 1
expect_fail "$R" 3 "$(line_of "$F" "which $((MAJOR + 1)).0 brings")" "the parenthetical naming another major"

R="$(copy r3-noversion)"; F="$R/$CONTRACT"
# The parenthetical's schema with no readable version fails rule 3 at it.
edit "$R/$SCHEMAS/peer-list.schema.json" "\"version\": \"$PEER_LIST\"" '"revision": "0"' || exit 1
expect_fail "$R" 3 "$(line_of "$F" "\`peer-list\` ($PEER_LIST since")" "the parenthetical's schema with no version"

echo "rule 4: no schema version above its file"
R="$(copy r4-above)"; F="$R/$CONTRACT"
edit "$F" "\`ipc/hello\` 1.2.0" "\`ipc/hello\` 1.10.0" || exit 1
expect_fail "$R" 4 "$(line_of "$F" "\`ipc/hello\` 1.10.0")" "ipc/hello named above its file (semver, not text, order)"
R="$(copy r4-name)"; F="$R/$CONTRACT"
edit "$F" "\`ipc/hello\` 1.2.0" "\`ipc/no-such\` 1.0.0" || exit 1
expect_fail "$R" 4 "$(line_of "$F" "\`ipc/no-such\` 1.0.0")" "a name with no schema file"

R="$(copy r4-noversion)"; F="$R/$CONTRACT"
# A schema whose version cannot be read fails the rule that needed it.
edit "$R/$SCHEMAS/hello.schema.json" '"version": "1.2.0"' '"revision": "1.2.0"' || exit 1
expect_fail "$R" 4 "$(line_of "$F" "\`ipc/hello\` 1.2.0")" "a schema with no x-contract.version"
R="$(copy r4-notjson)"; F="$R/$CONTRACT"
printf '{ not json\n' > "$R/$SCHEMAS/hello.schema.json"
expect_fail "$R" 4 "$(line_of "$F" "\`ipc/hello\` 1.2.0")" "a schema that is not JSON"

echo "rule 5: no minor above its major's end"
R="$(copy r5)"; F="$R/$CONTRACT"
edit "$F" "## Version negotiation and phases" "## Version negotiation and phases (2.14)" || exit 1
expect_fail "$R" 5 "$(line_of "$F" "## Version negotiation and phases")" "2.14, above major 2's end"
R="$(copy r5-sentence)"; F="$R/$CONTRACT"
# A minor that ends a sentence is still a minor.
edit "$F" "## Version negotiation and phases" "## Version negotiation and phases, since 2.14." || exit 1
expect_fail "$R" 5 "$(line_of "$F" "## Version negotiation and phases")" "2.14 ending a sentence"
R="$(copy r5-live)"; F="$R/$CONTRACT"
edit "$F" "## Version negotiation and phases" "## Version negotiation and phases ($MAJOR.$((MAX + 1)))" || exit 1
expect_fail "$R" 5 "$(line_of "$F" "## Version negotiation and phases")" "a major-$MAJOR minor above IPC_MAX_MINOR"
R="$(copy r5-later)"; F="$R/$CONTRACT"
# A forward rule names a minor to come; it is not read.
edit "$F" "## Version negotiation and phases" "## Version negotiation and phases (from $MAJOR.$((MAX + 1)) or later)" || exit 1
code="$(run_code "$R")"
if [ "$code" = 0 ]; then ok "\"$MAJOR.$((MAX + 1)) or later\" is not a minor claimed"; else bad "or later: $(run "$R")"; fi
R="$(copy r5-history)"; F="$R/$CONTRACT"
# Major 2's minors are history, valid up to 2.13 whatever IPC_MAX_MINOR is.
edit "$F" "## Version negotiation and phases" "## Version negotiation and phases (2.13)" || exit 1
code="$(run_code "$R")"
if [ "$code" = 0 ]; then ok "2.13 is history and passes"; else bad "2.13: $(run "$R")"; fi
R="$(copy r5-dotted)"; F="$R/$CONTRACT"
# The lookarounds: 2.99.1 and 12.99 are not minors.
edit "$F" "## Version negotiation and phases" "## Version negotiation and phases 2.99.1 12.99" || exit 1
code="$(run_code "$R")"
if [ "$code" = 0 ]; then ok "a dotted version and a longer number are not minors"; else bad "lookarounds: $(run "$R")"; fi

echo "the constants are inputs"
R="$(copy const-up)"
# Raising the code's minor without the prose fails rules 1 and 2.
edit "$R/$VERSION_RS" "IPC_MAX_MINOR: u64 = $MAX;" "IPC_MAX_MINOR: u64 = $((MAX + 1));" || exit 1
out="$(run "$R")"
case "$out" in *"rule 1:"*) ok "a raised IPC_MAX_MINOR fails rule 1" ;; *) bad "raised constant, rule 1: $out" ;; esac
case "$out" in *"rule 2:"*) ok "  and rule 2" ;; *) bad "raised constant, rule 2: $out" ;; esac
R="$(copy major-up)"
# A major the check does not model fails rather than checking old bounds.
edit "$R/$VERSION_RS" "IPC_MAJOR: u64 = $MAJOR;" "IPC_MAJOR: u64 = $((MAJOR + 1));" || exit 1
out="$(run "$R")"; code="$(run_code "$R")"
case "$code:$out" in 1:*"rule 1: IPC_MAJOR is $((MAJOR + 1))"*) ok "an unmodelled IPC_MAJOR fails rule 1" ;; *) bad "major up: $code: $out" ;; esac
for c in IPC_MAX_MINOR IPC_MAJOR; do
    R="$(copy "const-gone-$c")"
    edit "$R/$VERSION_RS" "pub const $c: u64" "pub const ${c}_X: u64" || exit 1
    code="$(run_code "$R")"
    if [ "$code" = 1 ]; then ok "no $c is a failure, not a pass"; else bad "no $c: want exit 1, got $code"; fi
done
R="$(copy contract-gone)"; rm "$R/$CONTRACT"
code="$(run_code "$R")"
if [ "$code" = 1 ]; then ok "no contract is a failure, not a pass"; else bad "no contract: want exit 1, got $code"; fi

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
