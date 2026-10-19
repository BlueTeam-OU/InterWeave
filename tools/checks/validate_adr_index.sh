#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/validate_adr_index.sh
#
# >>> help
# Prove the ADR set is navigable: every ADR follows the template, and the
# index and digest that route readers to it are complete in both
# directions.
#
#   tools/checks/validate_adr_index.sh
#   tools/checks/validate_adr_index.sh --root <dir>
#
# The digest (ADR-DIGEST.md) is what a session loads INSTEAD of reading
# every ADR in the directory, so an ADR missing from it is invisible — the
# failure mode is silent and the reader never learns what they did not
# see. Propagation is therefore checked mechanically, per ADR-0048.
#
# No count in that sentence: the one it used to give went stale, its
# replacement was wrong by one, and the success line prints the real
# count at runtime.
#
# Checks:
#   TEMPLATE   every NNNN-*.md has a `# Title` first line that does not
#              re-state its own number, a `**Status:**` line, and all
#              eight mandatory sections in order. Extra sections are
#              allowed; missing ones are not.
#   INDEX      every ADR has a row in README.md, and every README row
#              points at a file that exists.
#   DIGEST     every ADR has a `### NNNN — ` entry in ADR-DIGEST.md, and
#              every digest entry names an ADR that exists.
#   KEYWORDS   every **NNNN** reference in the digest's keyword table
#              resolves to a real ADR.
#   HISTORY    an ADR with an `## Amendments` section has a history file
#              at history/NNNN-amendments.md, and vice versa (ADR-0048).
#   AMENDED    an ADR whose `## Amendments` table gained a (date, title)
#              key since the base revision has a `### NNNN` digest entry
#              that differs from the base's. The DIGEST check only proves
#              an entry EXISTS, so an amendment whose digest bullet was
#              never written passed it while the entry went on describing
#              the record as it stood before (InterWeave #301, ADR-0041).
#
# What AMENDED does not prove: that the entry's new text is about the
# amendment, or that it is right — any edit to the entry satisfies it.
# It is a forcing function for the omission, not a reader of prose. Nor
# does it judge the lag that predates its base: the digest's entries
# cite amendments in several forms, or fold them in with no date at all
# (ADR-0048 asks for a current-state summary, not a citation), so no
# count of dated mentions is a rule the corpus follows.
#
# Options:
#   --root <dir>   validate this repository instead of the one
#                  containing this script
#   --base <rev>   the revision AMENDED compares against; by default the
#                  merge-base of HEAD and origin/main, which in CI is the
#                  pull request's or the queue's base. On a push to main
#                  it is HEAD itself, so that run compares nothing: the
#                  queue run before it compared the same change, and the
#                  ruleset admits no push that skipped the queue. `--base none`
#                  skips AMENDED, and the success line says it did — for
#                  a tree that is not a repository. With no base given
#                  and none resolvable the script exits 2: a check that
#                  cannot find its base is not a check that passed.
#   -h, --help     this text
#
# Exit codes:
#   0  everything propagated
#   1  one or more gaps found
#   2  invocation problem (expected paths missing, no base resolvable)
# <<< help

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd -- "$SCRIPT_DIR/../.." && pwd )"
BASE=""

show_help() {
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed -e '1d' -e '$d' -e 's/^# \{0,1\}//'
}

while [ $# -gt 0 ]; do
    case "$1" in
        -h|--help) show_help; exit 0 ;;
        --root)    [ $# -ge 2 ] || { echo "--root needs a value" >&2; exit 2; }
                   REPO_ROOT="$2"; shift 2 ;;
        --base)    [ $# -ge 2 ] || { echo "--base needs a value" >&2; exit 2; }
                   BASE="$2"; shift 2 ;;
        *)         echo "validate_adr_index: unexpected argument: $1" >&2; exit 2 ;;
    esac
done

ADR_DIR="$REPO_ROOT/architecture/adr"
README="$ADR_DIR/README.md"
DIGEST="$ADR_DIR/ADR-DIGEST.md"

for p in "$ADR_DIR" "$README" "$DIGEST"; do
    [ -e "$p" ] || { echo "validate_adr_index: expected path not found: $p" >&2; exit 2; }
done

# The base AMENDED compares against, resolved to a commit before any
# check runs, so a bad --base is an invocation problem and never a pass.
if [ "$BASE" = "none" ]; then
    base_sha=""
elif [ -n "$BASE" ]; then
    base_sha="$(git -C "$REPO_ROOT" rev-parse --verify --quiet "$BASE^{commit}")" \
        || { echo "validate_adr_index: --base $BASE does not resolve to a commit in $REPO_ROOT" >&2; exit 2; }
