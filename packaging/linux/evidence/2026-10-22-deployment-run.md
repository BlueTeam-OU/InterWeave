<!-- SPDX-License-Identifier: Apache-2.0 -->
# Host run: DEPLOYING.md under a systemd user manager, 2026-10-22

The recorded run of `packaging/linux/DEPLOYING.md`, steps 1 to 6: two
profiles from the installed programs to running
`interweave-daemon@<profile>` units that reach each other, then a
restart by `SIGHUP`, then stop and disable.

**Not a second uid.** It ran in the deploying account's own user manager
under a scratch `HOME` and XDG roots (architect-cto's decision of
2026-10-22, until a second login is provisioned; the run on one is to be
added here). The manager does not search a scratch config directory, so
the templates `install.sh` placed there were linked into the manager's
runtime unit directory, and a per-instance drop-in named the installed
binary and the scratch XDG tree -- the method of
`tests/desktop-e2e/tests/user_units.rs`. On a real account none of that
is needed: `install.sh` writes where the manager looks. For the same
reason `install.sh` reports no user manager reachable (it looks through
the scratch `XDG_RUNTIME_DIR`), and the script reaches the manager
through the account's real one.

- Commit: the run printed `b22a4b2d`. `transport-daemon` and
  `transportctl` were release builds of `cc8a07f4`, whose code is
  `b22a4b2d`'s (the commit after it changes only `DEPLOYING.md`);
  `human-desktop`, installed and not run, of `fbca15e6`.
- Host: Fedora, `systemd 258 (258.11-1.fc43)`, lingering on.
- Profiles `a` and `b`, ports 47201 and 47202 on `127.0.0.1`, each the
  other's only allowed peer and static-bootstrap entry: DEPLOYING.md's
  `config.yaml`, verbatim but for the names.
- Both units were enabled for this boot and started by one `systemctl`
  call, so they start together: the start race fixed in `bb40c908` (one
  of two daemons exiting "File exists") would show here.
- What was not shown: a second uid, a host other than this one, a boot
  with lingering, and `install.sh` writing where the manager looks.

## The script

