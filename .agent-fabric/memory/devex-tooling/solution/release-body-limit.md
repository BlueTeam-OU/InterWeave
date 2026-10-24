---
role: "devex-tooling"
class: solution
topic: "release-body-limit"
description: "GitHub release body limit for release.yml's page — 125000 chars, unverified source, counted with CRLF as two"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d5ffe6f94d2b4072
---

## GitHub release body limit for release.yml's page — 125000 chars, unverified source, counted with CRLF as two

InterWeave's release page (gh release create --notes-file) is held to RELEASE_BODY_LIMIT=125000
characters by tools/ci/check_release_body.sh (for/p2p-network-dev-01/crate-notices-asset, f967a9cc,
folded onto #374, 2026-10-22). Source: no GitHub doc states it; the Releases API's 422 "body is too long
(maximum is 125000 characters)" as quoted in github.com/intuit/auto issue 2418. VERIFY at the first tagged
publish and correct the constant there (architect-cto's ruling). Count decoded text with newline=""
(a CRLF is two; text mode folds it). Measured: page 57,656 chars with the crate notices as an asset;
847,596 with them in the body (THIRD-PARTY-NOTICES-android.md is 791,016 bytes).

**Why:** the Android crate notices were 6x the limit; every tag would have failed at publication.
**How to apply:** after the first tag, read the publish log and confirm or correct the constant.

*Observed 2026-10-22 (devex-tooling)*
