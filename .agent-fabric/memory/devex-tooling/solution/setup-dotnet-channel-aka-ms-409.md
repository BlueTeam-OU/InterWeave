---
role: "devex-tooling"
class: solution
topic: "setup-dotnet-channel-aka-ms-409"
description: "gzapp CI .NET install failing with HTTP 409 from dotnetorigin.blob.core.windows.net; why '10.0.x' breaks and an exact version does not"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - fdaf2d27681e6e79
---

## gzapp CI .NET install failing with HTTP 409 from dotnetorigin.blob.core.windows.net; why '10.0.x' breaks and an exact version does not

From ~18:56Z on 2026-10-25 every gzapp backend job failed at actions/setup-dotnet v6.0.0 (a98b5685) with `dotnet-version: '10.0.x'`: the install script is called with `--channel 10.0`, which takes the aka.ms route, and `https://aka.ms/dotnet/10.0/dotnet-sdk-linux-x64.tar.gz` 301s to `dotnetorigin.blob.core.windows.net/...`, which answers 409. The aka.ms route has no feed fallback. With `--version 10.0.403` (an exact version) the same bundled script uses `builds.dotnet.microsoft.com/dotnet` then `ci.dot.net/public`, which answered 200 (measured with `install-dotnet.sh --dry-run` from setup-dotnet v6.0.0's externals/). The LTS runtime pre-pass fails the same way but only warns.
Evidence: merge_group run 40223249199 (evicted #1170), PR run 40223484301. Five setup-dotnet uses: _backend.yml (x2), _db-migrations.yml, coverage.yml, major-updates-report.yml. No global.json in gzapp.
Fix taken as j236: pin the exact SDK in every use and a guard modelled on tools/checks/check_flutter_pin_is_consistent.sh (wired in _ban-checks.yml beside it), so a floating `x` pin cannot return. Below 8 work commits it needs the owner's or architect-cto's word to arm. Tell backend-dev-01 #1170's eviction was this, not their diff.

**Why:** a floating SDK pin let a green build go red with no commit; [[host-rust-floats-rustup-via-fabric-tools]] is the same shape.
**How to apply:** any setup-* action given a wildcard version: pin it exactly, and guard the pin.

*References: host-rust-floats-rustup-via-fabric-tools*

*Anchors: `gzapp:.github/workflows/_backend.yml::dotnet-version`, `gzapp:tools/checks/check_flutter_pin_is_consistent.sh::flutter-version`*

*Observed 2026-10-25 (devex-tooling)*
