---
role: "devex-tooling"
class: solution
topic: "skia-pin-check-job-cache"
description: "android check job reads the Skia pin — why its cargo cache key hashes tools/ci/skia-binaries.pin, and what the step self-test must pin besides run text"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 152f3d5638055c17
---

## android check job reads the Skia pin — why its cargo cache key hashes tools/ci/skia-binaries.pin, and what the step self-test must pin besides run text

InterWeave's android check job runs `tools/ci/skia_prebuilt.sh fetch` before its `cargo check` and `check target debug` after it. The job and the release run share one pin, `tools/ci/skia-binaries.pin` (j182, PR #566, ADR-0057 A 2026-10-20).

- **Debug profile.** The check reads `debug` because `cargo check` writes the dev profile. The Skia archive key does not change with the profile: only `SKIA_DEBUG` moves it (skia-bindings 0.153.3 `build.rs`). The five-package feature union resolves to the same Skia features as the release build's `-p interweave-human-android` (`cargo tree -i skia-bindings -f '{p}|{f}'`).
- **Cache key.** `check` reads EVERY `skia-bindings-*/output` in `target`. So the cache key and both restore keys must hash the pin, or a cache made under another pin turns a correct build red.
- **Rerun on the URL.** skia-bindings emits `rerun-if-env-changed=SKIA_BINARIES_URL`, so a new URL reruns its build script.
- **Self-test scope.** `tools/ci/test_android_check_skia.sh` runs ci.yml's step text under `bash -e`. Review F1 showed that running the text is not enough: a step's `continue-on-error`, `if:` or a step-level `env:` (which beats `GITHUB_ENV`) passed it. The test now also requires the three steps to be `name` and `run` only.
- **Measured.** CI job 127229647413 printed "took the pinned archive … 1 output(s)" on 58cf7811.

*Observed 2026-11-05 (devex-tooling)*
