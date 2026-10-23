#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/test_check_rustc_pin.sh
#
# Self-test for check_rustc_pin.sh.
#
# The check is copied into a sandbox repository whose rust-toolchain.toml
# each case writes, and `rustc` is a stub on PATH printing what each case
# sets — so no case depends on the compiler this host or runner has.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/check_rustc_pin.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

# The compiler is chosen by these too, and a caller's value would replace
# the stub every case below relies on; the cases that need one set it.
unset RUSTC CARGO_BUILD_RUSTC CARGO
export CARGO_HOME="/nonexistent-cargo-home"

failures=0
# Physical: the check works from the physical root (cargo's getcwd), so
# paths it prints and the cwd its stubs record are spelled that way; a
# TMPDIR reached through a symlink must not make them differ from these.
SANDBOX="$(mktemp -d)" && SANDBOX="$(cd "$SANDBOX" && pwd -P)"
[ -n "$SANDBOX" ] && [ -d "$SANDBOX" ] || { echo "test_check_rustc_pin: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
# The check walks from the repository up to / for cargo configs, so one
# above the sandbox that sets a rustc key would decide every case; refuse,
# by name, instead. Only such a one: any other config changes no case.
# The pattern is the check's own line.
eval "$(grep -m1 '^rustc_key_re=' "$UNDER_TEST")"
[[ -n "${rustc_key_re:-}" ]] || { echo "test_check_rustc_pin: no rustc_key_re= line in $UNDER_TEST" >&2; exit 1; }
d="$SANDBOX"
while :; do
    for f in "$d/.cargo/config.toml" "$d/.cargo/config"; do
        if [[ -f "$f" ]] && grep -Eq "$rustc_key_re" <<<"$(grep -Ev '^[[:space:]]*#' "$f")"; then
            echo "test_check_rustc_pin: cannot run under $f (above the sandbox, and it sets a rustc key); set TMPDIR elsewhere" >&2; exit 1
        fi
    done
    [[ "$d" == / ]] && break
    d="$(dirname "$d")"
done
REPO="$SANDBOX/repo"
mkdir -p "$REPO/tools/checks" "$SANDBOX/bin"
cp "$UNDER_TEST" "$REPO/tools/checks/"
# A git work tree, as the repository is: the check lists its tracked files.
git -C "$REPO" init -q

pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# The stub prints $SANDBOX/rustc-says, or fails when $SANDBOX/rustc-fails
# exists; it records the directory it ran in.
cat > "$SANDBOX/bin/rustc" <<EOF
#!/usr/bin/env bash
[[ "\$1" == --print ]] && { echo "$SANDBOX/sysroot"; exit 0; }
pwd > "$SANDBOX/rustc-cwd"
[[ -e "$SANDBOX/rustc-fails" ]] && { echo "error: toolchain '1.98.1' is not installed" >&2; exit 1; }
cat "$SANDBOX/rustc-says"
EOF
chmod +x "$SANDBOX/bin/rustc"

# cargo: `cargo clippy --version` and `cargo fmt --version` say what cargo
# runs. $SANDBOX/cargo-<sub>-says if it exists (an empty file prints
# nothing), else a build naming rustc-says's own "(hash date)", as
# rustup's do, after $SANDBOX/cargo-noise's lines if it exists; fails on
# $SANDBOX/cargo-fails, or for one subcommand on $SANDBOX/cargo-<sub>-fails.
# It records the directory it ran in.
cat > "$SANDBOX/bin/cargo" <<STUB
#!/usr/bin/env bash
pwd > "$SANDBOX/cargo-cwd"
[[ "\$2" == --version ]] || { echo "cargo stub: \$*" >&2; exit 2; }
[[ -e "$SANDBOX/cargo-fails" || -e "$SANDBOX/cargo-\$1-fails" ]] && { echo "error: no such command: \\\`\$1\\\`" >&2; exit 101; }
echo "warning: a cargo config warning, on stderr" >&2
[[ -e "$SANDBOX/cargo-noise" ]] && cat "$SANDBOX/cargo-noise"
[[ -e "$SANDBOX/cargo-\$1-says" ]] && exec cat "$SANDBOX/cargo-\$1-says"
build="\$(sed -n 's/^rustc [^ ]* \(([0-9a-f]* [0-9-]*)\).*/\1/p' "$SANDBOX/rustc-says")"
case "\$1" in clippy) echo "clippy 0.1.98 \$build" ;; fmt) echo "rustfmt 1.9.0-stable \$build" ;; esac
STUB
# clippy-driver on PATH, the distribution's: `--rustc --version` prints
# the compiler it is built on, $SANDBOX/clippy-says if set, else the
# pinned 1.98.1; fails on $SANDBOX/clippy-fails.
cat > "$SANDBOX/bin/clippy-driver" <<STUB
#!/usr/bin/env bash
[[ -e "$SANDBOX/clippy-fails" ]] && { echo "error: clippy-driver: component not installed" >&2; exit 1; }
[[ -s "$SANDBOX/clippy-says" ]] && exec cat "$SANDBOX/clippy-says"
echo "rustc 1.98.1 (48a229cea 2026-09-01)"
STUB
mkdir -p "$SANDBOX/sysroot/lib"
printf '#!/usr/bin/env bash\necho "rustfmt 1.9.0"\n' > "$SANDBOX/bin/rustfmt"
touch "$SANDBOX/sysroot/lib/librustc_driver-aaaa1111.so"
# ldd: records which binary it was asked about, and reports the driver in
# $SANDBOX/fmt-links (default: the sysroot's), or none on an empty file.
cat > "$SANDBOX/bin/ldd" <<STUB
#!/usr/bin/env bash
echo "\$1" > "$SANDBOX/ldd-asked"
echo "	linux-vdso.so.1 (0x00007ffd)"
lib="librustc_driver-aaaa1111.so"
[[ -e "$SANDBOX/fmt-links" ]] && lib="\$(cat "$SANDBOX/fmt-links")"
[[ -n "\$lib" ]] && echo "	\$lib => /somewhere/\$lib (0x0000785d3e800000)"
echo "	libc.so.6 => /lib64/libc.so.6 (0x00007f)"
STUB
# cargo-clippy: the distribution's, which runs the clippy-driver beside it;
# never run here, only found. CARGO_HOME is /nonexistent-cargo-home, so the
# check finds it on PATH.
printf '#!/usr/bin/env bash\nexit 0\n' > "$SANDBOX/bin/cargo-clippy"
chmod +x "$SANDBOX/bin/cargo" "$SANDBOX/bin/cargo-clippy" "$SANDBOX/bin/clippy-driver" "$SANDBOX/bin/ldd" "$SANDBOX/bin/rustfmt"
# A distribution's builds name no commit.
distro() { printf 'clippy 0.1.%s\n' "${1:-98}" > "$SANDBOX/cargo-clippy-says"; printf 'rustfmt 1.9.0\n' > "$SANDBOX/cargo-fmt-says"; }
rustup_builds() { rm -f "$SANDBOX/cargo-clippy-says" "$SANDBOX/cargo-fmt-says"; }

