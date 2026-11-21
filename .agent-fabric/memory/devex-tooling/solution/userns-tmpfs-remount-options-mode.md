---
role: "devex-tooling"
class: solution
topic: "userns-tmpfs-remount-options-mode"
description: "remounting a tmpfs inside `unshare -r` fails \"tmpfs: Invalid uid\" unless mount --options-mode ignore; ci.yml namespace probe covers it"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7be12ab2c73731e4
---

## remounting a tmpfs inside `unshare -r` fails "tmpfs: Invalid uid" unless mount --options-mode ignore; ci.yml namespace probe covers it

Inside an unprivileged user+mount namespace (`unshare -rm`), a plain
`mount -o remount,size=4m <tmpfs>` fails with "fsconfig() failed: tmpfs:
Invalid uid '<host uid>'" (exit 32): mount(8) re-sends the uid= it reads
back. `mount --options-mode ignore -o remount,size=…` works (util-linux
>= 2.39; host has 2.41.6).

InterWeave's ci.yml "Allow unprivileged user namespaces" step probes this
since j339 (commit 08352e256, for/ branch to rust-ui-dev-02, folded into
their S21-10 PR adding crates/human/store/tests/full_filesystem.rs):
size=1m tmpfs under /run/cap, remount to 4m, `df --output=size` = 4096.
Mutations measured: option removed -> exit 32; remount to 2m -> exit 1.
The step's comment list is the register of every test entering a
namespace in the rust job; a new one is a new line there.

*Anchors: `.github/workflows/ci.yml::--options-mode ignore -o remount,size=4m`*

*Observed 2026-11-16 (devex-tooling)*
