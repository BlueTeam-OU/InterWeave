#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# Uninstall InterWeave for the account that runs this: stop and disable
# every interweave-daemon@ and interweave-client@ instance, then remove the
# two unit templates and the three executables install.sh put in place.
# See README.md (plan section 22, packaging/linux).
#
# IT NEVER REMOVES A PROFILE. Every profile's config.yaml, identity key,
# state, human store and logs stay where they are: a removed key is a lost
# PeerId, recoverable only from its mnemonic, so deleting one is the
# person's own act, by hand (README.md names the directories). A reinstall
# serves the same PeerId from the kept key.
#
# Usage: uninstall.sh
set -eu

case "${1-}" in
-h | --help)
    sed -n '5,16p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
"") ;;
*)
    echo "usage: uninstall.sh" >&2
    exit 2
    ;;
esac

bin_dest="$HOME/.local/bin"
unit_dest="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

# The instances: every one the manager has loaded, running or not, and
# every one enabled -- persistently or for this boot only -- that it has
# not loaded, found by the wants links `enable` makes. Sorted, so every
# client comes before every daemon and none is restarted by its daemon's
# stop.
instances() {
    {
        systemctl --user list-units --all --plain --no-legend --full \
            'interweave-client@*.service' 'interweave-daemon@*.service' |
            awk '{ print $1 }'
        for wants in "$unit_dest"/*.wants \
            "${XDG_RUNTIME_DIR:-/nonexistent}"/systemd/user/*.wants; do
            [ -d "$wants" ] || continue
            for link in "$wants"/interweave-client@?*.service \
                "$wants"/interweave-daemon@?*.service; do
                if [ -L "$link" ]; then basename "$link"; fi
            done
        done
    } | sort -u
}

if systemctl --user show-environment >/dev/null 2>&1; then
    for unit in $(instances); do
        systemctl --user disable --now "$unit" >/dev/null 2>&1 || true
        systemctl --user disable --runtime "$unit" >/dev/null 2>&1 || true
        # `disable --now` stops what it disables; a loaded instance that
        # was never enabled is stopped here.
        systemctl --user stop "$unit"
        echo "stopped and disabled $unit"
    done
    manager=yes
else
    echo "uninstall.sh: no systemd user manager reachable; no instance can be running under it" >&2
    manager=no
fi

for unit in interweave-daemon@.service interweave-client@.service; do
    rm -f "$unit_dest/$unit"
done
for exe in transport-daemon transportctl human-desktop; do
    rm -f "$bin_dest/$exe"
done
if [ "$manager" = yes ]; then
    systemctl --user daemon-reload
    systemctl --user reset-failed 'interweave-client@*.service' \
        'interweave-daemon@*.service' >/dev/null 2>&1 || true
fi
echo "uninstalled; every profile's configuration, key and state are kept"