# pin <channel line>: the pin file, and a Cargo.toml whose rust-version
# states the same version, as the repository's does; manifest <version>
# rewrites the manifest alone.
manifest() { printf '[workspace]\nmembers = []\n\n[workspace.package]\nedition = "2024"\nrust-version = "%s"\n\n[workspace.dependencies]\nrust-version = "not this"\n' "$1" > "$REPO/Cargo.toml"; }
pin()   { printf '[toolchain]\n%s\ncomponents = ["rustfmt", "clippy"]\n' "$1" > "$REPO/rust-toolchain.toml"
          manifest "$(sed -n 's/^channel = "\(.*\)"$/\1/p' <<<"$1")"; }
says()  { printf '%s\n' "$1" > "$SANDBOX/rustc-says"; }

# expect <exit> <name> [<output substring>] — run from elsewhere, so the
# check must find the repository root itself.
expect() {
    local want="$1" name="$2" needle="${3:-}" out got
    out="$(cd "$SANDBOX" && PATH="$SANDBOX/bin:$PATH" bash "$REPO/tools/checks/check_rustc_pin.sh" 2>&1)"; got=$?
    if [[ "$got" -ne "$want" ]]; then fail "$name — wanted exit $want, got $got" "$out"; return; fi
    if [[ -n "$needle" && "$out" != *"$needle"* ]]; then fail "$name — output lacks: $needle" "$out"; return; fi
    pass "$name (exit $got)"
}
# expect_with <exit> <name> <output substring> <VAR=value>…: expect, with
# the environment changed; a PATH= given replaces the default.
expect_with() {
    local want="$1" name="$2" needle="$3" out got; shift 3
    out="$(cd "$SANDBOX" && env PATH="$SANDBOX/bin:$PATH" "$@" bash "$REPO/tools/checks/check_rustc_pin.sh" 2>&1)"; got=$?
    if [[ "$got" -ne "$want" ]]; then fail "$name — wanted exit $want, got $got" "$out"; return; fi
    if [[ -n "$needle" && "$out" != *"$needle"* ]]; then fail "$name — output lacks: $needle" "$out"; return; fi
    pass "$name (exit $got)"
}

echo "test_check_rustc_pin"

