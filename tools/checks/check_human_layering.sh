#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/checks/check_human_layering.sh
#
# >>> help
# Do the human application's crates keep to their layers?
#
#   tools/checks/check_human_layering.sh
#
# Plan §17's P2, as corrected with architect-cto (relay 01a0f76d, 2026-10-01):
#   1. crates/human/core, chat-protocol, store, transport-client, ui-model
#      and client-api reach nothing under crates/transport/*, no libp2p
#      crate and no Slint crate;
#   2. crates/human/ui-model and client-api reach no rusqlite — client-api
#      is the facade's caller-facing vocabulary, types only, and ui-model
#      may name it because it holds no storage (architect-cto, relay seq
#      10633);
#   3. among crates/human/*, only ui-slint reaches Slint;
#   4. no workspace member but crates/human/ui-slint DECLARES a Slint crate
#      itself: an app (apps/human-desktop, apps/human-android) reaches Slint
#      only through ui-slint, the composition-root rule of ADR-0045 seen from
#      the other side;
#   5. crates/human/app-core, the headless root, keeps rule 1, reaches no
#      tokio, and DECLARES no rusqlite itself: plan §18's P1 and its
#      decision (2) (architect-cto's Q1 ruling, relay seq 11163). The root
#      is shared with Android at Stage 17, so the runtime is the app's to
#      choose; and it reaches storage only through human-store, which holds
#      rusqlite -- so rusqlite is refused as a direct dependency, not
#      walked, which is decision (2)'s "no rusqlite directly". P1 first
#      read "no rusqlite in its normal and build graphs", which app-core
#      cannot meet while it depends on human-store; architect-cto ruled
#      for decision (2) and is amending P1's text to match on a branch
#      of its own, folded beside this rule (relay 01a10614, 2026-10-04).
#   6. apps/human-android, the release library, reaches no test-only
#      crate (interweave-android-e2e-cases): the instrumentation builds
#      that crate into the androidTest APK only, and the release APK's
#      graph names it nowhere (architect-cto's DECISION 01a12770, plan
#      §20 gate (c)). This is the Rust half; the test-only JNI entry
#      (org.interweave.human.test.E2eCase) is a Java class this walk
#      cannot see, checked on the APK once the instrumentation build
#      exists.
#   7. apps/human-cli, the command-line client, reaches no libp2p crate and
#      no Slint crate: the claim its Cargo.toml makes (the transport is the
#      shared daemon's, ADR-0040, and it has no display). Walked, as rule 1
#      is, so a dependency that brings either in transitively fails too;
#      rule 4 alone would see only a Slint crate it declared itself.
# check_ipc_layering.sh is the precedent; this is its shape, wider.
#
# WHAT IS FOLLOWED: the normal and build dependencies cargo resolves with
# --all-features, to any depth (a dependency optional behind a feature is
# linked by a build that enables it). A dev-dependency is not followed: a
# test may reach for anything, and it is not compiled into the library. A
# crate is placed by where its Cargo.toml sits, so a new crate is caught
# without this file naming it; libp2p by package name (`libp2p`,
# `libp2p-*`); Slint by package name (`slint`, `slint-*`, and the
# `i-slint-*` crates it is built from); rusqlite by `rusqlite`; tokio by
# `tokio` (a `tokio-*` crate that links the runtime reaches `tokio` itself,
# and one that does not is not the runtime).
#
# A LISTED CRATE (rule 1's six, rule 5's app-core and rule 7's CLI) THAT IS NOT A WORKSPACE MEMBER passes only
# while [workspace.metadata.interweave].planned_members names it: ui-model
# before its batch; transport-client once Stage 14's batch 2 plans it,
# which this check therefore needs first. Absent from both, the guard would
# pass having checked nothing, so that is exit 2.
#
# Exit codes:
#   0  every rule holds
#   1  one does not; the path from the crate to the offender is printed
#   2  a failure to check: cargo metadata failed or returned no resolved
#      graph, a guarded crate has no node in it, planned_members could not
#      be read, a listed crate is neither a member nor planned, or the load
#      or the walk raised an error it did not expect
# <<< help

set -uo pipefail

ROOT="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )/../.." && pwd )"
cd "$ROOT" || exit 2

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
fi

# stderr apart from the JSON, as check_ipc_layering.sh: a successful run
# may still print an index update or a manifest warning, and one such line
# in the document would read as a breach. Shown only when cargo fails.
err="$(mktemp)"; trap 'rm -f "$err"' EXIT
meta="$(cargo metadata --format-version 1 --locked --all-features 2>"$err")" || {
    echo "check_human_layering: cargo metadata failed:" >&2
    tail -5 "$err" >&2
    exit 2
}

