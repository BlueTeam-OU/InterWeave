#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/checks/check_no_grep_q_pipe.sh
#
# >>> help
# Does a tracked shell script that sets pipefail end a pipeline in a quiet
# grep?
#
#   tools/checks/check_no_grep_q_pipe.sh [--root <dir>]
#
# grep -q exits at its first match. When the command writing into the pipe
# still has more than the pipe buffer (64 KiB) to write, it dies of SIGPIPE
# (status 141), and under pipefail the pipeline's status is that 141: a
# match reads as a miss. Measured: `printf '%s' "$v" | grep -q PAT` with a
# 300 KB $v holding PAT on its first line missed 50 times in 50 under
# pipefail, and `grep -q PAT <<<"$v"` missed 0 in 50. The fixed form is to
# capture the output first and give grep a here-string or a file.
#
# WHAT IS ASKED: every git-tracked file named *.sh or *.bash (git ls-files,
# from the repository root, or from --root) with a non-comment line that
# sets pipefail. A pipeline whose LAST command is grep, egrep or fgrep
# (also by path, /usr/bin/grep, behind NAME=value assignments, command
# or env with their dash options, or a leading backslash) with a quiet flag -- -q, a cluster holding
# q such as -qxF or -Eqi, quoted or not, --quiet or --silent (-s is not
# quiet) -- is reported as path:line. The quiet flag is looked for among
# that grep's own arguments only, which end at ; && || | ) ]] } a
# whitespace-led # or <<<; a grep followed by another pipe is not last
# and is left alone. A pipe is | (also |&) outside single quotes; a line
# starting with | continues the line before it, and so does the next
# non-blank, non-comment line after one ending in |, |& or either and \.
#
# OUT OF SCOPE: third_party/ and spikes/ (spike code is empirical evidence),
# a pipe into grep without a quiet flag, a script without pipefail. This is
# a subset reader, not a bash parser: an exotic spelling (a pipe built in
# eval, a grep behind a function, alias, xargs or another wrapper, an
# option that takes an argument ahead of it (env -u NAME), an assignment
# whose quoted value holds a space, a -q on a continuation line after
# grep's own) can be missed.
# A tracked name git still quotes with core.quotePath off is refused.
#
# Exit codes:
#   0  no such pipeline
#   1  at least one, each named
#   2  usage, not in a git repository, a listing or read that failed,
#      or a name git quotes
# <<< help

set -uo pipefail

me="check_no_grep_q_pipe"
root=""
while [ $# -gt 0 ]; do
    case "$1" in
        -h|--help)
            sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
            exit 0 ;;
        --root)
            [ $# -ge 2 ] && [ -n "$2" ] || { echo "$me: --root needs a directory" >&2; exit 2; }
            root="$2"; shift 2 ;;
        *)
            echo "$me: unknown argument: $1" >&2; exit 2 ;;
    esac
done

if [ -z "$root" ]; then
    root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
fi
cd "$root" 2>/dev/null || { echo "$me: no such directory: $root" >&2; exit 2; }
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || { echo "$me: $root is not in a git repository" >&2; exit 2; }
top="$(git rev-parse --show-toplevel)" || exit 2
cd "$top" || exit 2