```bash
#!/usr/bin/env bash
# The j103 recorded run: packaging/linux/DEPLOYING.md under this account's
# own user manager, with a scratch HOME and XDG roots (NOT a second uid).
# The manager does not search the scratch config directory, so the two
# templates install.sh placed there are linked into the manager's runtime
# unit directory and a drop-in points ExecStart and the XDG roots at the
# scratch tree -- the method of tests/desktop-e2e/tests/user_units.rs.
# Everything it links, enables or starts is removed at the end.
set -uo pipefail
REPO=$REPO
BINS=$REPO/target/release
S=$1                                   # scratch root, created empty
R=/run/user/$(id -u)/iwrec             # short: a socket path fits 108 bytes
UNITS=/run/user/$(id -u)/systemd/user  # the manager's runtime unit dir
H=$S/home
export HOME=$H XDG_CONFIG_HOME=$H/.config XDG_DATA_HOME=$H/.local/share \
       XDG_STATE_HOME=$H/.local/state XDG_CACHE_HOME=$H/.cache XDG_RUNTIME_DIR=$R
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus
export PATH=$H/.local/bin:$PATH
PROFILES=(a b); declare -A PORT=([a]=47201 [b]=47202) PEER
# The manager is reached through the account's real runtime directory;
# only transportctl and the daemon see the scratch one.
REAL_RUNTIME=/run/user/$(id -u)
sc() { XDG_RUNTIME_DIR=$REAL_RUNTIME systemctl --user "$@"; }
jc() { XDG_RUNTIME_DIR=$REAL_RUNTIME journalctl --user "$@"; }
step() { printf '\n### %s\n' "$*"; }
run() { printf '$ %s\n' "$*"; "$@"; local rc=$?; printf '(exit %d)\n' "$rc"; return 0; }
cleanup() {
    for p in "${PROFILES[@]}"; do
        sc stop "interweave-daemon@$p" 2>/dev/null
        sc disable --runtime "interweave-daemon@$p" 2>/dev/null
        rm -rf "$UNITS/interweave-daemon@$p.service.d"
    done
    rm -f "$UNITS/interweave-daemon@.service" "$UNITS/interweave-client@.service"
    sc daemon-reload
    sc reset-failed 'interweave-*@*.service' 2>/dev/null
    rm -rf "$R"
}
trap cleanup EXIT
mkdir -p "$H" "$R"; chmod 700 "$S" "$H" "$R"

step "0. the host"
run uname -sm
run systemctl --version --quiet
run sc is-system-running
run loginctl show-user "$(id -un)" -p Linger
printf 'commit: %s\n' "$(git -C "$REPO" rev-parse HEAD)"

step "1. the programs: a release build of the commit above, installed"
run "$REPO/packaging/linux/install.sh" "$BINS"
run ls -l "$H/.local/bin" "$XDG_CONFIG_HOME/systemd/user"
# Stand-in for the manager searching the scratch config directory.
for t in interweave-daemon@.service interweave-client@.service; do
    sc link --runtime "$XDG_CONFIG_HOME/systemd/user/$t" >/dev/null
done

step "2. the identities"
for p in "${PROFILES[@]}"; do
    printf '$ transportctl --profile %s identity init\n' "$p"
    PEER[$p]="$(transportctl --profile "$p" identity init)"; printf '%s\n(exit %d)\n' "${PEER[$p]}" $?
done

step "3. config.yaml, per DEPLOYING.md step 3"
for p in "${PROFILES[@]}"; do
    o=b; [ "$p" = b ] && o=a
    d="$XDG_CONFIG_HOME/interweave/profiles/$p"
    mkdir -p "$d"; chmod 700 "$XDG_CONFIG_HOME/interweave" "$XDG_CONFIG_HOME/interweave/profiles" "$d"
    cat > "$d/config.yaml" <<EOF
schema_version: 2
profile: { name: $p }
trust:
  policy: static-allowlist
  allowed_peers: ["${PEER[$o]}"]
endpoints:
  default_direct_endpoint: claude
  entries:
    - { id: claude, enabled: true, advertise: false, allowed_client_kinds: [claude-channel] }
channels: { desired: [] }
discovery:
  providers:
    - type: static-bootstrap
      enabled: true
      priority: 5
      config:
        peers: ["/ip4/127.0.0.1/tcp/${PORT[$o]}/p2p/${PEER[$o]}"]
transport: { listen: { addresses: ["/ip4/127.0.0.1/tcp/${PORT[$p]}"] } }
observability: { log_level: info }
EOF
    chmod 600 "$d/config.yaml"
    printf -- '--- %s\n' "$d/config.yaml"; cat "$d/config.yaml"
    # The scratch tree for the unit (stands in for the account's own HOME).
    mkdir -p "$UNITS/interweave-daemon@$p.service.d"
    printf '[Service]\nExecStart=\nExecStart=%s --profile %%i\nEnvironment=XDG_CONFIG_HOME=%s XDG_DATA_HOME=%s XDG_STATE_HOME=%s XDG_CACHE_HOME=%s XDG_RUNTIME_DIR=%s\n' \
        "$H/.local/bin/transport-daemon" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_CACHE_HOME" "$R" \
        > "$UNITS/interweave-daemon@$p.service.d/scratch.conf"
done

step "4. start it (both enabled for this boot, started together)"
run sc daemon-reload
run sc enable --runtime interweave-daemon@a interweave-daemon@b
run sc start interweave-daemon@a interweave-daemon@b
for _ in $(seq 40); do
    transportctl --profile a status >/dev/null 2>&1 && transportctl --profile b status >/dev/null 2>&1 && break
    sleep 0.5
done
sleep 3
run sc --no-pager status interweave-daemon@a interweave-daemon@b

step "5. check it"
for p in "${PROFILES[@]}"; do
    run transportctl --profile "$p" status
    run transportctl --profile "$p" peers list
    run transportctl --profile "$p" trust list
    run transportctl --profile "$p" endpoints list
done

step "6. logs"
run jc --no-pager -o short-iso -u interweave-daemon@a -u interweave-daemon@b --since "-2min"

step "restart by SIGHUP: stops as on SIGTERM, exit 75, restarted 5 s later"
before="$(sc show -p MainPID --value interweave-daemon@a)"
run sc kill --kill-whom=main --signal=SIGHUP interweave-daemon@a
sleep 8
after="$(sc show -p MainPID --value interweave-daemon@a)"
printf 'MainPID before %s, after %s; ActiveState %s, NRestarts %s\n' "$before" "$after" \
    "$(sc show -p ActiveState --value interweave-daemon@a)" \
    "$(sc show -p NRestarts --value interweave-daemon@a)"
sleep 3
run transportctl --profile a peers list

step "stop and disable"
run sc disable --runtime --now interweave-daemon@a interweave-daemon@b
run transportctl --profile a status
```