pin 'channel = "1.98.1"'
says 'rustc 1.98.1 (48a229cea 2026-09-01) (Fedora 1.98.1-1.fc43)'
expect 0 "a distribution's rustc at the pinned version passes" "rustc 1.98.1 is the pinned 1.98.1"
[[ "$(cat "$SANDBOX/rustc-cwd" 2>/dev/null)" == "$REPO" ]] && pass "  and rustc was asked from the repository root, where rustup reads the pin" \
    || fail "rustc ran in $(cat "$SANDBOX/rustc-cwd" 2>/dev/null), not the repository root"
says 'rustc 1.98.1 (48a229cea 2026-09-01)'
expect 0 "rustup's rustc at the pinned version passes"

says 'rustc 1.99.0 (aaaaaaaaa 2026-10-01) (Fedora 1.99.0-1.fc43)'
expect 1 "a newer compiler fails, naming both versions" "the compiler is rustc 1.99.0; rust-toolchain.toml pins 1.98.1"
says 'rustc 1.98.0 (aaaaaaaaa 2026-08-01)'
expect 1 "an older patch release fails" "rustc 1.98.0"
says 'rustc 1.98.10 (aaaaaaaaa 2026-12-01)'
expect 1 "a version the pin is a prefix of fails" "rustc 1.98.10"
says 'rustc 1.98.1-beta.2 (aaaaaaaaa 2026-08-20)'
expect 1 "a pre-release of the pinned version fails" "rustc 1.98.1-beta.2"

pin 'channel = "1.98"'
says 'rustc 1.98.3 (aaaaaaaaa 2026-09-20)'
expect 0 "a minor pin admits any patch release of it"
says 'rustc 1.98.1-beta.2 (aaaaaaaaa 2026-08-20)'
expect 1 "a minor pin refuses a pre-release of it" "rustc 1.98.1-beta.2"
says 'rustc 1x98.3 (aaaaaaaaa 2026-09-20)'
expect 1 "a minor pin's dots are literal (1x98.3 is not 1.98)"
says 'rustc 1.99.0 (aaaaaaaaa 2026-10-01)'
expect 1 "a minor pin refuses the next minor"
says 'rustc 1.980.0 (aaaaaaaaa 2036-10-01)'
expect 1 "a minor pin is not a string prefix (1.98 against 1.980.0)"

pin 'channel = "1.98.1"'
says 'rustc 1.99.0 (aaaaaaaaa 2026-10-01)'
printf '#!/usr/bin/env bash\n[[ "$1" == --print ]] && { echo "%s/sysroot"; exit 0; }\necho "rustc 1.98.1 (48a229cea 2026-09-01)"\n' "$SANDBOX" > "$SANDBOX/bin/rustc-pinned"; chmod +x "$SANDBOX/bin/rustc-pinned"
# cargo's tools are the pinned compiler's here; only rustc differs.
printf 'clippy 0.1.98 (48a229ceae 2026-09-01)\n' > "$SANDBOX/cargo-clippy-says"
printf 'rustfmt 1.9.0-stable (48a229ceae 2026-09-01)\n' > "$SANDBOX/cargo-fmt-says"
out="$(cd "$SANDBOX" && PATH="$SANDBOX/bin:$PATH" RUSTC="$SANDBOX/bin/rustc-pinned" bash "$REPO/tools/checks/check_rustc_pin.sh" 2>&1)"; got=$?
[[ "$got" -eq 0 ]] && pass "\$RUSTC, the compiler cargo would use, is the one asked (exit 0)" \
    || fail "\$RUSTC should be asked instead of PATH's rustc, got $got" "$out"
out="$(cd "$SANDBOX" && PATH="$SANDBOX/bin:$PATH" CARGO_BUILD_RUSTC="$SANDBOX/bin/rustc-pinned" bash "$REPO/tools/checks/check_rustc_pin.sh" 2>&1)"; got=$?
[[ "$got" -eq 0 ]] && pass "\$CARGO_BUILD_RUSTC is asked when \$RUSTC is unset (exit 0)" \
    || fail "\$CARGO_BUILD_RUSTC should be asked instead of PATH's rustc, got $got" "$out"
out="$(cd "$SANDBOX" && PATH="$SANDBOX/bin:$PATH" RUSTC="$SANDBOX/bin/rustc" CARGO_BUILD_RUSTC="$SANDBOX/bin/rustc-pinned" bash "$REPO/tools/checks/check_rustc_pin.sh" 2>&1)"; got=$?
[[ "$got" -eq 1 ]] && pass "  and \$RUSTC wins over it, as in cargo (exit 1)" \
    || fail "\$RUSTC should win over \$CARGO_BUILD_RUSTC, got $got" "$out"
rustup_builds

