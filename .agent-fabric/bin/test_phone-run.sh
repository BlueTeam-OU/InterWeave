#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# test_phone-run.sh — phone-run's claims, against a stub lease and a stub adb,
# with no phone and no real lease: the guard refuses a hand-set or nested
# PHONE_RUN_HELD, the run never sees it, a stop does not wait on the
# watcher's sleeps, a second TERM does not cut the stop short, and neither
# a TERM nor a SIGKILLed wrapper leaves any part of the run or the server.
# Run by hand: bash .agent-fabric/bin/test_phone-run.sh
set -u
here=$(cd "$(dirname "$0")" && pwd)
T=$(mktemp -d "${TMPDIR:-/tmp}/phone-run-test.XXXXXX")
fail=0
cleanup() {
  pkill -KILL -f "$T" 2>/dev/null
  rm -rf "$T"
}
trap cleanup EXIT
mkdir -p "$T/bin"

# The lease as fabric-lease runs a child: a group of its own, a TERM
# forwarded, no lock (each case runs alone).
cat > "$T/bin/fabric-lease" <<'EOF'
#!/usr/bin/env python3
import signal, subprocess, sys
p = subprocess.Popen(sys.argv[3:], process_group=0)
signal.signal(signal.SIGTERM, lambda *a: p.send_signal(signal.SIGTERM))
sys.exit(p.wait())
EOF
# adb: a server that listens on the login's port; every other call succeeds.
cat > "$T/adbd.py" <<'EOF'
import os, socket
s = socket.socket()
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("127.0.0.1", int(os.environ["ANDROID_ADB_SERVER_PORT"])))
s.listen()
while True:
    s.accept()[0].close()
EOF
cat > "$T/bin/adb" <<EOF
#!/usr/bin/env bash
[ "\$*" = "nodaemon server" ] && exec python3 "$T/adbd.py"
exit 0
EOF
chmod +x "$T/bin/fabric-lease" "$T/bin/adb"
export PATH="$T/bin:$PATH"
pr="$here/phone-run"

check() {  # check <name> <condition...>
  local name=$1; shift
  if "$@"; then echo "ok   $name"; else echo "FAIL $name"; fail=1; fi
}
# Nothing of a case is left: the run carries $T in its arguments (its
# sleep renamed with exec -a, since bash execs a last command and the
# plain sleep would carry nothing to find), the server is adbd.py under
# $T, the lease stub lives in $T.
nothing_left() {
  local end=$((SECONDS + 10))
  while pgrep -f "$T" >/dev/null && [ $SECONDS -lt $end ]; do sleep 0.2; done
  ! pgrep -f "$T" >/dev/null
}
elapsed() { echo $(( (${EPOCHREALTIME/[.,]/} - $1) / 1000 )); }  # ms

# The guard: the variable set by hand, outside the lease.
out=$(PHONE_RUN_HELD=1 "$pr" true 2>&1); rc=$?
check "hand-set PHONE_RUN_HELD refused" [ "$rc" = 2 ]
check "refusal names the lease" grep -q "not under the pixel-9a lease" <<<"$out"

# The run never inherits the variable.
out=$("$pr" bash -c 'echo "held=${PHONE_RUN_HELD-unset}"' 2>&1)
check "the run does not see PHONE_RUN_HELD" grep -qx "held=unset" <<<"$out"

# A phone-run inside a run, the variable set: its parent is the outer
# phone-run, not the lease, so it is refused and the outer exits with it.
"$pr" bash -c 'PHONE_RUN_HELD=1 exec "$0" true' "$pr" >/dev/null 2>&1; rc=$?
check "nested phone-run with the variable refused" [ "$rc" = 2 ]
check "nested case leaves nothing" nothing_left

# A normal exit returns the run's status and does not wait on the
# watcher's TERM-ignoring sleep (review 5605022748 F1: it took 5.9 s).
t0=${EPOCHREALTIME/[.,]/}
"$pr" bash -c 'exit 7' >/dev/null 2>&1; rc=$?
ms=$(elapsed "$t0")
check "exit status passed through" [ "$rc" = 7 ]
check "a normal stop takes under 3 s (${ms} ms)" [ "$ms" -lt 3000 ]
check "normal exit leaves nothing" nothing_left

# Two TERMs to the lease while the run ignores TERM: the stop is not cut
# short, the run is KILLed at the 5 s deadline, and nothing is left.
"$pr" bash -c "trap '' TERM; exec -a $T/run sleep 300" >/dev/null 2>&1 &
lease=$!
sleep 1.5
t0=${EPOCHREALTIME/[.,]/}
kill -TERM "$lease"; sleep 0.3; kill -TERM "$lease" 2>/dev/null
wait "$lease"; rc=$?
ms=$(elapsed "$t0")
check "TERM exits 143" [ "$rc" = 143 ]
check "second TERM does not cut the stop short (${ms} ms >= 4500)" [ "$ms" -ge 4500 ]
check "TERM leaves nothing" nothing_left

# The lease SIGKILLed: the watcher ends the run and the server alone.
"$pr" bash -c "trap '' TERM; exec -a $T/run sleep 300" >/dev/null 2>&1 &
lease=$!
sleep 1.5
kill -KILL "$lease"
wait "$lease" 2>/dev/null
check "SIGKILLed lease leaves nothing" nothing_left

exit "$fail"