## The transcript

```text

### 0. the host
$ uname -sm
Linux x86_64
(exit 0)
$ systemctl --version --quiet
systemd 258 (258.11-1.fc43)
+PAM +AUDIT +SELINUX -APPARMOR +IMA +IPE +SMACK +SECCOMP -GCRYPT +GNUTLS +OPENSSL +ACL +BLKID +CURL +ELFUTILS +FIDO2 +IDN2 -IDN -IPTC +KMOD +LIBCRYPTSETUP +LIBCRYPTSETUP_PLUGINS +LIBFDISK +PCRE2 +PWQUALITY +P11KIT +QRENCODE +TPM2 +BZIP2 +LZ4 +XZ +ZLIB +ZSTD +BPF_FRAMEWORK +BTF +XKBCOMMON +UTMP +SYSVINIT +LIBARCHIVE
(exit 0)
$ sc is-system-running
degraded
(exit 1)
$ loginctl show-user <account> -p Linger
Linger=yes
(exit 0)
commit: b22a4b2d84075bd73cd56b33130f0559904d13dd

### 1. the programs: a release build of the commit above, installed
$ $REPO/packaging/linux/install.sh $REPO/target/release
install.sh: no systemd user manager reachable; the units load when it starts
installed into $SCRATCH/home/.local/bin and $SCRATCH/home/.config/systemd/user; nothing is enabled
(exit 0)
$ ls -l $SCRATCH/home/.local/bin $SCRATCH/home/.config/systemd/user
$SCRATCH/home/.config/systemd/user:
total 8
-rw-r--r--. 1 <account> <account> 1251 Oct 22 10:19 interweave-client@.service
-rw-r--r--. 1 <account> <account> 1723 Oct 22 10:19 interweave-daemon@.service

$SCRATCH/home/.local/bin:
total 351768
-rwxr-xr-x. 1 <account> <account> 181765688 Oct 22 10:19 human-desktop
-rwxr-xr-x. 1 <account> <account> 157875480 Oct 22 10:19 transport-daemon
-rwxr-xr-x. 1 <account> <account>  20562320 Oct 22 10:19 transportctl
(exit 0)
Created symlink '/run/user/1016/systemd/user/interweave-daemon@.service' → '$SCRATCH/home/.config/systemd/user/interweave-daemon@.service'.
Created symlink '/run/user/1016/systemd/user/interweave-client@.service' → '$SCRATCH/home/.config/systemd/user/interweave-client@.service'.

### 2. the identities
$ transportctl --profile a identity init
12D3KooWP6JKKVGLN6JqUD68icL4L1JP9p3nAfportwLWFeTf4mj
(exit 0)
$ transportctl --profile b identity init
12D3KooWKjUQX5PtHWJDqKMeFbTjvrMUtYoqipr6CXrNJ1tzJLPm
(exit 0)

### 3. config.yaml, per DEPLOYING.md step 3
--- $SCRATCH/home/.config/interweave/profiles/a/config.yaml
schema_version: 2
profile: { name: a }
trust:
  policy: static-allowlist
  allowed_peers: ["12D3KooWKjUQX5PtHWJDqKMeFbTjvrMUtYoqipr6CXrNJ1tzJLPm"]
endpoints:
  default_direct_endpoint: claude
  entries:
    - { id: claude, enabled: true, advertise: false, allowed_client_kinds: [claude-channel] }
channels: { desired: [] }
discovery:
  providers:
    - type: static-bootstrap
      enabled: true
      priority: 5
      config:
        peers: ["/ip4/127.0.0.1/tcp/47202/p2p/12D3KooWKjUQX5PtHWJDqKMeFbTjvrMUtYoqipr6CXrNJ1tzJLPm"]
transport: { listen: { addresses: ["/ip4/127.0.0.1/tcp/47201"] } }
observability: { log_level: info }
--- $SCRATCH/home/.config/interweave/profiles/b/config.yaml
schema_version: 2
profile: { name: b }
trust:
  policy: static-allowlist
  allowed_peers: ["12D3KooWP6JKKVGLN6JqUD68icL4L1JP9p3nAfportwLWFeTf4mj"]
endpoints:
  default_direct_endpoint: claude
  entries:
    - { id: claude, enabled: true, advertise: false, allowed_client_kinds: [claude-channel] }
channels: { desired: [] }
discovery:
  providers:
    - type: static-bootstrap
      enabled: true
      priority: 5
      config:
        peers: ["/ip4/127.0.0.1/tcp/47201/p2p/12D3KooWP6JKKVGLN6JqUD68icL4L1JP9p3nAfportwLWFeTf4mj"]
transport: { listen: { addresses: ["/ip4/127.0.0.1/tcp/47202"] } }
observability: { log_level: info }

### 4. start it (both enabled for this boot, started together)
$ sc daemon-reload
(exit 0)
$ sc enable --runtime interweave-daemon@a interweave-daemon@b
Created symlink '/run/user/1016/systemd/user/interweave-daemon@a.service' → '$SCRATCH/home/.config/systemd/user/interweave-daemon@.service'.
Created symlink '/run/user/1016/systemd/user/default.target.wants/interweave-daemon@a.service' → '$SCRATCH/home/.config/systemd/user/interweave-daemon@.service'.
Created symlink '/run/user/1016/systemd/user/interweave-daemon@b.service' → '$SCRATCH/home/.config/systemd/user/interweave-daemon@.service'.
Created symlink '/run/user/1016/systemd/user/default.target.wants/interweave-daemon@b.service' → '$SCRATCH/home/.config/systemd/user/interweave-daemon@.service'.
(exit 0)
$ sc start interweave-daemon@a interweave-daemon@b
(exit 0)
$ sc --no-pager status interweave-daemon@a interweave-daemon@b
● interweave-daemon@a.service - InterWeave transport daemon, profile a
     Loaded: loaded (/run/user/1016/systemd/user/interweave-daemon@a.service; enabled-runtime; preset: disabled)
    Drop-In: /usr/lib/systemd/user/service.d
             └─10-timeout-abort.conf
             /run/user/1016/systemd/user/interweave-daemon@a.service.d
             └─scratch.conf
     Active: active (running) since Thu 2026-10-22 10:19:15 CEST; 3s ago
 Invocation: 1c36ceb52a314cbc8e2de5933d41d659
       Docs: https://github.com/BlueTeam-OU/InterWeave/tree/main/packaging/linux
   Main PID: 4183261 (transport-daemo)
      Tasks: 12 (limit: 7680)
     Memory: 2M (peak: 2.8M)
        CPU: 16ms
     CGroup: /user.slice/user-1016.slice/user@1016.service/app.slice/app-interweave\x2ddaemon.slice/interweave-daemon@a.service
             └─4183261 $SCRATCH/home/.local/bin/transport-daemon --profile a

Oct 22 10:19:15 develop-qzapp systemd[2354]: Starting interweave-daemon@a.service - InterWeave transport daemon, profile a...
Oct 22 10:19:15 develop-qzapp systemd[2354]: Started interweave-daemon@a.service - InterWeave transport daemon, profile a.
Oct 22 10:19:15 develop-qzapp transport-daemon[4183261]: 2026-10-22T08:19:15.936360Z  INFO transport_daemon::daemon: starting profile="a" peer="12D3KooWP6JKKVGLN6JqUD68icL4L1JP9p3nAfportwLWFeTf4mj"
Oct 22 10:19:15 develop-qzapp transport-daemon[4183261]: 2026-10-22T08:19:15.956579Z  INFO transport_daemon::daemon: serving data=/run/user/1016/iwrec/interweave/a.sock

● interweave-daemon@b.service - InterWeave transport daemon, profile b
     Loaded: loaded (/run/user/1016/systemd/user/interweave-daemon@b.service; enabled-runtime; preset: disabled)
    Drop-In: /usr/lib/systemd/user/service.d
             └─10-timeout-abort.conf
             /run/user/1016/systemd/user/interweave-daemon@b.service.d
             └─scratch.conf
     Active: active (running) since Thu 2026-10-22 10:19:15 CEST; 3s ago
 Invocation: 031fcc5e81844df78fb1a084fb09026d
       Docs: https://github.com/BlueTeam-OU/InterWeave/tree/main/packaging/linux
   Main PID: 4183263 (transport-daemo)
      Tasks: 12 (limit: 7680)
     Memory: 1.9M (peak: 2.9M)
        CPU: 23ms
     CGroup: /user.slice/user-1016.slice/user@1016.service/app.slice/app-interweave\x2ddaemon.slice/interweave-daemon@b.service
             └─4183263 $SCRATCH/home/.local/bin/transport-daemon --profile b

Oct 22 10:19:15 develop-qzapp systemd[2354]: Starting interweave-daemon@b.service - InterWeave transport daemon, profile b...
Oct 22 10:19:15 develop-qzapp systemd[2354]: Started interweave-daemon@b.service - InterWeave transport daemon, profile b.
Oct 22 10:19:15 develop-qzapp transport-daemon[4183263]: 2026-10-22T08:19:15.962457Z  INFO transport_daemon::daemon: starting profile="b" peer="12D3KooWKjUQX5PtHWJDqKMeFbTjvrMUtYoqipr6CXrNJ1tzJLPm"
Oct 22 10:19:16 develop-qzapp transport-daemon[4183263]: 2026-10-22T08:19:16.031197Z  INFO transport_daemon::daemon: serving data=/run/user/1016/iwrec/interweave/b.sock
(exit 0)

### 5. check it
$ transportctl --profile a status
peer         12D3KooWP6JKKVGLN6JqUD68icL4L1JP9p3nAfportwLWFeTf4mj
health       Healthy
connections  0 data, 1 admin
leases       0
inbound      direct Unknown, relay Unavailable (0/0 reservations)
(exit 0)
$ transportctl --profile a peers list
12D3KooWKjUQX5PtHWJDqKMeFbTjvrMUtYoqipr6CXrNJ1tzJLPm  connected  last connected
(exit 0)
$ transportctl --profile a trust list
local    12D3KooWP6JKKVGLN6JqUD68icL4L1JP9p3nAfportwLWFeTf4mj
allowed  12D3KooWKjUQX5PtHWJDqKMeFbTjvrMUtYoqipr6CXrNJ1tzJLPm  configured
every other peer is denied
(exit 0)
$ transportctl --profile a endpoints list
claude                   enabled, default
(exit 0)
$ transportctl --profile b status
peer         12D3KooWKjUQX5PtHWJDqKMeFbTjvrMUtYoqipr6CXrNJ1tzJLPm
health       Healthy
connections  0 data, 1 admin
leases       0
inbound      direct Unknown, relay Unavailable (0/0 reservations)
(exit 0)
$ transportctl --profile b peers list
12D3KooWP6JKKVGLN6JqUD68icL4L1JP9p3nAfportwLWFeTf4mj  connected  last connected
(exit 0)
$ transportctl --profile b trust list
local    12D3KooWKjUQX5PtHWJDqKMeFbTjvrMUtYoqipr6CXrNJ1tzJLPm
allowed  12D3KooWP6JKKVGLN6JqUD68icL4L1JP9p3nAfportwLWFeTf4mj  configured
every other peer is denied
(exit 0)
$ transportctl --profile b endpoints list
claude                   enabled, default
(exit 0)

### 6. logs
$ jc --no-pager -o short-iso -u interweave-daemon@a -u interweave-daemon@b --since -2min
2026-10-22T10:19:15+02:00 develop-qzapp systemd[2354]: Starting interweave-daemon@a.service - InterWeave transport daemon, profile a...
2026-10-22T10:19:15+02:00 develop-qzapp systemd[2354]: Starting interweave-daemon@b.service - InterWeave transport daemon, profile b...
2026-10-22T10:19:15+02:00 develop-qzapp systemd[2354]: Started interweave-daemon@a.service - InterWeave transport daemon, profile a.
2026-10-22T10:19:15+02:00 develop-qzapp transport-daemon[4183261]: 2026-10-22T08:19:15.936360Z  INFO transport_daemon::daemon: starting profile="a" peer="12D3KooWP6JKKVGLN6JqUD68icL4L1JP9p3nAfportwLWFeTf4mj"
2026-10-22T10:19:15+02:00 develop-qzapp systemd[2354]: Started interweave-daemon@b.service - InterWeave transport daemon, profile b.
2026-10-22T10:19:15+02:00 develop-qzapp transport-daemon[4183261]: 2026-10-22T08:19:15.956579Z  INFO transport_daemon::daemon: serving data=/run/user/1016/iwrec/interweave/a.sock
2026-10-22T10:19:15+02:00 develop-qzapp transport-daemon[4183263]: 2026-10-22T08:19:15.962457Z  INFO transport_daemon::daemon: starting profile="b" peer="12D3KooWKjUQX5PtHWJDqKMeFbTjvrMUtYoqipr6CXrNJ1tzJLPm"
2026-10-22T10:19:16+02:00 develop-qzapp transport-daemon[4183263]: 2026-10-22T08:19:16.031197Z  INFO transport_daemon::daemon: serving data=/run/user/1016/iwrec/interweave/b.sock
(exit 0)

### restart by SIGHUP: stops as on SIGTERM, exit 75, restarted 5 s later
$ sc kill --kill-whom=main --signal=SIGHUP interweave-daemon@a
(exit 0)
MainPID before 4183261, after 4185408; ActiveState active, NRestarts 1
$ transportctl --profile a peers list
12D3KooWKjUQX5PtHWJDqKMeFbTjvrMUtYoqipr6CXrNJ1tzJLPm  connected  last connected
(exit 0)

### stop and disable
$ sc disable --runtime --now interweave-daemon@a interweave-daemon@b
Removed '/run/user/1016/systemd/user/interweave-daemon@b.service'.
Removed '/run/user/1016/systemd/user/default.target.wants/interweave-daemon@b.service'.
Removed '/run/user/1016/systemd/user/default.target.wants/interweave-daemon@a.service'.
Removed '/run/user/1016/systemd/user/interweave-daemon@a.service'.
(exit 0)
$ transportctl --profile a status
transportctl: no daemon is running for profile "a"
(exit 3)
```
