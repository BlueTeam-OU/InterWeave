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
# are #503's misses as they were made: the chain skipping 2.11, the
# history ending at 2.11, and the parenthetical still reading 1.1.0.

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
PREV=$((MAX - 1))
PEER_LIST="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["x-contract"]["version"])' "$REPO/$SCHEMAS/peer-list.schema.json")"

printf 'test_check_ipc_prose_versions\n'

echo "the tree's own inputs pass (the control)"
R="$(copy control)"
code="$(run_code "$R")"
if [ "$code" = 0 ]; then ok "unmutated copy exits 0"; else bad "control: want exit 0, got $code: $(run "$R")"; fi

echo "--help names the five rules and the two inputs"
help="$(python3 "$CHECK" --help 2>&1)"
for word in "IPC_MAX_MINOR" "x-contract.version" "1. highest-minor" "2. version history" \
            "3. method parenthetical" "4. no schema version above" "5. no minor above"; do
    case "$help" in *"$word"*) ok "help says '$word'" ;; *) bad "help lacks '$word'" ;; esac
done

echo "rule 1: the highest-minor sentence"
R="$(copy r1-number)"; F="$R/$CONTRACT"
edit "$F" "the server's highest minor, $MAX since" "the server's highest minor, $PREV since" || exit 1
expect_fail "$R" 1 "$(line_of "$F" "the server's highest minor,")" "the highest minor below IPC_MAX_MINOR"
R="$(copy r1-skip)"; F="$R/$CONTRACT"
# #503's first miss: the chain stepping from MAX straight past MAX-1.
edit "$F" "; $PREV since " "; $((MAX - 2)) since " || exit 1
edit "$F" "; $((MAX - 2)) since " "; " || exit 1
expect_fail "$R" 1 "$(line_of "$F" "the server's highest minor,")" "the chain skipping 2.$PREV"
R="$(copy r1-short)"; F="$R/$CONTRACT"
edit "$F" "; 3 since " "; three since " || exit 1
expect_fail "$R" 1 "$(line_of "$F" "the server's highest minor,")" "the chain stopping above 3"
R="$(copy r1-gone)"; F="$R/$CONTRACT"
edit "$F" "the server's highest minor, " "the server's top minor, " || exit 1
out="$(run "$R")"
case "$out" in *"rule 1:"*"found 0 times"*) ok "a vanished sentence fails rule 1" ;; *) bad "vanished sentence: $out" ;; esac

echo "rule 2: the version history"
R="$(copy r2-missing)"; F="$R/$CONTRACT"
# #503's second miss: the history ending a minor short.
edit "$F" "brought 2.$MAX" "added 2.$MAX" || exit 1
expect_fail "$R" 2 "$(grep -nF 'brought 2.' "$F" | tail -1 | cut -d: -f1)" "the history ending at 2.$PREV"
R="$(copy r2-extra)"; F="$R/$CONTRACT"
edit "$F" "brought 2.2" "brought 2.1" || exit 1
expect_fail "$R" 2 "$(line_of "$F" "brought 2.1")" "a history naming 2.1"

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
# The parenthetical's own minor bound. Rule 5 reports the same 2.N too;
# the case asserts rule 3's line, which only rule 3's branch prints.
edit "$F" "which 2.$MAX brings" "which 2.$((MAX + 1)) brings" || exit 1
expect_fail "$R" 3 "$(line_of "$F" "which 2.$((MAX + 1)) brings")" "the parenthetical naming a minor above the code"

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

echo "rule 5: no minor above the code"
R="$(copy r5)"; F="$R/$CONTRACT"
edit "$F" "## Version negotiation and phases" "## Version negotiation and phases (2.$((MAX + 1)))" || exit 1
expect_fail "$R" 5 "$(line_of "$F" "## Version negotiation and phases")" "a minor above IPC_MAX_MINOR"
R="$(copy r5-sentence)"; F="$R/$CONTRACT"
# A minor that ends a sentence is still a minor.
edit "$F" "## Version negotiation and phases" "## Version negotiation and phases, since 2.$((MAX + 1))." || exit 1
expect_fail "$R" 5 "$(line_of "$F" "## Version negotiation and phases")" "a minor above IPC_MAX_MINOR ending a sentence"
R="$(copy r5-dotted)"; F="$R/$CONTRACT"
# The lookarounds: 2.99.1 and 12.99 are not minors.
edit "$F" "## Version negotiation and phases" "## Version negotiation and phases 2.99.1 12.99" || exit 1
code="$(run_code "$R")"
if [ "$code" = 0 ]; then ok "a dotted version and a longer number are not minors"; else bad "lookarounds: $(run "$R")"; fi

echo "the constant is an input"
R="$(copy const-up)"
# Raising the code's minor without the prose fails rules 1 and 2.
edit "$R/$VERSION_RS" "IPC_MAX_MINOR: u64 = $MAX;" "IPC_MAX_MINOR: u64 = $((MAX + 1));" || exit 1
out="$(run "$R")"
case "$out" in *"rule 1:"*) ok "a raised IPC_MAX_MINOR fails rule 1" ;; *) bad "raised constant, rule 1: $out" ;; esac
case "$out" in *"rule 2:"*) ok "  and rule 2" ;; *) bad "raised constant, rule 2: $out" ;; esac
R="$(copy const-gone)"
edit "$R/$VERSION_RS" "pub const IPC_MAX_MINOR: u64" "pub const IPC_TOP_MINOR: u64" || exit 1
code="$(run_code "$R")"
if [ "$code" = 1 ]; then ok "no constant is a failure, not a pass"; else bad "no constant: want exit 1, got $code"; fi
R="$(copy contract-gone)"; rm "$R/$CONTRACT"
code="$(run_code "$R")"
if [ "$code" = 1 ]; then ok "no contract is a failure, not a pass"; else bad "no contract: want exit 1, got $code"; fi

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