# The stated minimum: rust-version must equal the channel as written, and
# only [workspace.package]'s counts.
pin 'channel = "1.98.1"'
says 'rustc 1.98.1 (48a229cea 2026-09-01)'
manifest "1.97.0"
expect 1 "a rust-version below the pin fails, naming both" "Cargo.toml's rust-version is 1.97.0; rust-toolchain.toml pins 1.98.1"
manifest "1.99.0"
expect 1 "a rust-version above the pin fails" "rust-version is 1.99.0"
manifest "1.98"
expect 1 "a rust-version naming only the pin's minor fails" "rust-version is 1.98;"
printf '[workspace]\nmembers = []\n\n[package]\nrust-version = "1.98.1"\n' > "$REPO/Cargo.toml"
expect 2 "a rust-version outside [workspace.package] does not count" "has no rust-version"
rm -f "$REPO/Cargo.toml"
expect 2 "no Cargo.toml is a failure to check" "no Cargo.toml"
pin 'channel = "1.98.1"'
expect 0 "rust-version equal to the pin passes" "and rust-version states it"

# clippy and rustfmt: asked through cargo, which is what runs them.
pin 'channel = "1.98.1"'
says 'rustc 1.98.1 (48a229cea 2026-09-01)'
rm -f "$SANDBOX/ldd-asked"
expect 0 "a clippy and rustfmt naming rustc's build pass" "clippy and rustfmt are built on it"
[[ "$(cat "$SANDBOX/cargo-cwd" 2>/dev/null)" == "$REPO" ]] && pass "  and cargo was asked from the repository root" \
    || fail "cargo ran in $(cat "$SANDBOX/cargo-cwd" 2>/dev/null), not the repository root"
[[ ! -e "$SANDBOX/ldd-asked" ]] && pass "  and no binary on PATH was asked about, since cargo named the build" \
    || fail "a build cargo named should decide alone; ldd was asked about $(cat "$SANDBOX/ldd-asked")"
# The case the check was wrong about: rustup's tool is the pin's while
# cargo runs another (a CARGO_HOME without rustup's proxies). Only what
# cargo says can tell; a tool on PATH or rustup's cannot.
printf 'rustc 1.99.0 (b940084d7 2026-09-28) (Fedora 1.99.0-1.fc43)\n' > "$SANDBOX/clippy-says"
distro 99
expect 1 "cargo running a distribution clippy of another minor fails, naming it" "cargo clippy is clippy 0.1.99; rust-toolchain.toml pins 1.98.1"
rustup_builds
expect 0 "  while a PATH clippy-driver off the pin does not decide when cargo names the build"
rm -f "$SANDBOX/clippy-says"
printf 'clippy 0.1.98 (bbbbbbbbbb 2026-09-01)\n' > "$SANDBOX/cargo-clippy-says"
expect 1 "a clippy naming another commit fails, naming both" "cargo clippy is clippy 0.1.98 (bbbbbbbbbb 2026-09-01), not rustc 1.98.1's build (48a229cea 2026-09-01)"
printf 'clippy 0.1.98 (48a229ceae 2026-09-02)\n' > "$SANDBOX/cargo-clippy-says"
expect 1 "a clippy naming rustc's commit with another date fails" "not rustc 1.98.1's build"
printf 'clippy 0.1.98 (48a229ceae 2026-09-01)\n' > "$SANDBOX/cargo-clippy-says"
expect 0 "a longer hash of rustc's commit is the same build"
printf 'rustfmt 1.9.0-stable (bbbbbbbbbb 2026-09-01)\n' > "$SANDBOX/cargo-fmt-says"
expect 1 "a rustfmt naming another build fails, naming it" "cargo fmt is rustfmt 1.9.0-stable (bbbbbbbbbb 2026-09-01)"
printf 'clippy 0.1.98 (48a229ceae 2026-09-01)\n' > "$SANDBOX/cargo-clippy-says"
says 'rustc 1.98.1'
expect 1 "a build named against a rustc that names none fails" "which names none"
rustup_builds
says 'rustc 1.98.1 (48a229cea 2026-09-01)'
touch "$SANDBOX/cargo-fails"
expect 2 "a cargo clippy that cannot run is a failure to check, with its error" "no such command"
out="$(cd "$SANDBOX" && PATH="$SANDBOX/bin:$PATH" bash "$REPO/tools/checks/check_rustc_pin.sh" 2>&1)"
[[ "$out" != *"printed no"* ]] && pass "  and it ends there, not in a parse of the empty answer" \
    || fail "a failed cargo clippy should end the check, not reach a later step" "$out"
