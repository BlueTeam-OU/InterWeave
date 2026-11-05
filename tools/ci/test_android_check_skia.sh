#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_android_check_skia.sh
#
# Self-test for the android check job's Skia steps, offline: ci.yml's own
# `Skia, pinned` and `Skia, checked` run blocks, read from the workflow
# and run as the runner runs a step (bash -e, the repository's root as the
# working directory), against a sandbox pin, Cargo.lock and archive
# (skia_prebuilt.sh's SKIA_PIN_* seams) and a build output laid out where
# `cargo check` puts skia-bindings'. skia_prebuilt.sh's refusals are its
# own self-test's; this one asks whether the JOB turns them into a red
# step: red on a wrong pin and on the source-build fallback, green on the
# right pin, the dev profile read, and the cargo cache keyed on the pin.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"
CI_YML="${ANDROID_CHECK_SKIA_CI_YML:-$ROOT/.github/workflows/ci.yml}"
[[ -f "$CI_YML" ]] || { echo "test: $CI_YML not found" >&2; exit 1; }

failures=0
SANDBOX="$(realpath -- "$(mktemp -d)")"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_android_check_skia: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# The job's step names in order, and the two run blocks, from the YAML.
python3 -I - "$CI_YML" "$SANDBOX" <<'PYEOF' || { echo "test: cannot read the android-check job from $CI_YML" >&2; exit 1; }
import sys, yaml
wf = yaml.safe_load(open(sys.argv[1]))
job = wf["jobs"]["android-check"]
out = sys.argv[2]
with open(out + "/names", "w") as f:
    for s in job["steps"]:
        f.write((s.get("name") or s.get("uses", "")) + "\n")
for s in job["steps"]:
    if s.get("name") == "Skia, pinned":
        open(out + "/pinned.sh", "w").write(s["run"])
    if s.get("name") == "Skia, checked":
        open(out + "/checked.sh", "w").write(s["run"])
key = job["steps"][[s.get("id") for s in job["steps"]].index("cargo-cache")]["with"]
open(out + "/cachekey", "w").write(key["key"] + "\n" + key["restore-keys"])
# What runs a step's text other than as written: an `if`, a
# continue-on-error or an env of its own on one of the three steps (a
# step's env beats GITHUB_ENV), the same on the job, or a Skia variable
# in the job's or the workflow's env.
shape = []
for s in job["steps"]:
    if s.get("name") in ("Skia, pinned", "cargo check for aarch64-linux-android", "Skia, checked") \
            and set(s) != {"name", "run"}:
        shape.append(f"step {s['name']!r} carries {sorted(set(s) - {'name', 'run'})}")
for k in ("if", "continue-on-error"):
    if k in job:
        shape.append(f"the job carries {k}")
for where, env in (("the job", job.get("env") or {}), ("the workflow", wf.get("env") or {})):
    shape += [f"{where}'s env sets {k}" for k in env if k.startswith(("SKIA_", "FORCE_SKIA"))]
open(out + "/shape", "w").write("".join(line + "\n" for line in shape))
PYEOF

echo "the android check job's steps:"
order="$(grep -nxE 'Skia, pinned|cargo check for aarch64-linux-android|Skia, checked' "$SANDBOX/names" | cut -d: -f2 | paste -sd'|')"
if [[ "$order" == "Skia, pinned|cargo check for aarch64-linux-android|Skia, checked" ]]; then
    pass "Skia, pinned runs before the cargo check and Skia, checked after it, once each"
else
    fail "the Skia steps are not pinned, cargo check, checked, once each" "$(cat "$SANDBOX/names")"
fi
[[ -f "$SANDBOX/pinned.sh" && -f "$SANDBOX/checked.sh" ]] || { echo "test: a Skia step has no run block" >&2; exit 1; }
# The runs below exercise each step's text; this is what would run that
# text otherwise, or not at all, in the job.
if [[ ! -s "$SANDBOX/shape" ]]; then
    pass "the three steps are name and run only; no if, continue-on-error or Skia env on the job or workflow"
else
    fail "something besides the run text decides the Skia steps" "$(cat "$SANDBOX/shape")"
fi
# Three hashFiles: the key and both restore keys, so a restore never
# brings back a skia-bindings output made under another pin.
n="$(grep -o "'tools/ci/skia-binaries.pin'" "$SANDBOX/cachekey" | wc -l)"
if [[ "$n" == 3 ]]; then
    pass "the cargo cache key and both restore keys hash the Skia pin"
else
    fail "the Skia pin is in $n of the cache key's three hashFiles" "$(cat "$SANDBOX/cachekey")"
fi

