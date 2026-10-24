---
role: "devex-tooling"
class: solution
topic: "sdkmanager-failed-attempt-leftovers"
description: "retrying or cleaning up an sdkmanager install in CI? a failed attempt leaves only staging and <pkg>/.installer; never rm -rf a pinned package dir"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 45b42fb7750d8620
---

## retrying or cleaning up an sdkmanager install in CI? a failed attempt leaves only staging and <pkg>/.installer; never rm -rf a pinned package dir

The GitHub runner's sdkmanager is the classic repository-core installer
(its log line "An error occurred while preparing SDK package ..." is
AbstractPackageOperation.prepare). A failed download or unzip leaves only
$ANDROID_HOME/.temp/ (and .downloadIntermediates/) and, at the package's
install path, a .installer/ made before the download; content reaches the
install path only on completion, by an overwrite that restores its backup
on failure. So InterWeave #347 (merged 50c82158, 2026-10-21) retries
`sdkmanager --install` 3x and before each retry removes exactly those:
the two staging dirs, then per stale pinned package `rm -rf $dir/.installer`
and `rmdir $dir` (empty only). Measured: sdkmanager exits non-zero on the
corrupt-zip failure (run 39658097479), so the retry fires.

This host's cmdline-tools 23.0 is different: bin/sdkmanager is a shim to
the `android` CLI, which stages under $SDK/.sdk/ and moves file by file.

**Why:** three review rounds each found a pin (category, ..;sdk,
extras;google, symlinked sdkmanager) that a recursive delete of
"$ANDROID_HOME/<pin path>" could still reach; reading the installer showed
the delete was never needed. Same lesson as
[[shell-reader-allow-list-not-parser]]: remove the dangerous mechanism
rather than refine which inputs may reach it.
**How to apply:** clean up only fixed names the tool itself creates, and
use rmdir for anything a config value names.

*References: shell-reader-allow-list-not-parser*

*Anchors: `tools/ci/android_ci_toolchain.sh::.installer`*

*Observed 2026-10-21 (devex-tooling)*