rm -f "$SANDBOX/cargo-fails"
# Each subcommand's failure ends the check: an exit inside a command
# substitution does not, so a failing cargo fmt once fell through to PATH.
touch "$SANDBOX/cargo-fmt-fails"
expect 2 "a cargo fmt that cannot run is a failure to check, though clippy is fine" "cargo fmt --version\` failed"
distro
expect 2 "  and so for a distribution's, where PATH's rustfmt would have passed" "cargo fmt --version\` failed"
rm -f "$SANDBOX/cargo-fmt-fails"
rustup_builds
: > "$SANDBOX/cargo-clippy-says"
expect 2 "a cargo clippy printing nothing is a failure to check" "printed no clippy version"
printf 'clippy-driver 0.1.98\n' > "$SANDBOX/cargo-clippy-says"
expect 2 "a cargo clippy printing no version is a failure to check" "printed no clippy version"
rustup_builds
printf 'info: syncing channel updates for 1.98.1\nwarning: something on stdout\n' > "$SANDBOX/cargo-noise"
expect 0 "lines before the version, and stderr, are not read as the version"
rm -f "$SANDBOX/cargo-noise"
printf '#!/usr/bin/env bash\necho "clippy 0.1.99 (bbbbbbbbbb 2026-09-28)"\n' > "$SANDBOX/cargo-at"; chmod +x "$SANDBOX/cargo-at"
out="$(cd "$SANDBOX" && PATH="$SANDBOX/bin:$PATH" CARGO="$SANDBOX/cargo-at" bash "$REPO/tools/checks/check_rustc_pin.sh" 2>&1)"; got=$?
[[ "$got" -eq 1 && "$out" == *"clippy 0.1.99"* ]] && pass "\$CARGO, the cargo running cargo xtask, is the one asked (exit 1)" \
    || fail "\$CARGO should be asked instead of PATH's cargo, got $got" "$out"

# A distribution's builds name no commit: clippy's minor must be the
# pin's, and the clippy-driver and rustfmt on PATH, beside it, are asked.
distro
expect 0 "a distribution clippy and rustfmt on the pinned compiler pass"
[[ "$(cat "$SANDBOX/ldd-asked" 2>/dev/null)" == "$SANDBOX/bin/rustfmt" ]] && pass "  and the rustfmt on PATH was asked about" \
    || fail "ldd should be asked about PATH's rustfmt, was: $(cat "$SANDBOX/ldd-asked" 2>/dev/null)"
