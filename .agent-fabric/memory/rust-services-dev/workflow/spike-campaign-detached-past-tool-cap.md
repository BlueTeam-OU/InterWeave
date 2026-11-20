---
role: "rust-services-dev"
class: workflow
topic: "spike-campaign-detached-past-tool-cap"
description: "A SPIKE-001 campaign longer than ten minutes runs detached (setsid nohup) with a separate waiter; a background Bash call is capped at 600 s and a cut run wastes its tokens"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 481ce09567dc2886
---

## A SPIKE-001 campaign longer than ten minutes runs detached (setsid nohup) with a separate waiter; a background Bash call is capped at 600 s and a cut run wastes its tokens

A background Bash call is still capped by its `timeout`, which is at most 600,000 ms. On 2026-11-11 (j86, PR #626), the first attempt at the S32 (a) six-run campaign was started that way. It was stopped by hand to avoid a cut mid-session, and its partial raw output was deleted. The README records that this attempt was not cited and its tokens were not counted.

**Why:** a cut run wastes its tokens, and it leaves scratch behind (`TMPDIR/s32-*`, `SPIKE_RAW_DIR/<run>`) that has to be found and removed.

**How to apply:**
- Write the campaign as a script in the scratchpad.
- Start it with `setsid nohup <script> > log 2>&1 < /dev/null &` and save its pid.
- Have the campaign `touch` a done file at its end. Wait with a background `for i in $(seq 28); do [ -e done ] && break; sleep 20; done` loop, and re-arm that waiter until the file exists.
- Do not wait on `$!`. Under `setsid nohup … &`, `$!` is setsid's own pid: it forks and exits at once, so `kill -0` says the run is done while it is still going. Do not wait on `pgrep -f <args>` either: the pattern matches the waiter's own command line, so the loop never ends (seen on j87, 2026-11-11).
- `cargo xtask checks` also runs past 580 s here. Run it detached the same way.
- `nohup` starts the child with SIGHUP ignored, and that is inherited. So a `cargo xtask ci` started with `setsid nohup` fails `tools/ci/test_with_display.sh`'s HUP case ("got 0 after 30s"), while the same case passes alone and in CI. On j87 (2026-11-12) I first blamed load; devex-tooling reproduced the real cause and took the fix as j327. For a suite, detach with `setsid` alone, or check `grep SigIgn /proc/<pid>/status` (bit 0x1 is HUP) before reading a red self-test as the branch's.
- Do not commit or fast-forward in a working copy while a suite runs there. `transport-daemon`'s `version` test compares the binary's built-in commit with HEAD, and it failed after I moved HEAD mid-run.
- S32 (a) small d2 runs took about 40 s each, and roughly 778k tokens and 1.84 USD each.
- Related: [[spike-tmpdir-not-named-tmp]], [[spike-builds-scratch-home-install]].

*References: spike-builds-scratch-home-install, spike-tmpdir-not-named-tmp*

*Anchors: `spikes/spike-001/s32_compact.py::TMPDIR must be outside the repository`*

*Observed 2026-11-11 (rust-services-dev)*
