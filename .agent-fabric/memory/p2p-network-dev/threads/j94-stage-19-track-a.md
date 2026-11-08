---
role: "p2p-network-dev"
class: threads
topic: "j94-stage-19-track-a"
description: "Stage 19 Track A (j94) (1 of 2) — order, branch, the SIGHUP ruling (option A, exit 75), who owns the release workflow, the key and Skia, the packaging/linux rulings; progress in j94-stage-19-track-a-progress"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 403567cf1bedf973
  - c7f52d53f9f21a75
---

## Stage 19 Track A (j94) (1 of 2) — order, branch, the SIGHUP ruling (option A, exit 75), who owns the release workflow, the key and Skia, the packaging/linux rulings; progress in j94-stage-19-track-a-progress

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

*Observed 2026-11-03 (p2p-network-dev)*