printf 'rustc 1.99.0 (aaaaaaaaa 2026-10-01)\n' > "$SANDBOX/clippy-says"
expect 1 "a clippy-driver built on another compiler fails, naming its path" "$SANDBOX/bin/clippy-driver: rustc 1.99.0"
printf 'rustc 1.98.1-beta.2 (aaaaaaaaa 2026-08-20)\n' > "$SANDBOX/clippy-says"
expect 1 "a clippy-driver built on a pre-release of the pin fails" "clippy is built on rustc 1.98.1-beta.2"
printf 'clippy 0.1.98\n' > "$SANDBOX/clippy-says"
expect 2 "a clippy-driver that prints no compiler is a failure to check" "printed no version"
rm -f "$SANDBOX/clippy-says"
touch "$SANDBOX/clippy-fails"
expect 2 "a clippy-driver that cannot run is a failure to check" "clippy-driver --rustc --version\` failed"
rm -f "$SANDBOX/clippy-fails"
printf 'librustc_driver-bbbb2222.so' > "$SANDBOX/fmt-links"
expect 1 "a rustfmt linking another compiler's librustc_driver fails, naming it" "links librustc_driver-bbbb2222.so, which is not rustc 1.98.1's"
printf '' > "$SANDBOX/fmt-links"
expect 2 "a rustfmt linking no librustc_driver is a failure to check" "links no librustc_driver"
rm -f "$SANDBOX/fmt-links"
mv "$SANDBOX/sysroot/lib/librustc_driver-aaaa1111.so" "$SANDBOX/sysroot/lib/x"
expect 2 "a sysroot with no librustc_driver is a failure to check" "names no directory holding a librustc_driver"
mv "$SANDBOX/sysroot/lib/x" "$SANDBOX/sysroot/lib/librustc_driver-aaaa1111.so"
# The clippy-driver asked is the one beside the cargo-clippy cargo finds:
# PATH in order, with $CARGO_HOME/bin first only when PATH lacks it; a
# clippy-driver alone, earlier on PATH, is not it.
mkdir -p "$SANDBOX/decoy" "$SANDBOX/lone" "$SANDBOX/ch/bin" "$SANDBOX/fmt"
printf '#!/usr/bin/env bash\necho "rustc 1.99.0 (aaaaaaaaa 2026-10-01)"\n' > "$SANDBOX/decoy/clippy-driver"
cp "$SANDBOX/decoy/clippy-driver" "$SANDBOX/ch/bin/clippy-driver"
cp "$SANDBOX/bin/cargo-clippy" "$SANDBOX/ch/bin/cargo-clippy"
cp "$SANDBOX/bin/cargo-clippy" "$SANDBOX/lone/cargo-clippy"
chmod +x "$SANDBOX/decoy/clippy-driver" "$SANDBOX/ch/bin/clippy-driver"
expect_with 0 "a clippy-driver earlier on PATH than cargo-clippy's own does not decide" "" PATH="$SANDBOX/decoy:$SANDBOX/bin:$PATH"
expect_with 1 "the cargo-clippy in \$CARGO_HOME/bin wins over PATH's, and its driver is asked" "$SANDBOX/ch/bin/clippy-driver: rustc 1.99.0" CARGO_HOME="$SANDBOX/ch"
expect_with 2 "a cargo-clippy with no clippy-driver beside it is a failure to check" "no clippy-driver beside $SANDBOX/lone/cargo-clippy" PATH="$SANDBOX/lone:$SANDBOX/bin:$PATH"
# $CARGO_HOME/bin already on PATH: cargo walks PATH in order, so a
# cargo-clippy earlier on it is the one run, not $CARGO_HOME/bin's.
mkdir -p "$SANDBOX/early"
cp "$SANDBOX/ch/bin/cargo-clippy" "$SANDBOX/ch/bin/clippy-driver" "$SANDBOX/early/"
cp "$SANDBOX/bin/clippy-driver" "$SANDBOX/ch/bin/clippy-driver.pinned"
mv "$SANDBOX/ch/bin/clippy-driver" "$SANDBOX/ch/bin/clippy-driver.off"; mv "$SANDBOX/ch/bin/clippy-driver.pinned" "$SANDBOX/ch/bin/clippy-driver"
expect_with 1 "with \$CARGO_HOME/bin on PATH, an earlier cargo-clippy is the one asked about" "$SANDBOX/early/clippy-driver: rustc 1.99.0" CARGO_HOME="$SANDBOX/ch" PATH="$SANDBOX/early:$SANDBOX/ch/bin:$SANDBOX/bin:$PATH"
expect_with 0 "  while off PATH, \$CARGO_HOME/bin comes first" "" CARGO_HOME="$SANDBOX/ch" PATH="$SANDBOX/early:$SANDBOX/bin:$PATH"
for spelling in "$SANDBOX/ch/bin/" "$SANDBOX/ch//bin" "$SANDBOX/ch/./bin" "$SANDBOX/./ch/bin/."; do
    expect_with 1 "  and \$CARGO_HOME/bin spelled $(sed "s#$SANDBOX#…#" <<<"$spelling") on PATH is on it, as cargo compares" "$SANDBOX/early/clippy-driver: rustc 1.99.0" CARGO_HOME="$SANDBOX/ch" PATH="$SANDBOX/early:$spelling:$SANDBOX/bin:$PATH"
done
expect_with 1 "  and so is it when CARGO_HOME carries the trailing slash" "$SANDBOX/early/clippy-driver: rustc 1.99.0" CARGO_HOME="$SANDBOX/ch/" PATH="$SANDBOX/early:$SANDBOX/ch/bin:$SANDBOX/bin:$PATH"
# A relative CARGO_HOME is cargo's cwd (the root) joined to it: the same
# relative spelling on PATH is not that path, so cargo puts it first.
mkdir -p "$REPO/rel/bin"
cp "$SANDBOX/bin/cargo-clippy" "$SANDBOX/bin/clippy-driver" "$REPO/rel/bin/"
expect_with 0 "a relative CARGO_HOME is the root's, put first though PATH spells it relative" "" CARGO_HOME=rel PATH="$SANDBOX/early:rel/bin:$SANDBOX/bin:$PATH"
expect_with 1 "  while its absolute form on PATH is on it" "$SANDBOX/early/clippy-driver: rustc 1.99.0" CARGO_HOME=rel PATH="$SANDBOX/early:$REPO/rel/bin:$SANDBOX/bin:$PATH"
# Through a symlinked path the root is the physical one cargo sees: the
# logical spelling of rel/bin on PATH is not cargo's home.
ln -s "$REPO" "$SANDBOX/link"
out="$(cd "$SANDBOX" && CARGO_HOME=rel PATH="$SANDBOX/early:$SANDBOX/link/rel/bin:$SANDBOX/bin:$PATH" bash "$SANDBOX/link/tools/checks/check_rustc_pin.sh" 2>&1)"; got=$?
[[ "$got" -eq 0 ]] && pass "run through a symlink, a relative CARGO_HOME is joined to the physical root, as cargo's getcwd (exit 0)" \
    || fail "through a symlink the logical rel/bin should not count as cargo's home, got $got" "$out"