# Pass one (NR == FNR) only learns whether the file sets pipefail; pass two
# prints path:line for each finding. term is args_of's second result.
# shellcheck disable=SC2016  # awk source: no shell expansion is wanted
AWK_PROG='
function ltrim(s) { sub(/^[ \t]+/, "", s); return s }
function args_of(s,    i, c, n, q, out) {
    n = length(s); q = ""; out = ""; term = "end"
    for (i = 1; i <= n; i++) {
        c = substr(s, i, 1)
        if (q != "") { if (c == q) q = ""; out = out c; continue }
        if (c == "\047" || c == "\"") { q = c; out = out c; continue }
        if (c == ";" || c == ")" || c == "}") { term = "other"; break }
        if (c == "&" && substr(s, i + 1, 1) == "&") { term = "other"; break }
        if (c == "]" && substr(s, i + 1, 1) == "]") { term = "other"; break }
        if (c == "|") { term = (substr(s, i + 1, 1) == "|") ? "other" : "|"; break }
        if (c == "<" && substr(s, i, 3) == "<<<") { term = "other"; break }
        if (c == "#" && (i == 1 || substr(s, i - 1, 1) ~ /[ \t]/)) { term = "other"; break }
        out = out c
    }
    return out
}
function quiet(a,    n, t, i) {
    n = split(a, t, /[ \t]+/)
    for (i = 1; i <= n; i++) {
        gsub(/["\047]/, "", t[i])
        if (t[i] == "--") return 0
        if (t[i] == "--quiet" || t[i] == "--silent") return 1
        if (t[i] ~ /^-[A-Za-z0-9]*q/) return 1
    }
    return 0
}
function after_pipe(s,    a) {
    s = ltrim(s)
    # What runs grep without being grep: assignments ahead of it, the
    # command builtin and the env program with their dash options, and a
    # backslash that skips an alias.
    for (;;) {
        if (s ~ /^[A-Za-z_][A-Za-z0-9_]*=[^ \t]*[ \t]+/) sub(/^[^ \t]+[ \t]+/, "", s)
        else if (s ~ /^(command|env)([ \t]+-[^ \t]*)*[ \t]+/) sub(/^(command|env)([ \t]+-[^ \t]*)*[ \t]+/, "", s)
        else break
    }
    sub(/^\\/, "", s)
    if (s !~ /^([^ \t|;&()]*\/)?(e|f)?grep([ \t]|$)/) return 0
    sub(/^[^ \t]+/, "", s)
    a = args_of(s)
    if (term == "|") return 0
    return quiet(a)
}
NR == FNR {
    if ($0 !~ /^[ \t]*#/ && $0 ~ /(^|[;&( \t])set[ \t]+[^#]*pipefail/) has = 1
    next
}
FNR == 1 { pend = 0 }
!has { next }
{
    line = $0
    if (line ~ /^[ \t]*#/) next
    # A blank line between a trailing pipe and its command is still one
    # pipeline, so it leaves pend as it was.
    if (line ~ /^[ \t]*$/) next
    hit = 0
    if (pend && after_pipe(line)) hit = 1
    pend = 0
    n = length(line); q = ""
    for (i = 1; i <= n && !hit; i++) {
        c = substr(line, i, 1)
        if (q != "") { if (c == q) q = ""; continue }
        if (c == "\047") { q = c; continue }
        if (c == "#" && (i == 1 || substr(line, i - 1, 1) ~ /[ \t]/)) break
        if (c != "|") continue
        if (substr(line, i + 1, 1) == "|") { i++; continue }
        if (substr(line, i + 1, 1) == "&") i++
        if (after_pipe(substr(line, i + 1))) hit = 1
    }
    if (hit) print name ":" FNR ": " ltrim(line)
    else if (line ~ /\|&?[ \t]*(\\[ \t]*)?$/ && line !~ /\|\|[ \t]*(\\[ \t]*)?$/) pend = 1
}
'

# A listing or a read that fails is "could not look", never "found none".
# quotePath off lists a non-ASCII name as itself; a name git must still
# quote (a newline, a control character) is refused below, not skipped.
files="$(git -c core.quotePath=false ls-files -- '*.sh' '*.bash')" \
    || { echo "$me: git ls-files failed in $top -- not checked" >&2; exit 2; }

found=0
scanned=0
while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in
        \"*) echo "$me: git quotes the name $f -- not checked" >&2; exit 2 ;;
    esac
    case "$f" in
        third_party/*|spikes/*) continue ;;
    esac
    [ -f "$f" ] || continue
    scanned=$((scanned + 1))
    # ./ keeps a name holding = from being read as an awk assignment.
    out="$(awk -v name="$f" "$AWK_PROG" "./$f" "./$f")" \
        || { echo "$me: awk could not read $f -- not checked" >&2; exit 2; }
    [ -n "$out" ] || continue
    found=1
    while IFS= read -r l; do
        echo "$l"
        echo "  a quiet grep ends this pipeline under pipefail: its early exit SIGPIPEs the writer and turns a match into a miss; capture, then grep -q PAT <<<\"\$captured\""
    done <<<"$out"
done <<<"$files"

if [ "$found" -ne 0 ]; then
    exit 1
fi
echo "$me: ok -- no pipeline ends in a quiet grep in $scanned tracked shell scripts"
exit 0