read -r -d '' walk <<'PYEOF'
import os, subprocess, sys

me = "check_human_layering"

def load():
    """The graph and the roster, as module globals. Inside the try below, so
    an error here (a Python without tomllib, a package with no manifest
    path) is a failure to check, exit 2 — never exit 1, which is a breach."""
    global ws_root, planned, packages, nodes, member_at
    import json, tomllib
    root = os.path.realpath(sys.argv[1])
    try:
        meta = json.load(sys.stdin)
    except ValueError as e:
        print(f"{me}: cargo metadata's output is not JSON ({e})", file=sys.stderr)
        sys.exit(2)
    if not (meta.get("resolve") or {}).get("nodes"):
        print(f"{me}: cargo metadata has no resolved dependency graph", file=sys.stderr)
        sys.exit(2)
    ws_root = os.path.realpath(meta.get("workspace_root", root))
    try:
        with open(os.path.join(ws_root, "Cargo.toml"), "rb") as f:
            planned = set(tomllib.load(f)["workspace"]["metadata"]["interweave"]["planned_members"])
    except (OSError, KeyError, TypeError, tomllib.TOMLDecodeError) as e:
        print(f"{me}: cannot read [workspace.metadata.interweave].planned_members ({type(e).__name__}: {e})", file=sys.stderr)
        sys.exit(2)
    packages = {p["id"]: p for p in meta["packages"]}
    members = [i for i in meta.get("workspace_members", []) if i in packages]
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    member_at = {rel_dir(packages[i]): i for i in members}

def rel_dir(pkg):
    return os.path.relpath(os.path.dirname(os.path.realpath(pkg["manifest_path"])), ws_root)

TRANSPORT = "crates/transport" + os.sep
UI_SLINT = "crates/human/ui-slint"

def is_slint(name):
    return name == "slint" or name.startswith("slint-") or name.startswith("i-slint-")

def is_libp2p(name):
    return name == "libp2p" or name.startswith("libp2p-")

def under_transport(pkg):
    return (rel_dir(pkg) + os.sep).startswith(TRANSPORT)

def runtime_deps(node):
    # A dependency counts when any of its kinds is normal (null) or build.
    for d in node.get("deps", []):
        if any(k.get("kind") in (None, "build") for k in d.get("dep_kinds", [])):
            yield d["pkg"]

bad = 0

def walk(start, offence):
    """Breadth-first from start; each offender printed once, by its shortest path."""
    global bad
    if start not in nodes:
        print(f"{me}: {packages[start]['name']} has no node in the resolved graph", file=sys.stderr)
        sys.exit(2)
    parent, queue, found = {start: None}, [start], 0
    while queue:
        cur = queue.pop(0)
        for dep in runtime_deps(nodes.get(cur, {})):
            if dep in parent:
                continue
            parent[dep] = cur
            why = offence(packages[dep])
            if why:
                chain, at = [], dep
                while at is not None:
                    chain.append(packages[at]["name"])
                    at = parent[at]
                print(f"FAIL: {rel_dir(packages[start])} depends on {why}: {' -> '.join(reversed(chain))}")
                found += 1
                continue  # the offender is enough; do not walk into it
            queue.append(dep)
    bad += found
    return len(parent) - 1, found

def runtime_free(pkg):
    if under_transport(pkg):
        return "a crate under crates/transport/"
    if is_libp2p(pkg["name"]):
        return "a libp2p crate"
    if is_slint(pkg["name"]):
        return "a Slint crate (only crates/human/ui-slint may)"
    return None

def model_free(pkg):
    return runtime_free(pkg) or ("rusqlite (ui-model and client-api hold no storage)" if pkg["name"] == "rusqlite" else None)

def headless_free(pkg):
    return runtime_free(pkg) or ("tokio (the headless root chooses no runtime)" if pkg["name"] == "tokio" else None)

def cli_free(pkg):
    if is_libp2p(pkg["name"]):
        return "a libp2p crate (the CLI's transport is the shared daemon's)"
    if is_slint(pkg["name"]):
        return "a Slint crate (the CLI has no display)"
    return None

def slint_free(pkg):
    return "a Slint crate (only crates/human/ui-slint may)" if is_slint(pkg["name"]) else None

GUARDED = [
    ("crates/human/core", runtime_free),
    ("crates/human/chat-protocol", runtime_free),
    ("crates/human/store", runtime_free),
    ("crates/human/transport-client", runtime_free),
    ("crates/human/ui-model", model_free),
    ("crates/human/client-api", model_free),
    ("crates/human/app-core", headless_free),
    ("apps/human-cli", cli_free),
]
APP_CORE = "crates/human/app-core"
HUMAN_ANDROID = "apps/human-android"
TEST_ONLY = {"interweave-android-e2e-cases"}