# The working directory a step runs in: the repository's tools, nothing
# else, so the step's relative `target` is the sandbox's.
W="$SANDBOX/work"
mkdir -p "$W" "$SANDBOX/source"
ln -s "$ROOT/tools" "$W/tools"
KEY="0123456789abcdef0123-aarch64-linux-android-ganesh-gl-pdf"
ARCHIVE="skia-binaries-$KEY.tar.gz"
printf 'stand-in archive bytes\n' > "$SANDBOX/source/$ARCHIVE"
SHA="$(sha256sum "$SANDBOX/source/$ARCHIVE" | cut -d' ' -f1)"
printf '[[package]]\nname = "skia-bindings"\nversion = "0.9.1"\nsource = "registry"\n' > "$SANDBOX/Cargo.lock"

# step <pinned|checked>: run the block as GitHub's default shell does.
step() {
    (cd "$W" && SKIA_PIN_FILE="$SANDBOX/pin" SKIA_PIN_LOCK="$SANDBOX/Cargo.lock" \
        SKIA_PIN_SOURCE="$SANDBOX/source" RUNNER_TEMP="$SANDBOX/runner" \
        GITHUB_ENV="$SANDBOX/runner/env" bash -e "$SANDBOX/$1.sh") >"$SANDBOX/out" 2>&1
}
# output <profile> <lines...>: a skia-bindings build output under target.
output() {
    local profile="$1"; shift
    rm -rf "$W/target"
    mkdir -p "$W/target/aarch64-linux-android/$profile/build/skia-bindings-1a2b3c"
    printf '%s\n' "$@" > "$W/target/aarch64-linux-android/$profile/build/skia-bindings-1a2b3c/output"
}
fresh() { rm -rf "$SANDBOX/runner"; mkdir -p "$SANDBOX/runner"; : > "$SANDBOX/runner/env"; }

echo "Skia, pinned:"
fresh
printf '0.9.1 %s %s\n' "$ARCHIVE" "$(printf '%064d' 0)" > "$SANDBOX/pin"
if step pinned; then
    fail "a pin whose SHA-256 is not the archive's: the step is green" "$(cat "$SANDBOX/out")"
elif [[ -s "$SANDBOX/runner/env" ]]; then
    fail "a wrong pin: red, but GITHUB_ENV was written" "$(cat "$SANDBOX/runner/env")"
else
    pass "a pin whose SHA-256 is not the archive's: red, GITHUB_ENV untouched"
fi
fresh
printf '0.9.1 %s %s\n' "$ARCHIVE" "$SHA" > "$SANDBOX/pin"
URL="file://$SANDBOX/runner/skia/$ARCHIVE"
if step pinned && [[ "$(cat "$SANDBOX/runner/env")" == "SKIA_BINARIES_URL=$URL" ]]; then
    pass "the right pin: green, GITHUB_ENV holds exactly SKIA_BINARIES_URL=file://<the fetched archive>"
else
    fail "the right pin was not handed to the build" "$(cat "$SANDBOX/out" "$SANDBOX/runner/env")"
fi

echo "Skia, checked (the pinned archive fetched above):"
took=("TRYING TO DOWNLOAD AND INSTALL SKIA BINARIES: 0.9.1/$KEY" "  FROM: $URL" "DOWNLOAD AND INSTALL SUCCEEDED")
output debug "${took[@]}"
if step checked; then
    pass "the dev profile's output took the pinned archive: green"
else
    fail "a build that took the pinned archive: red" "$(cat "$SANDBOX/out")"
fi
output debug "TRYING TO DOWNLOAD AND INSTALL SKIA BINARIES: 0.9.1/$KEY" \
    "  FROM: https://github.com/rust-skia/skia-binaries/releases/download/0.9.1/$ARCHIVE" \
    "DOWNLOAD AND INSTALL FAILED: curl exited 22" "STARTING A FULL BUILD" "HOST: x86_64-unknown-linux-gnu"
if step checked; then
    fail "the fallback to a source build: the step is green" "$(cat "$SANDBOX/out")"
else
    pass "the fallback to a source build: red"
fi
# The control for the profile: `cargo check` writes debug, never release,
# so an output only under release is no build of this job's.
output release "${took[@]}"
if step checked; then
    fail "an output under release only: green, so the step does not read the dev profile" "$(cat "$SANDBOX/out")"
else
    pass "an output under the release profile only: red (the step reads debug)"
fi

echo
if (( failures )); then
    echo "test_android_check_skia.sh: $failures assertion(s) FAILED" >&2
    exit 1
fi
echo "test_android_check_skia.sh: all assertions passed"
