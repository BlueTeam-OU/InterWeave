---
role: "devex-tooling"
class: solution
topic: "build-dep-sources-owe-notices"
description: "release notices — a -src build-dependency's C sources ARE compiled in; notice_sources.json classifies every build-dep"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1e8f21bd94b6531a
---

## release notices — a -src build-dependency's C sources ARE compiled in; notice_sources.json classifies every build-dep

A build-dependency can put code in the program: `sqlcipher-src` (via libsqlite3-sys) and `openssl-src` (via openssl-sys, vendored) are compiled into the desktop, CLI and Android builds. Until j307, `tools/ci/release_notices.py` walked normal deps only, so SQLCipher's BSD-3 and OpenSSL's Apache-2.0 texts were missing from both release notice pages.

Since j307 (#604, merged cb36a1e70 on 2026-11-09, with ADR-0058 A 2026-11-09), every build-dependency of a compiled crate (workspace members included) must be listed in `tools/ci/notice_sources.json`:
- `tools`: listed by name, with a reason;
- `sources`: listed by name and version, with the licence and the notice files inside the crate.

`tools/checks/check_release_notices.sh` runs the tool over both shipped graphs in tree checks, so an unclassified or bumped -src crate turns the PR red.

Traps:
- `cargo metadata` resolves features for the whole workspace and over-reaches. Use only the crates the `cargo tree -e normal` compiled list names.
- A -src crate's own top-level LICENSE covers its Rust build code. The notice owed is the bundled library's file in a subdirectory, e.g. `sqlcipher/LICENSE-sqlcipher` or `openssl/LICENSE.txt`.
- Skia on Android is a download handled by ADR-0057's `skia_notices.py`, not by this list. Skia is not in the Linux graph.
- A sources crate that another crate also compiles as a normal dependency is one row: its own licence texts plus its entry's files (review F1 on #604). An earlier version skipped it whenever it was in the compiled set, and that silently dropped the C notice.

*Anchors: `tools/ci/release_notices.py::def built_in`, `tools/ci/notice_sources.json::sqlcipher-src`, `tools/checks/check_release_notices.sh::release_crate_notices`*

*Observed 2026-11-09 (devex-tooling)*
