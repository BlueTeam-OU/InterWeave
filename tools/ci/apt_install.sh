#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/apt_install.sh
#
# >>> help
# Install a CI job's apt packages so a slow or stalled mirror costs
# retries rather than the job, with the downloaded .debs kept in a
# directory actions/cache can hold.
#
#   tools/ci/apt_install.sh key <list>
#   tools/ci/apt_install.sh install [--cache <dir>] <list>
#
# <list> is a file of package names, one a line; a `#` starts a comment
# and blank lines are skipped. A name outside Debian's grammar
# ([a-z0-9][a-z0-9+.-]+) is refused before apt runs.
#
# key      prints the cache key: apt-<os id>-<os version>-<arch>-<hash of
#          the sorted package names>-<ISO week>. So the packages, never the
#          file's comments or order, key the cache (the owner, 2026-10-26),
#          and a new runner image is a new key, since a .deb of another
#          release is no use. Everything before the week is the prefix a
#          restore falls back on: the same packages' newest cache. The week
#          is what makes a save happen at all once a key has hit, so a
#          mirror's newer versions reach the cache weekly rather than being
#          downloaded on every run; what a save carries stays bounded
#          because the install's autoclean drops each .deb the mirror has
#          stopped offering.
#
# install  runs `apt-get update` and `apt-get install -y -q
#          --no-install-recommends <packages>`, both with
#          Acquire::Retries=5 and an http/https timeout of 30 s (apt's
#          defaults: 3 retries, 120 s), so a connection that stops
#          answering is dropped and retried within a minute. A connection
#          that keeps trickling bytes is not one a timeout cuts; the cache
#          is what saves that run. With --cache, apt reads and downloads
#          .debs in <dir> (Dir::Cache::Archives), so a restored cache is
#          installed from without a download; afterwards it drops what the
#          mirror no longer offers (autoclean), removes apt's partial/ and
#          lock, and gives <dir> to the calling user, so actions/cache can
#          read it. Under that <dir> apt may note that it downloads
#          unsandboxed (its _apt user cannot reach a runner's home): a
#          notice, not a failure.
#          As root (a job's container) apt runs directly, otherwise under
#          `sudo -n`.
#
# Exit codes:
#   0  printed / installed
#   1  apt-get failed (its status is printed)
#   2  usage: no such list, an empty list, a name apt would not take
# <<< help

set -euo pipefail

die() { echo "apt_install: $1" >&2; exit "${2:-2}"; }

case "${1:-}" in
    -h|--help) sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'; exit 0 ;;
esac

# The packages of <list>, sorted and unique, one a line.
packages() {
    local list="$1" names bad
    [[ -f "$list" ]] || die "$list is not a file"
    names="$(sed 's/#.*//' "$list" | tr -s '[:space:]' '\n' | sed '/^$/d' | LC_ALL=C sort -u)"
    [[ -n "$names" ]] || die "$list names no package"
    bad="$(grep -Ev '^[a-z0-9][a-z0-9+.-]+$' <<<"$names" || true)"
    [[ -z "$bad" ]] || die "$list: not a package name: $(tr '\n' ' ' <<<"$bad")"
    printf '%s\n' "$names"
}

cmd="${1:-}"; shift || true
case "$cmd" in
    key)
        [[ $# -eq 1 ]] || die "usage: $0 key <list> (try --help)"
        names="$(packages "$1")"
        os_release="${APT_INSTALL_OS_RELEASE:-/etc/os-release}"
        [[ -r "$os_release" ]] || die "cannot read $os_release"
        # shellcheck source=/dev/null
        os="$(. "$os_release"; echo "${ID:-}-${VERSION_ID:-}")"
        [[ "$os" != -* && "$os" != *- ]] || die "$os_release names no ID and VERSION_ID"
        hash="$(sha256sum <<<"$names" | cut -c1-16)"
        echo "apt-$os-$(uname -m)-$hash-$(date -u +%G-W%V)"
        ;;
    install)
        cache=
        if [[ "${1:-}" == --cache ]]; then
            [[ $# -ge 2 && -n "$2" ]] || die "usage: $0 install [--cache <dir>] <list>"
            cache="$2"; shift 2
        fi
        [[ $# -eq 1 ]] || die "usage: $0 install [--cache <dir>] <list> (try --help)"
        list="$(packages "$1")"
        mapfile -t names <<<"$list"

        as_root=()
        [[ "$(id -u)" -eq 0 ]] || as_root=(sudo -n)
        opts=(-o Acquire::Retries=5 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30)
        if [[ -n "$cache" ]]; then
            mkdir -p "$cache"
            cache="$(cd "$cache" && pwd)"
            opts+=(-o "Dir::Cache::Archives=$cache/")
            echo "apt_install: $(find "$cache" -maxdepth 1 -name '*.deb' | wc -l) .deb(s) in $cache before the install"
        fi
        apt() {
            "${as_root[@]}" env DEBIAN_FRONTEND=noninteractive apt-get "${opts[@]}" "$@" \
                || die "apt-get $1 failed (status $?)" 1
        }
        apt update -q
        apt install -y -q --no-install-recommends "${names[@]}"
        if [[ -n "$cache" ]]; then
            apt autoclean -q
            "${as_root[@]}" rm -rf "$cache/partial" "$cache/lock"
            "${as_root[@]}" chown -R "$(id -u):$(id -g)" "$cache"
            echo "apt_install: $(find "$cache" -maxdepth 1 -name '*.deb' | wc -l) .deb(s) in $cache after it"
        fi
        ;;
    *) die "usage: $0 key <list> | install [--cache <dir>] <list> (try --help)" ;;
esac