else
    base_sha="$(git -C "$REPO_ROOT" merge-base HEAD origin/main 2>/dev/null)" \
        || { echo "validate_adr_index: no base for AMENDED — origin/main does not resolve in $REPO_ROOT (fetch it, pass --base <rev>, or --base none to skip)" >&2; exit 2; }
fi

# The mandatory section set, in order. Kept here rather than in the
# template file so the check does not depend on parsing prose.
MANDATORY=(
    "## Context"
    "## Decision"
    "## Alternatives considered"
    "## Consequences"
    "## Security implications"
    "## Operational implications"
    "## Implementation implications"
    "## Revisit conditions"
)

gaps=0
report() { printf '%s\n' "$1"; gaps=$((gaps + 1)); }

# amendment_keys: an ADR on stdin -> its `## Amendments` table's
# "date — title" keys, one per line, sorted. The (date, title) pair is
# an amendment's identity (ADR-0048); the effect column is not part of it.
amendment_keys() {
    awk '/^## Amendments$/{f=1;next} f && /^\| [0-9]{4}-[0-9]{2}-[0-9]{2} /{print}' \
        | awk -F'|' '{gsub(/^[ \t]+|[ \t]+$/,"",$2); gsub(/^[ \t]+|[ \t]+$/,"",$3); print $2" — "$3}' | sort
}

# digest_entry <num>: the digest on stdin -> the `### <num> — ` entry,
# heading through the line before the next entry, cluster heading or
# rule.
digest_entry() {
    awk -v h="### $1 — " 'index($0,h)==1{f=1;print;next} f && (/^### / || /^## / || /^---$/){exit} f{print}'
}

amended=0
if [ -n "$base_sha" ]; then
    # `./` makes every base path relative to --root rather than to the
    # repository's top level, which differ when --root is a subdirectory.
    base_digest="$(git -C "$REPO_ROOT" show "$base_sha:./architecture/adr/ADR-DIGEST.md" 2>/dev/null)" \
        || { echo "validate_adr_index: no architecture/adr/ADR-DIGEST.md at base ${base_sha:0:8} — every entry would compare as changed; pass a later --base" >&2; exit 2; }
    base_names="$(git -C "$REPO_ROOT" ls-tree --name-only "$base_sha" ./architecture/adr/ | sed 's|.*/||')"
fi

