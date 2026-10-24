---
role: "p2p-network-dev"
class: threads
topic: "j94-stage-19-track-a"
description: Stage 19 Track A (j94) — order, branch, the SIGHUP ruling (option A), who owns the release workflow, key and Skia choice
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 403567cf1bedf973
---

## Stage 19 Track A (j94) — order, branch, the SIGHUP ruling (option A), who owns the release workflow, key and Skia choice

j94 (architect-cto REQUEST 01a157dd, 2026-10-20): Stage 19 Track A, plan §22 (activation record
#317). Branch develop-qzapp/p2p-network-dev-01/feat/j94-next-stage-19-activated-packaging-linux (worktree
interweave-j94-next-stage-19-activated-packaging-linux, made by fabric-jobs switch 2026-10-20; I told
architect-cto "feat/j94-stage-19-track-a" in REPLY 01a157df -- tell them the real name in the next
message), its work cut from main after #317
lands and after #314 is armed (architect-cto is InterWeave's arming authority below 8 work).

Order, accepted: (1) Stage 13 carry — SIGHUP, then flock on NFS homes (host test or limit row);
(2) packaging/linux — read §28's interweave-daemon@/client@ units first; service/autostart
lifecycle, uninstall; fresh install is not named in my half; (3) Android release run (ADR-0057).

SIGHUP ruled 2026-10-20 (DECISION 01a157e2, option A; AMENDED by 01a157ec): transport-daemon handles
SIGHUP with SIGTERM's orderly stop (SHUTDOWN_GRACE, settle, unlink) but exits with a DISTINCT status
(mine to choose; not 0, not 2 which RestartPreventExitStatus= names), and interweave-daemon@.service
names it in RestartForceExitStatus= -- because the shipped unit is Restart=on-failure and a clean stop
exits 0, so "exactly as SIGTERM" would leave it stopped (the #317 review caught it). Met when one test
shows SIGHUP's stop sequence + that status and the unit row shows the restart on it (§28 measured a
restart only after SIGKILL). Added by 01a157f3: the status ALSO goes in SuccessExitStatus= (else
systemd logs a failure); restart follows RestartSec= (5 s), not an immediate restart, and the record
says so; the status joins plan §16 (11)'s exit-code list (0 ok, 1 refused, 2 usage, 3 unreachable
taken) -- update §16 (11), main.rs's doc comment and the unit's comment in the same change, on
architect-cto's instruction; the met test READS the number from the unit's RestartForceExitStatus=,
never a constant. The unit restarts it; fresh epochs, NO lease
survives, by design; no ExecReload; README says a profile change is
`systemctl --user restart interweave-daemon@<profile>`. My row proves: SIGHUP under the unit ends
through the grace path and the restarted daemon issues fresh epochs (model: the existing
restart-epoch case). architect-cto puts the ruling into §22 as a second commit on #317. A narrow
reload (option B) is a later ADR only on a shown client need. rust-ui-dev owes nothing; the §22
lifecycle row proves the restart from the client side with the existing client.

Release run: the tagged workflow is devex-tooling's (.github/), wired from my steps; the signing key
is the owner's; the Skia choice (source build vs prebuilt pinned by checksum) goes to architect-cto
as a QUESTION with costs when I reach it.

