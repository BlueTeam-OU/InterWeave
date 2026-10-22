#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# Uninstall InterWeave for the account that runs this: stop and disable
# every interweave-daemon@ and interweave-client@ instance, then remove the
# two unit templates and the four programs install.sh put in place.
# See README.md (plan section 22, packaging/linux).
#
# IT NEVER REMOVES A PROFILE. Every profile's config.yaml, identity key,
# state, human store, logs and peer cache stay where they are: a removed
# key is a lost PeerId, recoverable only from its mnemonic, so deleting one
# is the person's own act, by hand (README.md names the directories). A
# reinstall serves the same PeerId from the kept key. Held by
# tests/desktop-e2e/tests/user_units.rs's uninstall case under a user
# manager, which compares every profile file and directory before and
# after.
#
# A shell that cannot reach the account's user manager (sudo -u, su -c,
# sudo -E, cron) -- no XDG_RUNTIME_DIR, or another account's -- has the
# manager looked up in this account's own /run/user/<uid>; if a manager
# is running there or under the inherited directory and still cannot be
# reached, nothing is removed and the script fails, since it could not
# stop what runs. With no manager running at all, the instances'
# dependency links are removed by hand.
#
# Usage: uninstall.sh
set -eu

case "${1-}" in
-h | --help)
    sed -n '5,27p' "$0" | sed 's/^# \{0,1\}//'
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

# The kinds of dependency link that start an instance with another unit:
# .wants (`enable`, `add-wants`), .requires (`add-requires`, RequiredBy=)
# and .upholds (UpheldBy=). Each is looked for in the persistent and the
# runtime configuration.
link_kinds="wants requires upholds"

# The instances: every one in $1, the manager's list of the units it has
# loaded, running or not (empty when no manager runs), and
# every one linked -- persistently or for this boot only -- that it has
# not loaded, found by those links. Sorted, so every client comes before
# every daemon and none is restarted by its daemon's stop.
instances() {
    {
        printf '%s\n' "$1" | awk 'NF { print $1 }'
        for kind in $link_kinds; do
            for dir in "$unit_dest"/*."$kind" \
                "${XDG_RUNTIME_DIR:-/nonexistent}"/systemd/user/*."$kind"; do
                [ -d "$dir" ] || continue
                for link in "$dir"/interweave-client@?*.service \
                    "$dir"/interweave-daemon@?*.service; do
                    if [ -L "$link" ]; then basename "$link"; fi
                done
            done
        done
    } | sort -u
}

# Every dependency link naming $1, of every kind, persistent and runtime,
# removed. `disable` leaves one whose template is already gone (it refuses
# a unit it cannot find), and a link left behind would make a later
# install.sh start the instance again without anyone asking.
remove_links() {
    for kind in $link_kinds; do
        for link in "$unit_dest"/*."$kind"/"$1" \
            "${XDG_RUNTIME_DIR:-/nonexistent}"/systemd/user/*."$kind"/"$1"; do
            if [ -L "$link" ]; then
                rm -f "$link"
                echo "removed the link $link"
            fi
        done
    done
}

# This account's own runtime directory, whatever the shell inherited: an
# XDG_RUNTIME_DIR kept from another account (`su -c`, `sudo -E`) or stale
# points at a manager that is not this account's. An inherited bus
# address does not matter once it is set: `systemctl --user` reaches the
# manager through $XDG_RUNTIME_DIR/systemd/private (the uninstall case
# runs with a foreign one).
own="/run/user/$(id -u)"
inherited="${XDG_RUNTIME_DIR-}"
if ! systemctl --user show-environment >/dev/null 2>&1 \
    && [ "$inherited" != "$own" ] && [ -d "$own" ]; then
    XDG_RUNTIME_DIR="$own"
    export XDG_RUNTIME_DIR
fi
if systemctl --user show-environment >/dev/null 2>&1; then
    manager=yes
elif [ -S "$own/systemd/private" ] \
    || { [ -n "$inherited" ] && [ -S "$inherited/systemd/private" ]; }; then
    echo "uninstall.sh: this account's systemd user manager is running, but this shell cannot reach it, so no instance could be stopped; nothing was removed. Run uninstall.sh from the account's own login session." >&2
    exit 1
else
    manager=no
fi

if [ "$manager" = yes ]; then
    # A manager that answered and then fails a query is not one with
    # nothing running: a failed list, or a failed question about one
    # instance, removes nothing more, where an unchecked one would read as
    # no instance, or as one stopped (`is-active` fails both ways alike).
    if ! loaded="$(systemctl --user list-units --all --plain --no-legend \
        --full 'interweave-client@*.service' 'interweave-daemon@*.service')"; then
        echo "uninstall.sh: this account's systemd user manager did not list its instances, so none could be stopped; nothing was removed" >&2
        exit 1
    fi
    for unit in $(instances "$loaded"); do
        # Stopped whether or not it was enabled -- a stop of an instance the
        # manager has not loaded fails, and is judged by what is active
        # after it -- then disabled, persistent and runtime.
        systemctl --user stop "$unit" 2>/dev/null || true
        if ! state="$(systemctl --user show -p ActiveState --value "$unit")"; then
            echo "uninstall.sh: the manager did not say whether $unit still runs; nothing more was removed" >&2
            exit 1
        fi
        case "$state" in
        inactive | failed) ;;
        *)
            echo "uninstall.sh: $unit could not be stopped ($state); nothing more was removed" >&2
            exit 1
            ;;
        esac
        systemctl --user disable "$unit" >/dev/null 2>&1 || true
        systemctl --user disable --runtime "$unit" >/dev/null 2>&1 || true
        remove_links "$unit"
        echo "stopped and disabled $unit"
    done
else
    # No manager runs for this account, so no instance runs under one; what
    # is left of an enable, or of an add-wants or add-requires, is its links.
    echo "uninstall.sh: no systemd user manager is running for this account" >&2
    for unit in $(instances ""); do
        remove_links "$unit"
    done
fi

for unit in interweave-daemon@.service interweave-client@.service; do
    rm -f "$unit_dest/$unit"
done
for exe in transport-daemon transportctl human-desktop interweave-human; do
    rm -f "$bin_dest/$exe"
done
if [ "$manager" = yes ]; then
    systemctl --user daemon-reload
    systemctl --user reset-failed 'interweave-client@*.service' \
        'interweave-daemon@*.service' >/dev/null 2>&1 || true
fi
echo "uninstalled; every profile's configuration, key and state are kept"
