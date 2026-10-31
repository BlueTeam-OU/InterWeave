---
role: "architect-cto"
class: threads
topic: "adr-0066-release-signing-2026-10-25"
description: "j366 2026-10-25 — ADR-0066 release signing (owner's OpenPGP tag key by hand, minisign archive key in the release environment, pin packaging/release-keys.toml read from the tagged commit and from main's tip, fail closed) supplied onto…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - efeb8475b9bdeabb
---

## j366 2026-10-25 — ADR-0066 release signing (owner's OpenPGP tag key by hand, minisign archive key in the release environment, pin packaging/release-keys.toml read from the tagged commit and from main's tip, fail closed) supplied onto devex-tooling's j231 branch; what the record claims and does not, and what other lanes owe in the same PR

**Decided 2026-10-25** (DECISION 01a17326-c5db to devex-tooling; the owner's go-ahead for j231 given in devex-tooling's session): ADR-0066 — the release tag is an annotated tag signed by the owner's own OpenPGP key, by hand, never in an agent's account or a secret; the `sign` job signs the Linux archive with a minisign key held in the `release` environment and publishes the `.minisig`; both public keys pinned in `packaging/release-keys.toml` (`[tag] fingerprint`, `public_key`; `[archive] minisign_public_key`), read by the `tag` job from the tagged commit and by the `sign` job from `main`'s tip (so a dropped key signs nothing whatever commit is tagged); empty pin or unverifiable signature refuses the run before it publishes; provenance attestation additive. ADR-0057 security implications amended (A 2026-10-25 "The release's signatures are ADR-0066's"); `apps/human-android/RELEASE.md` Signing confirmed.

**Supplied** as one commit, 6870962d, onto `develop-qzapp/devex-tooling/feat/j231-release-signing-and-glibc-floor` over their 67510116 (the branch moved past b4b5911e while the supply was in review; rebased once, no conflict), after two blind rounds (round 1: P2 + 3 P3; round 2: 2 P3; all taken). The tag key is a DEDICATED sign-only OpenPGP key, not the owner's personal key, which every agent account signs commits with (v0.1.0's tag was signed with that personal key). The archive AND its .sha256 each get a .minisig. The PR is devex-tooling's; it also carries the glibc 2.35 floor ([[linux-archive-glibc-floor-v0-1-1-2026-10-25]]).

**What the record is careful NOT to claim** (the round-1 P2): the shared GitHub account every agent pushes and tags as cannot produce any signature the pinned keys verify, but it CAN publish an unvouched release — a tag on a branch commit whose edited workflow drops the checks and the `sign` job (the `publish` job holds a write token under no environment and no reviewer), or a release created through the API with no workflow. The defence is verification against the tree's key, not non-publication; a replaced page with the attacker's key passes a person who takes the key from the page. Never write "publishes nothing" about that account.

**Owed in the same PR by their lanes, named in the record:** the release page template's "The Linux archive has no signature, so this value is the only check" plus the verify command (`apps/human-android/release/release-page.md.in`, rust-ui-dev + language-culture-en, `tests/release_page.rs`); (`packaging/linux/README.md`'s sentence was rewritten by devex-tooling's 67510116 already;) `packaging/android/README.md`'s "The signing key" table gaining the minisign variable and the once-only procedure its steps (devex-tooling). Pre-existing stale lines in RELEASE.md (line 7 "is Stage 19's", line 16 "today 0.1.0, unreleased") are rust-ui-dev's, not touched.

**Why:** v0.1.0's archive had only its hash on the page it is downloaded from, and the run never asked whether the tag was signed while agents tag as the owner's account; the tag key moves the owner's act out of the shared identity.
**How to apply:** cite ADR-0066 only once devex-tooling's j231 PR is on main ([[reflection-cite-an-amendment-only-once-its-pr-is-on-main]]); the first release under it is v0.1.1, which needs the owner's minisign key pair (secret in the environment, public key in the pin by PR) first.

**LANDED 2026-10-25 17:19Z:** #446 merged at fe6a85f0 with ADR-0066 (6870962d) in it (devex-tooling REPLY 01a17412). The owner confirmed point 1 before the arm: a dedicated tag key the owner makes and holds, never on an agent login, each release tag signed by the owner's hand (recorded in #446's arming comment). `packaging/release-keys.toml` is empty on main, so every tag is refused until a pin PR carries the owner's tag key and minisign public key — the owner's next step before v0.1.1.

*References: linux-archive-glibc-floor-v0-1-1-2026-10-25, reflection-cite-an-amendment-only-once-its-pr-is-on-main*

*Observed 2026-10-25 (architect-cto)*
