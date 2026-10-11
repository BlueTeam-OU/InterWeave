#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/gh/actions-health.sh
#
# >>> help
# Is it worth spending CI minutes right now? GitHub Actions' status and
# the organisation's included allowance, in one line and one exit code
# (0 healthy, 1 degraded, 2 could not find out).
#
# actions-health lives in agent-fabric, the control plane checked out
# beside this working copy: tools/fabric/github/actions_health.py. This file
# only locates it and hands it the arguments untouched, so every
# documented invocation (tools/gh/actions-health.sh, --quiet, --json,
# --included N, --org NAME) keeps working.
#
# The allowance setting is the fabric's own name,
# AGENT_FABRIC_ACTIONS_INCLUDED_MINUTES, which fabric-pr wait-merged reads
# too; nothing here maps or renames it (InterWeave's former
# INTERWEAVE_ACTIONS_INCLUDED_MINUTES is retired with the forwarders). It
# stays a script because fabric-pr has no verb for it. This
# repository is public: its own runs on standard runners bill nothing,
# which the fabric's check reports as such.
#
# It refuses with exit 2 when agent-fabric is not beside this working
# copy, and maps the fabric wrapper's 127 (its pinned Python missing) to
# 2: this script answers 0, 1 or 2.
#
#   tools/gh/actions-health.sh --help      # the real script's help
# <<< help
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tools/gh/fabric-root.sh
. "$here/fabric-root.sh"
fabric="$(interweave_fabric_root "$here")"
target="$fabric/tools/fabric/github/actions_health.py"
[[ -f "$target" ]] || {
    echo "actions-health: agent-fabric not found at $fabric (expected beside this working copy, as projects/agent-fabric, carrying tools/fabric/github/actions_health.py); set AGENT_FABRIC_ROOT or update it. See CLAUDE.md §9, agent-fabric beside the checkout." >&2
    exit 2
}
py="$(interweave_fabric_python actions-health)" || exit 2
rc=0; "$py" "$target" "$@" || rc=$?
[[ $rc -eq 127 ]] && exit 2
exit "$rc"
