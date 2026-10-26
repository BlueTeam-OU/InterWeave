#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_apt_install.sh
#
# Self-test for apt_install.sh.
#
# apt-get, sudo, id and date are stubs on PATH. apt-get logs each call
# (who ran it, its environment's DEBIAN_FRONTEND, its arguments) and, for
# `install` with a Dir::Cache::Archives, downloads by writing a .deb and
# apt's partial/ and lock into that directory, as the real one does; a
# file in the sandbox makes a named subcommand fail. sudo logs and runs
# its command; id answers the uid the sandbox names.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/apt_install.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(realpath -- "$(mktemp -d)")"
[ -n "$SANDBOX" ] && [ -d "$SANDBOX" ] || { echo "test_apt_install: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
BIN="$SANDBOX/bin"
mkdir -p "$BIN"

pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

cat > "$BIN/apt-get" <<STUB
#!/usr/bin/env bash
echo "apt-get[\${AS:-user}] FRONTEND=\${DEBIAN_FRONTEND:-} \$*" >> "$SANDBOX/log"
sub=
for a in "\$@"; do case "\$a" in update|install|autoclean) sub="\$a" ;; esac; done
[[ -e "$SANDBOX/fail-\$sub" ]] && exit 100
dir=
for a in "\$@"; do case "\$a" in Dir::Cache::Archives=*) dir="\${a#*=}" ;; esac; done
if [[ "\$sub" == install && -n "\$dir" ]]; then
    mkdir -p "\$dir/partial"; touch "\$dir/lock" "\$dir/partial/half.deb" "\$dir/xvfb_2_amd64.deb"