Seam with rust-ui-dev-02 (j30, QUESTION 01a157f7, my REPLY 01a157f9): INTERWEAVE_PACKAGE_BIN names one
dir with transport-daemon, transportctl, human-desktop; each must run bare (--profile + XDG vars), a
unit only wraps it. Linux artefact shape PROPOSED, not agreed until packaging/linux lands:
interweave-{version}-x86_64-unknown-linux-gnu.tar.gz (three executables, two user units, packaging
README, licence notices), SHA-256 only, no desktop signature (owner's call if wanted); placeholders
{linux_archive}, {linux_archive_url}, {linux_sha256}. One release page per tag for all platforms
(apps/human-android/release/release-page.md.in). My REPLY wrongly called packaging/linux "the third
step" (it is the second) -- correct it in the next message to them.

packaging/linux ruled 2026-10-20 (DECISION 01a1582d on my QUESTION 01a1582c): (1) uninstall stops+disables every
daemon@/client@ instance, removes the two units and three binaries, KEEPS every profile's config/key/state/human
store/logs, no purge; test: no unit loaded, no daemon, no socket, reinstall serves the same PeerId; (2) ship
packaging/linux/install.sh + uninstall.sh (POSIX sh, ~/.local/bin + ~/.config/systemd/user), tests drive them;
(3) install never enables; human-desktop XDG autostart is rust-ui-dev's; (4) unit rows #[ignore = "needs a systemd
user manager"] in tests/desktop-e2e, runtime-scoped link + drop-in, host run with log committed, `limit` "no user
manager in CI" until devex-tooling's job exists -- REQUEST to devex-tooling naming path::fn. NFS flock: limit row +
README "never run one profile on two hosts at once". Exit 75 accepted. SIGHUP code: daemon.rs Stop struct + hangup arm.

STATE 2026-10-20 ~08:50Z, paused for j95 (branch pushed at 48296fd7, no PR yet, 5 work commits, min 8):
bf0c6017 SIGHUP carry (daemon Stop struct + hangup arm, exit 75, unit RestartForceExitStatus=/SuccessExitStatus=75,
e2e daemon.rs::sighup_stops_as_sigterm_does_and_exits_with_the_units_restart_status reads 75 from the unit, §16 (11));
0d45a65b install.sh/uninstall.sh + tests/desktop-e2e/tests/user_units.rs (2 #[ignore] cases, runtime link + drop-in,
journal oracle); 2 uninstall-case fixes; evidence packaging/linux/evidence/2026-10-20-user-units.md (run at d7f05382).
Mutations M1-M7 all CAUGHT (scratch mutate.py is gone with the session; redo by hand if needed). Clippy clean on
daemon + desktop-e2e with -A double_must_use -A assert_is_empty. `cargo xtask checks`: only the toolchain-pin check
failed (host rustc 1.99 vs 1.98.1, environmental). devex-tooling took the CI job as j180 (REPLY 01a15862): probes
the runner's user journal first; I owe them the PR number when opened.
STILL TO DO in j94: (a) the §22 record's rows/limits are architect-cto's -- send them the row text when the PR opens:
service/autostart + uninstall met by user_units.rs host run + limit "no user manager in CI"; SIGHUP carry met;
NFS flock limit row (mechanism + README rule, already in README); (b) item 3, Android release run (ADR-0057): the
release steps script (tools/, mine) building the signed APK via cargo-ndk + Gradle, SHA-256, filling
apps/human-android/release/release-page.md.in; the tagged workflow is devex's, the key the owner's; FIRST send
architect-cto the Skia QUESTION: skia-bindings 0.153.3 downloads
github.com/rust-skia/skia-binaries/releases/download/{tag}/skia-binaries-{key}.tar.gz; SKIA_BINARIES_URL accepts
file:// so "pinned prebuilt" = fetch, verify a committed SHA-256, point SKIA_BINARIES_URL at the local file
(cost: one checksum per bindings bump; trust = rust-skia's CI); "source" = FORCE_SKIA_BUILD=1 (python3, ninja,
clang, gn, the skia+deps sources at Cargo.toml metadata commits; build time unmeasured -- this host cannot build
Android: no rustup/cargo-ndk/android std). (c) open the PR (load pull-requests skill), review class, arm at >=8.

STATE 2026-10-20 ~18:50Z (session 2): Skia ruled A (DECISION 01a15a1e-d458, ADR-0057 A 2026-10-20, architect's for/
branch folded at 0185af17). Added on the branch (pushed c5d77f81), all in tools/ci/ so CI's and xtask's self-test
loops find them: release_page.py (merges values files), release_android_apk.sh (aapt2 checks, zipalign -P 16, apksigner
v2+v3 no v1; verify needs --min-sdk-version 24 or v2 reads false at minSdk 30), skia-binaries.pin + skia_prebuilt.sh
fetch|check (key confirmed by running the real build script: cargo check --keep-going --target aarch64-linux-android
-p interweave-human-android runs build scripts without the android std), release_notices.py + release_linux_archive.sh
(55 crates publish no licence file -> listed by expression+source, a limit), skia-notices.pin + skia_notices.py (11 texts,
FreeType credit year 2026 = 2.14.2), packaging/android/README.md (the 9 steps). /opt/android-sdk has build-tools 36.
Told architect-cto the pin path (REPLY 01a15a30); devex-tooling the interface (INFO 01a15a30-b8c5) and REQUESTED the
tagged workflow (01a15a3a-8e3b, reply expected). Work commits of mine: 12. NEXT: real Linux archive run -> evidence
file + packaging/linux/README archive section; PR after #317 merges (body: rows/limits for architect-cto); review; arm.

j97 (blocking, #318 marker P3s, OBSERVATION 01a15a3c): 1319de38 pushed to for/rust-ui-dev-02/profile-format-marker (comment-only: F1 fixed, F2 test declined -- embedded LOCK_WAIT is zero so the window is two statements); alerts 82-89 dismissed; REPLY 01a15a41 sent. cargo check of the embedded tests passed at 1319de38; j97 delivered.

STATE 2026-10-20 ~19:20Z (session 3): PR #324 open (16 work + review fixes). Round 1 on b3c3f587: review A (F1 P2
uninstall unreachable manager, F2-F4 P3), review B (F1 notices from unified metadata -> cargo tree, F2 --first-release
with cert, F3 umask, F4 page version binding, F5 cpu_features provenance, F6 stale ADR prose -> architect-cto
OBSERVATION 01a15aba), Codex (C1 FROM hash, C2 profile/mtime, C3=A F1, C4 umask, C5 artifacts on failure); all judged
real; fixes b3c3f587..f42d88d1 with mutations caught. Lessons: a planted failure that trips an earlier check is not the
failure it names (use a PATH stub for the last mv); two cleanups covering each other make each mutation survive -- drop
one; systemctl show loads a template instance on demand (a "never-loaded" control needs the template gone); test
scripts get the person's four XDG roots or a profile-removal mutation misses. NEXT: re-reviews round 2 (dispatched),
xtask ci on f42d88d1 (detached, ci.log in scratchpad), architect's ADR text commit, post replies per finding, arm.

STATE 2026-10-20 ~20:20Z: #324 head 5fd2ed22 (21 work). Folded: architect 873175a0 (ADR wording, 474430be),
devex 4f1ca850 (user_units in CI rust job, d471a6fa), devex release.yml 0c9e8a2c (j185). Round-2 re-reviews posted
on f42d88d1; A F1 foreign XDG_RUNTIME_DIR fixed 587b7bca/130abe94; F3 epoch/lock declined with the judge's reasons
(answers comment on #324). BLOCKED for green CI: guards check release_notices.py (fabric one-hop rule, fabric-ref
move, devex tells me), RUSTSEC-2026-0382 rustls-webpki (devex j187 lockfile on main -> fold main). Then: xtask ci,
final claude review + codex (fabric-review --engine codex 324 rev-all.yaml --gateway chatgpt) of the final head, arm.

*Observed 2026-10-20 (p2p-network-dev)*