rm -f "$SANDBOX/link"
rm -rf "$REPO/rel"
out="$(cd "$SANDBOX" && env -u CARGO_HOME -u HOME PATH="$SANDBOX/bin:$PATH" bash "$REPO/tools/checks/check_rustc_pin.sh" 2>&1)"; got=$?
[[ "$got" -eq 2 && "$out" == *"neither CARGO_HOME nor HOME is set"* ]] && pass "neither CARGO_HOME nor HOME set is a failure to check, not a FAIL (exit 2)" \
    || fail "neither CARGO_HOME nor HOME set should be exit 2, got $got" "$out"
mv "$SANDBOX/ch/bin/clippy-driver.off" "$SANDBOX/ch/bin/clippy-driver"
# A symlinked cargo-clippy runs the driver beside its target.
mkdir -p "$SANDBOX/farm"
ln -s "$SANDBOX/early/cargo-clippy" "$SANDBOX/farm/cargo-clippy"
cp "$SANDBOX/bin/clippy-driver" "$SANDBOX/farm/clippy-driver"
expect_with 1 "a symlinked cargo-clippy is followed to the driver beside its target" "$SANDBOX/early/clippy-driver: rustc 1.99.0" PATH="$SANDBOX/farm:$SANDBOX/bin:$PATH"
# cargo-fmt runs $RUSTFMT when it is set.
cp "$SANDBOX/bin/rustfmt" "$SANDBOX/fmt/rustfmt-other"
expect_with 0 "\$RUSTFMT, the rustfmt cargo fmt runs, is the one asked" "" RUSTFMT="$SANDBOX/fmt/rustfmt-other"
[[ "$(cat "$SANDBOX/ldd-asked" 2>/dev/null)" == "$SANDBOX/fmt/rustfmt-other" ]] && pass "  and it was" \
    || fail "ldd should be asked about \$RUSTFMT, was: $(cat "$SANDBOX/ldd-asked" 2>/dev/null)"
rustup_builds

# A key spelled rustc in a cargo config in scope is a failure to check, in
# every TOML spelling, in the repository, a parent directory or under
# $CARGO_HOME; rustc-wrapper and other [build] keys are not.
says 'rustc 1.98.1 (48a229cea 2026-09-01)'
mkdir -p "$REPO/.cargo"
printf '[build]\nrustc = "/opt/rust-1.99/bin/rustc"\n' > "$REPO/.cargo/config.toml"
expect 2 "build.rustc under [build] in the repository's config is named" ".cargo/config.toml sets a key spelled rustc"
printf 'build.rustc = "/opt/rust-1.99/bin/rustc"\n' > "$REPO/.cargo/config.toml"
expect 2 "a dotted build.rustc is named too" "sets a key spelled rustc"
for form in 'build = { rustc = "/opt/r/rustc" }' '[build]\n"rustc" = "/opt/r/rustc"' '[ build ]\nrustc="/opt/r/rustc"' 'build . rustc = "/opt/r/rustc"' "build.'rustc' = '/opt/r/rustc'"; do
    printf "$form\n" > "$REPO/.cargo/config.toml"
    expect 2 "the spelling $(printf "$form" | tr '\n' ' ' | sed 's/ $//') is named" "sets a key spelled rustc"
done
printf '[build]\nrustc-wrapper = "sccache"\nrustc-workspace-wrapper = "x"\nrustflags = ["-Dwarnings"]\n' > "$REPO/.cargo/config.toml"
expect 0 "rustc-wrapper and other [build] keys are not a compiler choice"
printf '[build]\n# rustc = "/opt/other/rustc"\n  #build.rustc = "/x"\n' > "$REPO/.cargo/config.toml"
expect 0 "a whole-line comment naming rustc carries no key"
printf '[build]\n# a note\nrustc = "/opt/other/rustc"\n' > "$REPO/.cargo/config.toml"
expect 2 "a real rustc key beside a comment is still named" "sets a key spelled rustc"
printf 'build = { rustflags = ["-C#"], rustc = "/x" }\n' > "$REPO/.cargo/config.toml"
expect 2 "a # inside a string ahead of a real rustc key does not hide it" "sets a key spelled rustc"
# A key early in a large config: a grep -q at the end of a pipe exits at
# the match, and under pipefail the writer it killed would read as none.
{ printf '[build]\nrustc = "/opt/other/rustc"\n'; for ((i = 0; i < 4000; i++)); do printf 'x%04d = "padding padding padding"\n' "$i"; done; } > "$REPO/.cargo/config.toml"
expect 2 "a rustc key ahead of 130 KB of other keys is named" "sets a key spelled rustc"
rm -f "$REPO/.cargo/config.toml"
mkdir -p "$SANDBOX/.cargo"
printf '[build]\nrustc = "/opt/rust-1.99/bin/rustc"\n' > "$SANDBOX/.cargo/config.toml"
expect 2 "a parent directory's config is searched too" "$SANDBOX/.cargo/config.toml sets"
rm -rf "$SANDBOX/.cargo"

