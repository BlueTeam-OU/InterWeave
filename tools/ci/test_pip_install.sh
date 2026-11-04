#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_pip_install.sh
#
# Self-test for pip_install.sh.
#
# python3 is a stub on PATH. It logs each call's arguments and acts out
# the next line of the sandbox's plan: `ok` exits 0, `fail` exits 1,
# `hang` sleeps far past the attempt limit, `deaf` does the same with
# TERM ignored (so only --kill-after ends it). The attempt limit is
# one second, so a hang costs the run a second, not two minutes.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/pip_install.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(realpath -- "$(mktemp -d)")"
[ -n "$SANDBOX" ] && [ -d "$SANDBOX" ] || { echo "test_pip_install: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
BIN="$SANDBOX/bin"
mkdir -p "$BIN"

pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

cat > "$BIN/python3" <<STUB
#!/usr/bin/env bash
echo "python3 \$*" >> "$SANDBOX/log"
n=\$(wc -l < "$SANDBOX/log")
case "\$(sed -n "\${n}p" "$SANDBOX/plan")" in
    ok) exit 0 ;;
    fail) exit 1 ;;
    hang) exec sleep 60 ;;
    deaf) trap '' TERM; sleep 60 & wait \$!; sleep 60 ;;
    *) echo "stub: no plan for call \$n" >&2; exit 3 ;;
esac
STUB
chmod +x "$BIN/python3"

REQ="$SANDBOX/requirements.txt"
echo 'attrs==26.1.0 --hash=sha256:00' > "$REQ"

# run <plan...> -- <args...>: OUT, STATUS, CALLS (python3 calls made).
run() {
    : > "$SANDBOX/log"; : > "$SANDBOX/plan"
    while [[ $# -gt 0 && "$1" != -- ]]; do echo "$1" >> "$SANDBOX/plan"; shift; done
    shift
    OUT="$(PATH="$BIN:$PATH" PIP_INSTALL_ATTEMPT_SECONDS="${LIMIT:-1}" bash "$UNDER_TEST" "$@" 2>&1)"
    STATUS=$?
    CALLS=$(wc -l < "$SANDBOX/log")
}

echo "pip_install.sh — a stalled or failed install is retried within a bound"

run ok -- "$REQ"
if [[ $STATUS -eq 0 && $CALLS -eq 1 && "$OUT" == *"installed on attempt 1 of 3"* ]]; then
    pass "a healthy install runs pip once and says so"
else fail "healthy install: status $STATUS, $CALLS calls" "$OUT"; fi
want="python3 -m pip install --require-hashes --progress-bar off --timeout 30 --disable-pip-version-check -r $REQ"
if [[ "$(cat "$SANDBOX/log")" == "$want" ]]; then
    pass "pip is asked for hashes, no progress bar, a 30 s socket timeout, no upgrade notice, and not --quiet"
else fail "pip's arguments" "$(cat "$SANDBOX/log")"; fi

start=$SECONDS
run hang ok -- "$REQ"
took=$((SECONDS - start))
if [[ $STATUS -eq 0 && $CALLS -eq 2 && "$OUT" == *"attempt 1 of 3 stopped after 1 s"* \
      && "$OUT" == *"installed on attempt 2 of 3"* && $took -lt 10 ]]; then
    pass "a stalled attempt is stopped at the limit and retried (${took} s)"
else fail "stalled attempt: status $STATUS, $CALLS calls, ${took} s" "$OUT"; fi

start=$SECONDS
run deaf ok -- "$REQ"
took=$((SECONDS - start))
if [[ $STATUS -eq 0 && $CALLS -eq 2 && "$OUT" == *"attempt 1 of 3 stopped"* && $took -lt 20 ]]; then
    pass "an attempt that ignores TERM is killed and retried (${took} s)"
else fail "TERM-deaf attempt: status $STATUS, $CALLS calls, ${took} s" "$OUT"; fi

run fail fail ok -- "$REQ"
if [[ $STATUS -eq 0 && $CALLS -eq 3 && "$OUT" == *"attempt 2 of 3 failed (status 1"* ]]; then
    pass "a failed attempt is retried, up to the third"
else fail "failed attempts: status $STATUS, $CALLS calls" "$OUT"; fi

run fail hang fail ok -- "$REQ"
if [[ $STATUS -eq 1 && $CALLS -eq 3 && "$OUT" == *"every attempt failed"* ]]; then
    pass "three failures exit 1, and no fourth attempt is made"
else fail "exhausted attempts: status $STATUS, $CALLS calls" "$OUT"; fi

run -- "$SANDBOX/missing.txt"
if [[ $STATUS -eq 2 && $CALLS -eq 0 ]]; then
    pass "a missing requirements file exits 2 before pip runs"
else fail "missing file: status $STATUS, $CALLS calls" "$OUT"; fi

run --
if [[ $STATUS -eq 2 && $CALLS -eq 0 ]]; then
    pass "no argument exits 2"
else fail "no argument: status $STATUS" "$OUT"; fi

for bad in 0 abc -5; do
    LIMIT="$bad" run ok -- "$REQ"
    if [[ $STATUS -eq 2 && $CALLS -eq 0 ]]; then
        pass "an attempt limit of '$bad' exits 2"
    else fail "limit '$bad': status $STATUS, $CALLS calls" "$OUT"; fi
done

OUT="$(bash "$UNDER_TEST" --help)"
if [[ $? -eq 0 && "$OUT" == *"PIP_INSTALL_ATTEMPT_SECONDS"* && "$OUT" != *">>> help"* ]]; then
    pass "--help prints the help block"
else fail "--help" "$OUT"; fi

echo
if [[ $failures -eq 0 ]]; then
    echo "test_pip_install: OK — all assertions passed."
    exit 0
fi
echo "test_pip_install: $failures assertion(s) FAILED." >&2
exit 1