mapfile -t ADR_FILES < <(find "$ADR_DIR" -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.md' -printf '%f\n' | sort)
[ "${#ADR_FILES[@]}" -gt 0 ] && [ -n "${ADR_FILES[0]}" ] \
    || { echo "validate_adr_index: no ADR files found in $ADR_DIR" >&2; exit 2; }

readme_body="$(cat "$README")"
digest_body="$(cat "$DIGEST")"

for f in "${ADR_FILES[@]}"; do
    num="${f%%-*}"
    path="$ADR_DIR/$f"

    # --- TEMPLATE -------------------------------------------------------
    first="$(head -1 "$path")"
    case "$first" in
        "# "*) ;;
        *) report "$f: first line is not a '# Title' heading" ;;
    esac
    case "$first" in
        *ADR-[0-9]*) report "$f: title re-states its own number — the number is the filename" ;;
    esac
    head -6 "$path" | grep -q '^\*\*Status:\*\*' \
        || report "$f: no '**Status:**' line in the opening lines"

    last_pos=0
    for section in "${MANDATORY[@]}"; do
        pos="$(grep -n -F -x "$section" "$path" | head -1 | cut -d: -f1)"
        if [ -z "$pos" ]; then
            report "$f: missing mandatory section '$section'"
        elif [ "$pos" -lt "$last_pos" ]; then
            report "$f: section '$section' is out of template order"
        else
            last_pos="$pos"
        fi
    done

    # --- INDEX ----------------------------------------------------------
    case "$readme_body" in
        *"(./$f)"*) ;;
        *) report "$f: no row in README.md — a reader browsing the index cannot see it" ;;
    esac

    # --- DIGEST ---------------------------------------------------------
    case "$digest_body" in
        *"### $num — "*) ;;
        *) report "$f: no '### $num — ' entry in ADR-DIGEST.md — invisible to a digest-first session" ;;
    esac

    # --- AMENDMENT RECORD -----------------------------------------------
    # Any amendment-ish heading other than the end-matter table is the
    # pre-ADR-0048 convention: the note belongs in history/, and the text
    # itself belongs folded into the section it qualifies.
    while IFS= read -r stray; do
        [ -n "$stray" ] && report "$f: '$stray' — fold the text into its section and record the note in history/ (ADR-0048)"
    done < <(grep '^## .*[Aa]mendment' "$path" | grep -v '^## Amendments$')

    hist="$ADR_DIR/history/$num-amendments.md"
    has_table=0
    grep -q '^## Amendments$' "$path" && has_table=1

    if [ "$has_table" -eq 1 ] && [ ! -f "$hist" ]; then
        report "$f: has an '## Amendments' table but no history/$num-amendments.md"
    elif [ "$has_table" -eq 0 ] && [ -f "$hist" ]; then
        report "$f: history/$num-amendments.md exists but the ADR has no '## Amendments' table"
    elif [ "$has_table" -eq 1 ]; then
        # `## Amendments` is end-matter: nothing may follow it.
        after="$(awk '/^## Amendments$/{f=1;next} f && /^## /{print; exit}' "$path")"
        [ -n "$after" ] && report "$f: '## Amendments' is not the last section — '$after' follows it"

        # (date, title) pairs must match between the table and the notes,
        # as multisets. A row without a note is a citation that resolves
        # to nothing; a note without a row is unreachable from the ADR.
        tbl="$(amendment_keys < "$path")"
        notes="$(grep '^### Amendment ' "$hist" | sed 's/^### Amendment //' | sort)"
        if [ "$tbl" != "$notes" ]; then
            report "$f: amendment (date, title) keys differ between the table and history/$num-amendments.md"
            diff <(printf '%s\n' "$tbl") <(printf '%s\n' "$notes") 2>/dev/null \
                | sed 's/^/      /' | head -6
        fi

        # --- AMENDED ----------------------------------------------------
        # The base's copy is found by NUMBER, so a rename in the same
        # change is still compared. With none the ADR is new: its keys
        # all count as added, and its entry, absent from the base digest,
        # compares as changed — no skip, which would also skip a rename.
        if [ -n "$base_sha" ]; then
            base_name="$(printf '%s\n' "$base_names" | grep "^$num-.*\.md\$" | head -1)"
            base_adr=""
            if [ -n "$base_name" ]; then
                base_adr="$(git -C "$REPO_ROOT" show "$base_sha:./architecture/adr/$base_name")" \
                    || { echo "validate_adr_index: cannot read $base_name at base ${base_sha:0:8}" >&2; exit 2; }
            fi
            added="$(comm -13 <(printf '%s\n' "$base_adr" | amendment_keys) <(printf '%s\n' "$tbl"))"
            if [ -n "$added" ]; then
                amended=$((amended + 1))
                if [ "$(printf '%s\n' "$base_digest" | digest_entry "$num")" = "$(digest_entry "$num" < "$DIGEST")" ]; then
                    report "$f: amended since ${base_sha:0:8} but its '### $num — ' digest entry is unchanged — the digest still describes the ADR before:"
                    printf '%s\n' "$added" | sed 's/^/      /'
                fi
            fi
        fi
    fi
done

# --- reverse direction: index and digest must not name absent ADRs ------
while IFS= read -r ref; do
    [ -f "$ADR_DIR/$ref" ] || report "README.md links ./$ref, which does not exist"
done < <(grep -o '(\./[0-9][0-9][0-9][0-9]-[^)]*\.md)' "$README" | sed -e 's/^(\.\///' -e 's/)$//' | sort -u)

while IFS= read -r num; do
    ls "$ADR_DIR/$num"-*.md >/dev/null 2>&1 \
        || report "ADR-DIGEST.md has an entry for $num, which is not an ADR file"
done < <(grep -o '^### [0-9][0-9][0-9][0-9] — ' "$DIGEST" | awk '{print $2}' | sort -u)

# --- keyword table targets ----------------------------------------------
while IFS= read -r num; do
    ls "$ADR_DIR/$num"-*.md >/dev/null 2>&1 \
        || report "ADR-DIGEST.md keyword table points at **$num**, which is not an ADR file"
done < <(awk '/^## Keyword/,/^---/' "$DIGEST" | grep -o '\*\*[0-9][0-9][0-9][0-9]\*\*' | tr -d '*' | sort -u)

if [ "$gaps" -gt 0 ]; then
    printf '\nvalidate_adr_index: %d gap(s). Propagation is part of the change, not follow-up (ADR-0048).\n' "$gaps" >&2
    exit 1
fi

if [ -z "$base_sha" ]; then
    amended_line="amendments against a base not checked (--base none)"
else
    amended_line="$amended amended since ${base_sha:0:8}, each with its digest entry edited"
fi
printf 'validate_adr_index: OK — %d ADRs, all template-conformant, indexed, with a digest entry each; %s.\n' \
    "${#ADR_FILES[@]}" "$amended_line"
