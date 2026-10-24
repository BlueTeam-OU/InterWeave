---
role: "devex-tooling"
class: solution
topic: "hosted-runner-has-user-manager"
description: "CI needs systemctl --user or journalctl --user on a GitHub-hosted ubuntu runner? it lingers at boot; only the session vars are missing"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9983dca578a8ad55
---

## CI needs systemctl --user or journalctl --user on a GitHub-hosted ubuntu runner? it lingers at boot; only the session vars are missing

Measured 2026-10-20, probe run 39478964879 (ubuntu-24.04 image, systemd
255): the `runner` account (uid 1001) is ALREADY lingering at boot —
`Linger=yes`, `user@1001.service` active before the first step. A step's
environment lacks `XDG_RUNTIME_DIR` and `DBUS_SESSION_BUS_ADDRESS`; with
`/run/user/<uid>` and `unix:path=/run/user/<uid>/bus` set,
`systemctl --user show-environment` exits 0 and `journalctl --user -u
<unit>` returns a user unit's lines (no Storage= override needed).
InterWeave's user_units.rs cases passed there in 21.7 s. The wiring
(j180) exports only the uid as a step output and sets the two variables
on the test step alone; `sudo loginctl enable-linger` kept as a no-op
guard. A push-triggered workflow on a throwaway branch is how to probe a
runner without touching ci.yml (`on: push: branches: [<that branch>]`).
Landed on main with #324 (merge 08b4d9e6, 2026-10-21): in the required
`rust` job, run 39653619704, the two ignored cases took 18.0 s + 3.0 s
(2 run, 2 passed). Read the step log for "2 tests run" before calling a
green step delivered; --no-tests=fail guards the zero case.
Related: [[ignored-row-ci-nextest-exact]].

*References: ignored-row-ci-nextest-exact*

*Anchors: `.github/workflows/ci.yml::steps.user-manager.outputs.uid`*

*Observed 2026-10-20 (devex-tooling)*
