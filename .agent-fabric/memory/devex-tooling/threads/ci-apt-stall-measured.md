---
role: "devex-tooling"
class: threads
topic: "ci-apt-stall-measured"
description: "CI apt-get install stalls on azure.archive.ubuntu.com (20 kB/s): gzapp driver-flutter 22.9 min, InterWeave 3.4 min; j246/j247 add a .deb cache + Acquire retries"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 949625ac12fa1168
---

## CI apt-get install stalls on azure.archive.ubuntu.com (20 kB/s): gzapp driver-flutter 22.9 min, InterWeave 3.4 min; j246/j247 add a .deb cache + Acquire retries

Measured 2026-10-26, from the owner's question about a 26-minute gzapp merge.

- gzapi-org/gzapp #1184 sat 29.5 min in the merge queue (normal 8.5-10.3
  min). Run 40370163870: every job done by 9.6 min except driver-flutter
  (28.5 min). Its step "Linux desktop toolchain for the integration suite"
  (apt-get install clang cmake ninja-build pkg-config libgtk-3-dev
  libsecret-1-dev xvfb) took 22.9 min: azure.archive.ubuntu.com served
  26.8 MB at 20.1 kB/s, many `Ign:` lines. Median of that step over 58
  runs: 0.9 min; slow twice that day (04:59 14.2 min, 17:40 22.9 min).
- BlueTeam-OU/InterWeave's display apt step: median 0.1 min, 3.4 min at
  17:20 the same day. InterWeave queue normally 12.7-15.7 min.
- Both queues: MERGE, build 5, min/max 1-5, 2-min wait, 60-min timeout,
  ALLGREEN. Queue itself was healthy; the mirror was the cause.

The owner asked (2026-10-26) for both fixes in both repositories: cache
the downloaded packages keyed on the package list, and
`Acquire::Retries` + `Acquire::http::Timeout` so a stalled mirror costs
a minute. Jobs: j246 (InterWeave), j247 (gzapp). Related:
[[rust-job-cache-and-nextest-measured]].

j246 landed as BlueTeam-OU/InterWeave #464 (tools/ci/apt_install.sh, list
tools/ci/apt-display.txt). Measured on the hosted runner 2026-10-26: the
image already holds most display packages, so the cache is 7 .debs /
1.09 MB (5 new, 2 upgraded); a custom Dir::Cache::Archives under
$RUNNER_TEMP prints "W: Download is performed unsandboxed as root" (_apt
cannot reach /home/runner) and works. Rootless podman ubuntu:24.04:
cold 105 .debs 65 s, warm "Need to get 0 B". release.yml's linux job runs
apt before checkout, so it writes `apt_install.sh conf`'s lines itself and
the self-test compares them. For gzapp (j247) the same script shape fits.

Open follow-up (2026-10-26): #464 was armed at 08ad5218 on
architect-cto-01's AUTHORIZES (01a17a1f). After it merges, read the first
push-to-main ci.yml run's rust job: "Save the display packages' .debs
(main only)" must run and succeed, and a later PR run must log "7 .deb(s)
... before the install" (a hit). gzapp's j247 lives in
~/projects/gzapp-j247-gzapp-ci-apt-installs-cache-the;
port tools/ci/apt_install.sh + its test (conf subcommand included).

j247 (gzapp) committed 2026-10-26 as 04491d2cb on the gzapp devex batch branch
develop-qzapp/devex-tooling/feat/j238-gzapp-make-up-under-podman-compose (pushed,
no PR). gzapp has no push-to-main Flutter run, so its save gate is
github.ref == refs/heads/main = cache-warm.yml (weekly, Mon 04:17). The #464
main-run check was still owed then: #464 armed, not yet merged.

CLOSED 2026-10-27: #464 merged 3affdebc (2026-10-26 22:00). Main run 40409019539
saved apt-ubuntu-26.04-x86_64-12e885648b5f2442-2026-W44 (1.09 MB, refs/heads/main);
main run 40415352716 restored it exactly ("7 .deb(s) ... before", "Need to get
0 B/1091 kB") and skipped the save. Note the image is ubuntu 26.04 now.

*References: rust-job-cache-and-nextest-measured*

*Observed 2026-10-26 (devex-tooling)*