def release_free(pkg):
    if pkg["name"] in TEST_ONLY:
        return "a test-only crate (the androidTest APK's, never the release one)"
    return None


def release_build(name):
    """The crate names a build of `name` itself compiles, as cargo resolves
    its features for that package alone (cargo tree -p, every target,
    normal and build edges). The all-features graph the other rules walk
    cannot answer this: there an optional test dependency another crate
    turns on (android-platform's e2e-cases, which tests/android-e2e enables)
    looks linked into the release library when it is not."""
    out = subprocess.run(
        ["cargo", "tree", "-p", name, "-e", "normal,build", "--target", "all",
         "--locked", "--prefix", "none", "-f", "{p}"],
        capture_output=True, text=True, timeout=600)
    if out.returncode != 0:
        print(f"{me}: cargo tree -p {name} failed:\n{out.stderr.strip()}", file=sys.stderr)
        sys.exit(2)
    return {line.split()[0] for line in out.stdout.splitlines() if line.strip()}


def main():
    global bad
    # Rules 1, 2, 5 and 7: the listed crates.
    for d, offence in GUARDED:
        start = member_at.get(d)
        if start is None:
            if d in planned:
                print(f"{me}: {d} is planned, not a member yet — nothing to check for it")
                continue
            print(f"{me}: {d} is neither a workspace member nor in planned_members — the guard would check nothing", file=sys.stderr)
            sys.exit(2)
        n, found = walk(start, offence)
        if not found:
            print(f"{me}: {d} keeps to its layer ({n} runtime dependencies walked)")
    # Rule 5's direct half: app-core declares no rusqlite.
    start = member_at.get(APP_CORE)
    if start is not None:
        for dep in runtime_deps(nodes.get(start, {})):
            if packages[dep]["name"] == "rusqlite":
                print(f"FAIL: {APP_CORE} declares rusqlite itself — the headless root reaches storage through human-store")
                bad += 1
    # Rule 3: every other member under crates/human/ but ui-slint.
    listed = {d for d, _ in GUARDED}
    for d, start in sorted(member_at.items()):
        if d.startswith("crates/human/") and d != UI_SLINT and d not in listed:
            n, found = walk(start, slint_free)
            if not found:
                print(f"{me}: {d} reaches no Slint crate ({n} runtime dependencies walked)")
    # Rule 4: a direct Slint dependency anywhere but ui-slint.
    for d, start in sorted(member_at.items()):
        if d == UI_SLINT:
            continue
        for dep in runtime_deps(nodes.get(start, {})):
            if is_slint(packages[dep]["name"]):
                print(f"FAIL: {d} declares {packages[dep]['name']} itself — only {UI_SLINT} may; an app reaches Slint through it")
                bad += 1
    # Rule 6: the release library's own build reaches no test-only crate.
    start = member_at.get(HUMAN_ANDROID)
    if start is not None:
        unknown = TEST_ONLY - {p["name"] for p in packages.values()}
        if unknown:
            print(f"{me}: no package named {', '.join(sorted(unknown))} — renamed? rule 6 would check nothing", file=sys.stderr)
            sys.exit(2)
        built = release_build(packages[start]["name"])
        if built & TEST_ONLY:
            # It is in the build: name the path through the graph.
            n, found = walk(start, release_free)
            if not found:
                print(f"FAIL: {HUMAN_ANDROID}'s build compiles {', '.join(sorted(built & TEST_ONLY))}")
                bad += 1
        else:
            print(f"{me}: {HUMAN_ANDROID} reaches no test-only crate ({len(built)} crates in its own build)")
    if bad:
        print()
        print("The human application layers: core, chat-protocol, store, transport-client,")
        print("ui-model and client-api know no transport runtime, no libp2p and no Slint;")
        print("ui-model and client-api hold no storage; app-core runs on no tokio and")
        print("declares no rusqlite; Slint is ui-slint's alone, and an app composes ui-slint;")
        print("the release Android library reaches no test-only crate; the CLI reaches")
        print("no libp2p and no Slint.")
        sys.exit(1)

# Exit 1 means a breach and nothing else: an error the load or the walk did
# not expect is a failure to check, exit 2. SystemExit is not an Exception.
try:
    load()
    main()
except Exception as e:
    print(f"{me}: could not walk cargo metadata ({type(e).__name__}: {e})", file=sys.stderr)
    sys.exit(2)
PYEOF
python3 -c "$walk" "$ROOT" <<<"$meta"
