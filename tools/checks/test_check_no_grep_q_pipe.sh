#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/test_check_no_grep_q_pipe.sh
#
# Self-test for check_no_grep_q_pipe.sh. Builds throwaway git repositories
# and asserts the guard's verdict on each shape. It never pipes into a
# quiet grep itself: the guard scans this file too.

set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUARD="${GUARD:-$HERE/check_no_grep_q_pipe.sh}"
TMP="$(mktemp -d)"
[ -n "$TMP" ] && [ -d "$TMP" ] || { echo "test_check_no_grep_q_pipe: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$TMP"' EXIT

fails=0
ok()  { printf '  \342\234\223 %s\n' "$1"; }
bad() { printf '  \342\234\227 %s\n' "$1"; fails=$((fails + 1)); }

n=0
# fixture <relative path> <body> [untracked]: a fresh repo holding one script.
fixture() {
  n=$((n + 1))
  local dir="$TMP/r$n" rel="$1" body="$2"
  mkdir -p "$dir/$(dirname "$rel")"
  git -C "$dir" init -q
  printf '%s\n' "$body" > "$dir/$rel"
  [ "${3:-}" = untracked ] || git -C "$dir" add -A
  FIX="$dir"
}

HDR=$'#!/usr/bin/env bash\nset -euo pipefail'

# expect <name> <want exit> <relative path> <body> [untracked]
expect() {
  local name="$1" want="$2"
  fixture "$3" "$4" "${5:-}"
  local out got
  out="$(bash "$GUARD" --root "$FIX" 2>&1)"
  got=$?
  if [ "$got" -eq "$want" ]; then ok "$name"; else bad "$name (exit $got, wanted $want): $out"; fi
}

echo "check_no_grep_q_pipe: flagged shapes"
expect "printf into grep -q"        1 a.sh "$HDR"$'\nprintf \'%s\' "$v" | grep -q PAT'
expect "grep -qxF cluster"          1 a.sh "$HDR"$'\ncmd | grep -qxF PAT'
expect "grep -Eqi cluster"          1 a.sh "$HDR"$'\ncmd | grep -Eqi PAT'
expect "--quiet"                    1 a.sh "$HDR"$'\ncmd | grep --quiet PAT'
expect "--silent"                   1 a.sh "$HDR"$'\ncmd | grep --silent PAT'
expect "continuation line"          1 a.sh "$HDR"$'\ncmd \\\n  | grep -q PAT'
expect "trailing pipe then grep"    1 a.sh "$HDR"$'\ncmd |\n  grep -q PAT'
expect "grep by path"               1 a.sh "$HDR"$'\ncmd | /usr/bin/grep -q PAT'
expect "egrep"                      1 a.sh "$HDR"$'\ncmd | egrep -q PAT'
expect "fgrep"                      1 a.sh "$HDR"$'\ncmd | fgrep -q PAT'
expect "|& into grep -q"            1 a.sh "$HDR"$'\ncmd |& grep -q PAT'
expect "inside an if"               1 a.sh "$HDR"$'\nif cmd | grep -q PAT; then echo y; fi'
expect "inside a command sub"       1 a.sh "$HDR"$'\nx=$(cmd | grep -q PAT && echo y)'
expect "second pipe on the line"    1 a.sh "$HDR"$'\ncmd | tr a b | grep -q PAT'
expect "set -o pipefail spelling"   1 a.sh $'#!/bin/bash\nset -o pipefail\ncmd | grep -q PAT'
expect "set -uo pipefail spelling"  1 a.sh $'#!/bin/bash\nset -uo pipefail\ncmd | grep -q PAT'
expect ".bash extension"            1 a.bash "$HDR"$'\ncmd | grep -q PAT'
expect "nested directory"           1 tools/x/a.sh "$HDR"$'\ncmd | grep -q PAT'
expect "options before -q"          1 a.sh "$HDR"$'\ncmd | grep -E -q PAT'
expect "trailing | \\ then grep"    1 a.sh "$HDR"$'\ncmd | \\\n  grep -q PAT'
expect "blank line after a trailing pipe" 1 a.sh "$HDR"$'\ncmd |\n\n  grep -q PAT'
expect "blank and comment after | \\" 1 a.sh "$HDR"$'\ncmd | \\\n\n  # why\n  grep -q PAT'
expect "an assignment ahead of grep" 1 a.sh "$HDR"$'\ncmd | LC_ALL=C grep -q PAT'
expect "two assignments ahead of grep" 1 a.sh "$HDR"$'\ncmd | A=1 B= grep -q PAT'
expect "command grep"               1 a.sh "$HDR"$'\ncmd | command grep -q PAT'
expect "env grep"                   1 a.sh "$HDR"$'\ncmd | env LC_ALL=C grep -q PAT'
expect "backslash grep"             1 a.sh "$HDR"$'\ncmd | \\grep -q PAT'
expect "a quoted -q"                1 a.sh "$HDR"$'\ncmd | grep "-q" PAT'

echo "check_no_grep_q_pipe: passing shapes"
expect "here-string"                0 a.sh "$HDR"$'\ngrep -q PAT <<<"$x"'
expect "file argument"              0 a.sh "$HDR"$'\ngrep -q PAT file'
expect "|| before grep"             0 a.sh "$HDR"$'\ncmd || grep -q PAT file'
expect "comment line"               0 a.sh "$HDR"$'\n  # cmd | grep -q PAT'
expect "trailing comment"           0 a.sh "$HDR"$'\ncmd | grep PAT # grep -q'
expect "-c then -eq after pipe"     0 a.sh "$HDR"$'\n[ "$(cmd | grep -c .)" -eq 1 ]'
expect "-s is not quiet"            0 a.sh "$HDR"$'\ncmd | grep -s PAT'
expect "trailing || \\ then grep"   0 a.sh "$HDR"$'\ncmd || \\\n  grep -q PAT f'
expect "blank line, then a quiet grep with no pipe" 0 a.sh "$HDR"$'\ncmd | tee f\n\ngrep -q PAT f'
expect "an assignment ahead of grep, no pipe" 0 a.sh "$HDR"$'\nLC_ALL=C grep -q PAT f'
expect "command, then not grep"     0 a.sh "$HDR"$'\ncmd | command cat'
expect "grep without -q"            0 a.sh "$HDR"$'\ncmd | grep PAT >/dev/null'
expect "grep -q is not last"        0 a.sh "$HDR"$'\ncmd | grep -q PAT | cat'
expect "-q after &&"                0 a.sh "$HDR"$'\ncmd | grep PAT && grep -q X f'
expect "-q after ]] ends the arguments"  0 a.sh "$HDR"$'\ncmd | grep PAT ]] -q'
expect "-q after a here-string"     0 a.sh "$HDR"$'\ncmd | grep PAT <<<"$x" -q'
expect "-q as a pattern after --"   0 a.sh "$HDR"$'\ncmd | grep -- -q'
expect "pipe inside a trailing comment" 0 a.sh "$HDR"$'\ncmd # x | grep -q PAT'
expect "indented comment, no other cover" 0 a.sh "$HDR"$'\n\t# a | grep -q PAT'
expect "-q in a later command"      0 a.sh "$HDR"$'\ncmd | grep PAT; echo -q'
sqbody="grep -E 'a|grep -q' f"
expect "pipe inside single quotes"  0 a.sh "$HDR"$'\n'"$sqbody"
expect "no pipefail"                0 a.sh $'#!/bin/bash\nset -eu\ncmd | grep -q PAT'
expect "pipefail only in comment"   0 a.sh $'#!/bin/bash\n# set -o pipefail\ncmd | grep -q PAT'
expect "under spikes/"              0 spikes/a.sh "$HDR"$'\ncmd | grep -q PAT'
expect "under third_party/"         0 third_party/x/a.sh "$HDR"$'\ncmd | grep -q PAT'
expect "untracked file"             0 a.sh "$HDR"$'\ncmd | grep -q PAT' untracked
expect "not a shell file"           0 a.txt "$HDR"$'\ncmd | grep -q PAT'

echo "check_no_grep_q_pipe: a finding names path:line"
fixture dir/a.sh "$HDR"$'\ntrue\n  cmd | grep -q PAT'
out="$(bash "$GUARD" --root "$FIX" 2>&1)"
want='dir/a.sh:4: cmd | grep -q PAT'
if [[ "$out" == *"$want"* ]]; then ok "path:line and the trimmed line"; else bad "finding text: $out"; fi
fixture a.sh "$HDR"$'\ncmd \\\n  | grep -q PAT'
out="$(bash "$GUARD" --root "$FIX" 2>&1)"
want='a.sh:4: | grep -q PAT'
if [[ "$out" == *"$want"* ]]; then ok "a continuation names its own line"; else bad "continuation text: $out"; fi
fixture a.sh "$HDR"$'\ncmd | grep PAT'
out="$(bash "$GUARD" --root "$FIX" 2>&1)"
if [[ "$out" == *"ok"* ]]; then ok "a clean tree says ok"; else bad "clean text: $out"; fi

echo "check_no_grep_q_pipe: exit codes and help"
bash "$GUARD" --bogus >/dev/null 2>&1; got=$?
if [ "$got" -eq 2 ]; then ok "unknown argument is 2"; else bad "unknown argument (exit $got)"; fi
bash "$GUARD" --root >/dev/null 2>&1; got=$?
if [ "$got" -eq 2 ]; then ok "--root without a value is 2"; else bad "--root alone (exit $got)"; fi
mkdir -p "$TMP/notgit"
bash "$GUARD" --root "$TMP/notgit" >/dev/null 2>&1; got=$?
if [ "$got" -eq 2 ]; then ok "not a git repository is 2"; else bad "not a repo (exit $got)"; fi
bash "$GUARD" --root "$TMP/absent" >/dev/null 2>&1; got=$?
if [ "$got" -eq 2 ]; then ok "missing directory is 2"; else bad "missing dir (exit $got)"; fi
bash "$GUARD" --root "" >/dev/null 2>&1; got=$?
if [ "$got" -eq 2 ]; then ok "an empty --root is 2"; else bad "empty --root (exit $got)"; fi

echo "check_no_grep_q_pipe: could not look is 2, never ok"
# A listing that fails: the index is garbage, while rev-parse still
# answers that this is a work tree.
fixture a.sh "$HDR"$'\ncmd | grep PAT'
printf 'garbage' > "$FIX/.git/index"
out="$(bash "$GUARD" --root "$FIX" 2>&1)"; got=$?
if [ "$got" -eq 2 ]; then ok "a failed git ls-files is 2"; else bad "corrupt index (exit $got): $out"; fi
# A file awk cannot read. root reads anything, so this case needs a
# login that permissions bind.
if [ "$(id -u)" -ne 0 ]; then
  fixture a.sh "$HDR"$'\ncmd | grep PAT'
  chmod 000 "$FIX/a.sh"
  out="$(bash "$GUARD" --root "$FIX" 2>&1)"; got=$?
  chmod 644 "$FIX/a.sh"
  if [ "$got" -eq 2 ]; then ok "an unreadable script is 2"; else bad "unreadable script (exit $got): $out"; fi
else
  echo "  - an unreadable script: skipped as root"
fi
help="$(bash "$GUARD" --help 2>&1)"; got=$?
if [ "$got" -eq 0 ] && [[ "$help" == *"SIGPIPE"* ]] && [[ "$help" != *">>> help"* ]]; then ok "--help prints the block"; else bad "--help (exit $got)"; fi

echo
if [ "$fails" -eq 0 ]; then echo "check_no_grep_q_pipe: all assertions passed"; exit 0; fi
echo "check_no_grep_q_pipe: $fails assertion(s) failed" >&2
exit 1