# A tracked cargo config below the root is read by cargo run from there,
# and not by this check: a failure to check. The root's own is not.
mkdir -p "$REPO/.cargo" "$REPO/crates/x/.cargo"
printf '[alias]\nxtask = "run -p xtask --"\n' > "$REPO/.cargo/config.toml"
git -C "$REPO" add .cargo/config.toml
expect 0 "the root's own tracked cargo config is in scope, not below it"
printf '[net]\nretry = 2\n' > "$REPO/crates/x/.cargo/config.toml"
git -C "$REPO" add crates/x/.cargo/config.toml
expect 2 "a tracked cargo config below the root is named" "crates/x/.cargo/config.toml is a cargo config below the repository root"
git -C "$REPO" rm -q --cached -r .cargo crates
rm -rf "$REPO/.cargo" "$REPO/crates"

# Not a git work tree: the tracked configs cannot be listed, so the check
# cannot say none is below the root -- a failure to check, not a pass.
printf '#!/usr/bin/env bash\necho "fatal: not a git repository (or any of the parent directories): .git" >&2\nexit 128\n' > "$SANDBOX/bin/git"
chmod +x "$SANDBOX/bin/git"
expect 2 "a tree git cannot list is a failure to check" "cannot list the tree's tracked files"
# A warning git prints while succeeding is not a tracked path, even one
# that ends like a cargo config below the root.
printf '#!/usr/bin/env bash\necho "warning: ignoring crates/x/.cargo/config.toml" >&2\necho rust-toolchain.toml\n' > "$SANDBOX/bin/git"
expect 0 "a warning on git's stderr is not read as a tracked path"
rm -f "$SANDBOX/bin/git"

# A config in scope that cannot be read cannot be said to set no rustc
# key. Root reads it regardless, so the case does not apply there.
mkdir -p "$REPO/.cargo"
printf '[net]\nretry = 2\n' > "$REPO/.cargo/config.toml"
chmod 000 "$REPO/.cargo/config.toml"
if [[ "$(id -u)" -eq 0 ]]; then
    echo "  - SKIP the unreadable-config case: root reads a mode-000 file, so it cannot run here"
else
    expect 2 "a config in scope that cannot be read is a failure to check" "cannot read $REPO/.cargo/config.toml"
fi
chmod 600 "$REPO/.cargo/config.toml"
rm -rf "$REPO/.cargo"
mkdir -p "$SANDBOX/cargo-home"
printf '[build]\nrustc = "/opt/rust-1.99/bin/rustc"\n' > "$SANDBOX/cargo-home/config.toml"
out="$(cd "$SANDBOX" && PATH="$SANDBOX/bin:$PATH" CARGO_HOME="$SANDBOX/cargo-home" bash "$REPO/tools/checks/check_rustc_pin.sh" 2>&1)"; got=$?
if [[ "$got" -eq 2 && "$out" == *"cargo-home/config.toml sets a key spelled rustc"* ]]; then pass "build.rustc in \$CARGO_HOME's config is named (exit 2)"
else fail "build.rustc under \$CARGO_HOME should be exit 2, got $got" "$out"; fi

touch "$SANDBOX/rustc-fails"
expect 2 "a rustc that cannot run is a failure to check, with its error" "is not installed"
rm -f "$SANDBOX/rustc-fails"
says 'something else entirely'
expect 2 "output with no version is a failure to check" "printed no version"

says 'rustc 1.98.1 (48a229cea 2026-09-01)'
pin 'channel = "stable"'
expect 2 "a floating channel names no version: a failure to check" "names no version"
pin 'channel = "nightly-2026-09-01"'
expect 2 "a nightly channel names no version either" "names no version"
pin 'profile = "minimal"'
expect 2 "no channel is a failure to check" "has no channel"
rm -f "$REPO/rust-toolchain.toml"
expect 2 "no rust-toolchain.toml is a failure to check" "no rust-toolchain.toml"

help_out="$(bash "$UNDER_TEST" --help 2>/dev/null)"
if [[ "$help_out" == *"rust-toolchain.toml pins"* ]]; then pass "--help prints the help block"; else fail "--help should print the help block" "$help_out"; fi

echo
if (( failures > 0 )); then
    echo "test_check_rustc_pin: $failures failure(s)" >&2
    exit 1
fi
echo "test_check_rustc_pin: OK — all assertions passed."
