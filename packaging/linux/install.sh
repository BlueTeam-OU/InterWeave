#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# Install InterWeave for the account that runs this: the three executables
# into ~/.local/bin and the two systemd user-unit templates into
# ${XDG_CONFIG_HOME:-~/.config}/systemd/user. Nothing system-wide, no root.
# See README.md (plan section 22, packaging/linux).
#
# It ENABLES NOTHING: starting a profile's daemon at login is the person's
# explicit preference (human-client-packaging.md section 2), given with
# `systemctl --user enable --now interweave-daemon@<profile>`. And it touches
# no profile: config.yaml, the key and every state directory are the
# person's, before an install and after an uninstall alike.
#
# Usage: install.sh [BIN_DIR]
#   BIN_DIR holds transport-daemon, transportctl and human-desktop; it
#   defaults to the bin/ directory beside this script, as the release
#   archive lays it out. The unit templates are read from the systemd/
#   directory beside this script.
set -eu

usage() {
    echo "usage: install.sh [BIN_DIR]" >&2
    exit 2
}

case "${1-}" in
-h | --help)
    sed -n '5,20p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
-*) usage ;;
esac
[ "$#" -le 1 ] || usage

here=$(cd "$(dirname "$0")" && pwd)
bin_dir=${1:-"$here/bin"}
unit_src="$here/systemd"
# The units name %h/.local/bin, so this is the one place they look.
bin_dest="$HOME/.local/bin"
unit_dest="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

# Everything is checked before anything is written, so a missing file
# leaves no half install behind.
for exe in transport-daemon transportctl human-desktop; do
    if [ ! -f "$bin_dir/$exe" ] || [ ! -x "$bin_dir/$exe" ]; then
        echo "install.sh: no executable $bin_dir/$exe" >&2
        exit 1
    fi
done
for unit in interweave-daemon@.service interweave-client@.service; do
    if [ ! -f "$unit_src/$unit" ]; then
        echo "install.sh: no unit template $unit_src/$unit" >&2
        exit 1
    fi
done
# A directory at a name written below would take the file inside it (cp and
# mv both write into a directory) and the install would report success with
# nothing at the name a unit starts, so it is refused here, before anything
# is written. Anything else at a temporary name -- a symlink, a hard link, a
# FIFO, a file an interrupted install left -- is removed before the copy,
# which would otherwise write through it; a symlink at a final name is
# replaced, not followed (mv -T).
refuse_directory() {
    if [ -d "$1" ] && [ ! -L "$1" ]; then
        echo "install.sh: $1 is a directory in the way; nothing was installed" >&2
        exit 1
    fi
}
for exe in transport-daemon transportctl human-desktop; do
    refuse_directory "$bin_dest/$exe"
    refuse_directory "$bin_dest/.$exe.install"
done
for unit in interweave-daemon@.service interweave-client@.service; do
    refuse_directory "$unit_dest/$unit"
    refuse_directory "$unit_dest/.$unit.install"
done

umask 077
mkdir -p "$bin_dest" "$unit_dest"
for exe in transport-daemon transportctl human-desktop; do
    # Copied to a temporary name and renamed over the old one, so a daemon
    # running the old binary keeps its inode and an upgrade never leaves a
    # half-written executable at the name a unit starts.
    rm -f "$bin_dest/.$exe.install"
    cp "$bin_dir/$exe" "$bin_dest/.$exe.install"
    chmod 0755 "$bin_dest/.$exe.install"
    mv -fT "$bin_dest/.$exe.install" "$bin_dest/$exe"
done
for unit in interweave-daemon@.service interweave-client@.service; do
    rm -f "$unit_dest/.$unit.install"
    cp "$unit_src/$unit" "$unit_dest/.$unit.install"
    chmod 0644 "$unit_dest/.$unit.install"
    mv -fT "$unit_dest/.$unit.install" "$unit_dest/$unit"
done

# A user manager that is not running (a container, a session without one)
# reads the units when it next starts; this install is complete without it.
if systemctl --user show-environment >/dev/null 2>&1; then
    systemctl --user daemon-reload
else
    echo "install.sh: no systemd user manager reachable; the units load when it starts" >&2
fi
echo "installed into $bin_dest and $unit_dest; nothing is enabled"
