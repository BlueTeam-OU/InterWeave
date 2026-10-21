#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# test_phone-run.sh — phone-run's claims, against a stub lease and a stub adb
# that never touch the phone, while it holds the real pixel-9a lease: the
# guard refuses a hand-set or nested PHONE_RUN_HELD, the run never sees it,
# a stop does not wait on the watcher's sleeps, a second TERM does not
# re-run the stop, the start's and the stop's kill-server end at their
# bound, and neither a TERM, a SIGKILLed wrapper nor a SIGKILLed phone-run
# leaves any part of the run or the server.
# Run by hand: bash .agent-fabric/bin/test_phone-run.sh
#
# Not checked: the port's value per login and the readiness wait being a
# bare TCP connect, which the stub cannot see; and the watcher's
# kill-server bound, which runs only in the SIGKILL cases, where the stub
# does not hang.
#
# Not held: if the real lease is SIGKILLed, this script runs on without it
# (the lease cannot forward a SIGKILL) and its remaining cases take this
# login's port until it exits. If this script is SIGKILLed, nothing ends
# the case in flight until its run ends: up to the 300 s sleep of the
# TERM-ignoring cases.
#
# It holds the real pixel-9a lease while it runs: its stub server takes
# this login's adb port, which only a holder of the lease may use, and a
# real phone-run of this login must not start against the stub.
set -u
if [ "${PHONE_RUN_TEST_HELD:-}" != 1 ]; then
  exec fabric-lease pixel-9a -- env PHONE_RUN_TEST_HELD=1 bash "$0" "$@"
fi
here=$(cd "$(dirname "$0")" && pwd)
T=$(mktemp -d "${TMPDIR:-/tmp}/phone-run-test.XXXXXX")
fail=0
# Every case's group goes before the stubs do: a watcher left running
# after $T/bin is gone would reach the host's real adb.
cleanup() {
  local g
  for g in $(cat "$T/groups" 2>/dev/null); do kill -KILL -- "-$g" 2>/dev/null; done
  pkill -KILL -f "$T" 2>/dev/null
  rm -rf "$T"
}
trap cleanup EXIT
mkdir -p "$T/bin"

# The lease as fabric-lease runs a child: a group of its own, a TERM
# forwarded, no lock (each case runs alone).
cat > "$T/bin/fabric-lease" <<EOF
#!/usr/bin/env python3
import signal, subprocess, sys
p = subprocess.Popen(sys.argv[3:], process_group=0)
open("$T/groups", "a").write(f"{p.pid}\\n")
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
# kill-server is counted, and with $T/hang it hangs ignoring TERM.
cat > "$T/bin/adb" <<EOF
#!/usr/bin/env bash
[ "\$*" = "nodaemon server" ] && exec python3 "$T/adbd.py"
if [ "\$*" = kill-server ]; then
  echo x >> "$T/kills"
  [ -e "$T/hang" ] && { trap '' TERM; exec -a "$T/hang" sleep 60; }
fi
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
kills() { wc -l < "$T/kills" 2>/dev/null || echo 0; }
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
: > "$T/kills"
t0=${EPOCHREALTIME/[.,]/}
"$pr" bash -c 'exit 7' >/dev/null 2>&1; rc=$?
ms=$(elapsed "$t0")
check "exit status passed through" [ "$rc" = 7 ]
check "a normal stop takes under 3 s (${ms} ms)" [ "$ms" -lt 3000 ]
check "normal exit leaves nothing" nothing_left
check "kill-server at start and in the stop only ($(kills))" [ "$(kills)" = 2 ]

# Two TERMs to the lease while the run ignores TERM: the stop is not cut
# short, the run is KILLed at the 5 s deadline, and nothing is left.
: > "$T/kills"
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
# What the ignore is for: a second TERM that re-entered the stop would
# run it, and its kill-server, twice.
check "the stop ran once under two TERMs ($(kills) kill-servers)" [ "$(kills)" = 2 ]

# The lease SIGKILLed: the watcher ends the run and the server alone.
"$pr" bash -c "trap '' TERM; exec -a $T/run sleep 300" >/dev/null 2>&1 &
lease=$!
sleep 1.5
kill -KILL "$lease"
wait "$lease" 2>/dev/null
check "SIGKILLed lease leaves nothing" nothing_left

# The script SIGKILLed alone: the lease frees, and the watcher ends the
# run, the server and the group.
"$pr" bash -c "trap '' TERM; exec -a $T/run sleep 300" >/dev/null 2>&1 &
lease=$!
sleep 1.5
kill -KILL "$(tail -n 1 "$T/groups")"
wait "$lease" 2>/dev/null
check "SIGKILLed script leaves nothing" nothing_left

# A kill-server that hangs ignoring TERM is ended by its bound, at the
# start and in the stop: about 10 s, never the hang's 60.
touch "$T/hang"
t0=${EPOCHREALTIME/[.,]/}
"$pr" true >/dev/null 2>&1; rc=$?
ms=$(elapsed "$t0")
rm -f "$T/hang"
check "a run with a hung kill-server exits 0" [ "$rc" = 0 ]
check "a hung kill-server is bounded (${ms} ms < 14000)" [ "$ms" -lt 14000 ]
check "hung kill-server leaves nothing" nothing_left

exit "$fail"