fi
exit 0
STUB
cat > "$BIN/sudo" <<STUB
#!/usr/bin/env bash
echo "sudo \$*" >> "$SANDBOX/log"
[[ "\$1" == -n ]] || exit 99
shift
AS=root exec "\$@"
STUB
cat > "$BIN/id" <<STUB
#!/usr/bin/env bash
case "\$1" in -u) cat "$SANDBOX/uid" ;; -g) echo 1001 ;; *) exit 99 ;; esac
STUB
cat > "$BIN/chown" <<STUB
#!/usr/bin/env bash
echo "chown \$*" >> "$SANDBOX/log"
STUB
cat > "$BIN/date" <<STUB
#!/usr/bin/env bash
[[ "\$*" == "-u +%G-W%V" ]] || exit 99
cat "$SANDBOX/week"
STUB
chmod +x "$BIN"/*

cat > "$SANDBOX/os-release" <<'OS'
NAME="Ubuntu"
ID=ubuntu
VERSION_ID="24.04"
OS
printf 'xvfb\nat-spi2-core   # the bus\n\n# a comment\nlibx11-6\n' > "$SANDBOX/list"

reset() { rm -f "$SANDBOX/log" "$SANDBOX"/fail-*; echo 1001 > "$SANDBOX/uid"; echo 2026-W44 > "$SANDBOX/week"; }
run() { PATH="$BIN:$PATH" APT_INSTALL_OS_RELEASE="$SANDBOX/os-release" bash "$UNDER_TEST" "$@" 2>&1; }
log() { cat "$SANDBOX/log" 2>/dev/null; }

echo "key"
reset
k1="$(run key "$SANDBOX/list")"; rc=$?
[[ $rc -eq 0 && "$k1" =~ ^apt-ubuntu-24\.04-$(uname -m)-[0-9a-f]{16}-2026-W44$ ]] \
    && pass "names the OS, its version, the arch, a hash, then the week" || fail "key" "rc=$rc $k1"
printf '# reordered, other comments\nlibx11-6 xvfb\nat-spi2-core\nxvfb\n' > "$SANDBOX/list2"
k2="$(run key "$SANDBOX/list2")"
[[ "$k1" == "$k2" ]] && pass "comments, order and a repeat keep the key" || fail "reformat changed the key" "$k1 / $k2"
printf 'xvfb\nat-spi2-core\nlibx11-6\nlibxi6\n' > "$SANDBOX/list3"
k3="$(run key "$SANDBOX/list3")"
[[ "$k1" != "$k3" ]] && pass "a new package is a new key" || fail "a new package kept the key" "$k3"
echo 2026-W45 > "$SANDBOX/week"
k4="$(run key "$SANDBOX/list")"
[[ "$k1" != "$k4" && "$k4" == *-2026-W45 && "${k4%-*-W*}" == "${k1%-*-W*}" ]] \
    && pass "a new week is a new key with the same prefix" || fail "week" "$k1 / $k4"
echo 2026-W44 > "$SANDBOX/week"
sed -i 's/24.04/26.04/' "$SANDBOX/os-release"
k5="$(run key "$SANDBOX/list")"
[[ "$k5" == apt-ubuntu-26.04-* ]] && pass "a new image release is a new key" || fail "release" "$k5"
sed -i 's/26.04/24.04/' "$SANDBOX/os-release"
printf 'ID=ubuntu\n' > "$SANDBOX/os-bare"
out="$(PATH="$BIN:$PATH" APT_INSTALL_OS_RELEASE="$SANDBOX/os-bare" bash "$UNDER_TEST" key "$SANDBOX/list" 2>&1)"; rc=$?
[[ $rc -eq 2 && "$out" == *"no ID and VERSION_ID"* ]] && pass "an os-release without VERSION_ID is refused" || fail "bare os-release" "rc=$rc $out"

echo "the list is checked before apt runs"
reset
printf 'xvfb\nfoo;rm\n' > "$SANDBOX/bad"
out="$(run install "$SANDBOX/bad")"; rc=$?
[[ $rc -eq 2 && "$out" == *"not a package name: foo;rm"* && -z "$(log)" ]] \
    && pass "a name outside the grammar: exit 2, apt never called" || fail "bad name" "rc=$rc $out | $(log)"
printf 'Xvfb\n' > "$SANDBOX/upper"
out="$(run install "$SANDBOX/upper")"; rc=$?
[[ $rc -eq 2 && -z "$(log)" ]] && pass "an upper-case name is refused" || fail "upper" "rc=$rc $out"
printf '# nothing\n\n' > "$SANDBOX/empty"
out="$(run install "$SANDBOX/empty")"; rc=$?
[[ $rc -eq 2 && "$out" == *"names no package"* && -z "$(log)" ]] && pass "an empty list: exit 2" || fail "empty" "rc=$rc $out"
out="$(run install "$SANDBOX/missing")"; rc=$?
[[ $rc -eq 2 && -z "$(log)" ]] && pass "a missing list: exit 2" || fail "missing" "rc=$rc $out"
out="$(run key "$SANDBOX/empty")"; rc=$?
[[ $rc -eq 2 ]] && pass "key refuses an empty list too" || fail "key empty" "rc=$rc $out"
for args in "" "install" "install --cache" "frob x" "install a b"; do
    reset
    # shellcheck disable=SC2086 # the words are the arguments
    out="$(run $args)"; rc=$?
    [[ $rc -eq 2 && -z "$(log)" ]] && pass "usage '$args': exit 2" || fail "usage '$args'" "rc=$rc $out"
done

echo "install, no cache"
reset
out="$(run install "$SANDBOX/list")"; rc=$?
want="sudo -n env DEBIAN_FRONTEND=noninteractive apt-get -o Acquire::Retries=5 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30 update -q
apt-get[root] FRONTEND=noninteractive -o Acquire::Retries=5 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30 update -q
sudo -n env DEBIAN_FRONTEND=noninteractive apt-get -o Acquire::Retries=5 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30 install -y -q --no-install-recommends at-spi2-core libx11-6 xvfb
apt-get[root] FRONTEND=noninteractive -o Acquire::Retries=5 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30 install -y -q --no-install-recommends at-spi2-core libx11-6 xvfb"
[[ $rc -eq 0 && "$(log)" == "$want" ]] \
    && pass "update then install, both with the retries and timeouts, under sudo -n, no Dir option" \
    || fail "no-cache install" "rc=$rc $out"$'\n'"$(log)"

echo "install as root (a job's container)"
reset; echo 0 > "$SANDBOX/uid"
out="$(run install "$SANDBOX/list")"; rc=$?
[[ $rc -eq 0 && "$(log)" != *sudo* && "$(grep -c '^apt-get\[user\] FRONTEND=noninteractive ' "$SANDBOX/log")" -eq 2 ]] \
    && pass "apt runs directly, never sudo" || fail "root" "rc=$rc $out"$'\n'"$(log)"

echo "install with a cache"
reset
C="$SANDBOX/cache"; rm -rf "$C"
out="$(run install --cache "$C" "$SANDBOX/list")"; rc=$?
inst="$(grep ' install ' "$SANDBOX/log" | grep '^apt-get')"
[[ $rc -eq 0 && "$inst" == *" -o Dir::Cache::Archives=$C/ install -y -q --no-install-recommends at-spi2-core libx11-6 xvfb" ]] \
    && pass "the install reads and downloads in the cache" || fail "Dir option" "rc=$rc $out"$'\n'"$(log)"
grep -q "^apt-get\[root\] .*Acquire::Retries=5 .*Dir::Cache::Archives=$C/ autoclean -q$" "$SANDBOX/log" \
    && pass "autoclean runs against the cache" || fail "autoclean" "$(log)"
[[ -f "$C/xvfb_2_amd64.deb" && ! -e "$C/partial" && ! -e "$C/lock" ]] \
    && pass "the .deb stays; partial/ and lock are gone" || fail "cache contents" "$(ls -la "$C")"
grep -qx "sudo -n chown -R 1001:1001 $C" "$SANDBOX/log" \
    && pass "the cache is given back to the caller" || fail "chown" "$(log)"
[[ "$out" == *"0 .deb(s) in $C before"* && "$out" == *"1 .deb(s) in $C after"* ]] \
    && pass "the counts before and after are printed" || fail "counts" "$out"
order="$(grep -o '^apt-get\[root\].* \(update\|install\|autoclean\) ' "$SANDBOX/log" | awk '{print $(NF)}' | tr '\n' ' ')"
[[ "$order" == "update install autoclean " ]] && pass "update, install, autoclean, in order" || fail "order" "$order"

echo "a relative cache is made absolute"
reset
( cd "$SANDBOX" && rm -rf rel && PATH="$BIN:$PATH" bash "$UNDER_TEST" install --cache rel "$SANDBOX/list" >/dev/null 2>&1 )
grep -q "Dir::Cache::Archives=$SANDBOX/rel/ install" "$SANDBOX/log" \
    && pass "Dir::Cache::Archives is the absolute path" || fail "relative cache" "$(log)"

echo "apt-get failing"
for sub in update install autoclean; do
    reset; rm -rf "$C"; touch "$SANDBOX/fail-$sub"
    out="$(run install --cache "$C" "$SANDBOX/list")"; rc=$?
    [[ $rc -eq 1 && "$out" == *"apt-get $sub failed (status 100)"* ]] \
        && pass "$sub failing: exit 1, named with its status" || fail "$sub failing" "rc=$rc $out"
done
reset; touch "$SANDBOX/fail-update"
run install --cache "$C" "$SANDBOX/list" >/dev/null
[[ "$(log)" != *" install "* ]] && pass "a failed update installs nothing" || fail "install after failed update" "$(log)"

echo
if [[ $failures -eq 0 ]]; then echo "test_apt_install: all passed"; exit 0; fi
echo "test_apt_install: $failures failed" >&2; exit 1
