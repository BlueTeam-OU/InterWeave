---
role: "architect-cto"
class: threads
topic: "adr-0058-third-party-notices-2026-10-21"
description: "ADR-0058 (accepted 2026-10-21): how a release ships the notice of a crate with no licence file; who implements what; what the acceptance round taught"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 353ce608a9870e43
---

## ADR-0058 (accepted 2026-10-21): how a release ships the notice of a crate with no licence file; who implements what; what the acceptance round taught

p2p-network-dev-01's OBSERVATION 01a15c55 (2026-10-21): #324's review
(Codex F1, judged P1) found THIRD-PARTY-NOTICES.md listed ~42 compiled
crates with no licence file (libp2p family: notice in src/lib.rs headers;
taffy, uuid-simd, base45: nowhere) by expression and URL alone. Since
56d0ae4a the tool refuses them; no release has run.

**Ruled (ADR-0058, accepted by the owner 2026-10-21):** source order from the pinned registry copy:
licence files → source-header notice (same evidence) → declared licence's
standard text under `authors` + "publishes no notice"; derived crates in a
reviewed list in tools/ci (unlisted fails, stale reported); no licence
election; Android page owes the same (ADR-0057 A 2026-10-21, decision 5);
fail-closed until the derivation lands.

**Supplied:** 8812a039 (ADR + README row + digest) and 4001c1a3 (0057
triple) on for/p2p-network-dev-01/j94-third-party-notices-adr from #324's
head 293ffd06. REPLY to p2p-01, DECISION to rust-ui-dev-02 (page holder via
language-culture-en's for/ branch), REQUEST to the owner for acceptance.

**Open:** p2p-01's fold of the acceptance flip 2d5f11c2 (for/…/j94-adr-0058-accepted from #324 head 7d95615f, REPLY 01a15cc3); p2p-01's
derivation PR with the list and the libp2p-core + one case-iii test; the
Android run feeding the placeholder. Related: [[android-no-store-no-play-services]].

- Round-2 F2 on #324: a third cell in the two-column ADR index is dropped by the renderer, so a status word there is invisible and validate_adr_index does not catch it; the status goes in the decision cell. Fixed 55d02006 (also status line, digest "Proposed", and the accepted 0057 amendment naming the cited rule as proposed).
- Carried P3s to supply onto p2p-01's j99 derivation branch (REPLY 01a15c61 answered: carry): README.md:3 "unless marked Proposed or later superseded"; ADR-0057 log row 2026-10-21 Effect cell "(pending ADR-0058's acceptance)"; plus the Accepted flip of ADR-0058 once the owner speaks.
- Accepted 2026-10-21, in session, before #324 merged: the flip is one commit over five files (status line, README row, digest heading+entry, 0057 decision 5 + history note + digest line); the carried P3s dissolved rather than being edited. fabric-coordinator declined to accept (project truth is the owner's) and suggested a legal-expert read of the standard-text fallback: not yet done, a Revisit item.

**2026-10-22 A (header cases), on j99's first derivation (461 crates,
42 file-less):** a header counts for 2.ii only with a copyright line (a
lone SPDX tag → 2.iii); a header naming an unpublished LICENSE file
ships whole + the elected term's standard text (case header+text); a
full permission notice ships alone; the list (tools/ci/notice_derived.
json) keyed by name+version with the case; texts under tools/ci/
licence_texts/ pinned by SHA-256 in license_exempt.txt. Supplied onto
j99. Also 2026-10-22: the CLI batch 2 landed (#369 15181342): cards,
proposals, contacts verbs.

**2026-10-22 (ii), the asset ruling:** the first Android derivation measured {crate_notices} at ~790k characters (379 crates, 281 texts) against GitHub's ~125k release-body limit, so decision 5 was amended: THIRD-PARTY-NOTICES-android.md attached to the release beside the APK, SHA-256 in the page hash list, {crate_notices} = link + hash, native libraries' texts stay in the body, publish step refuses an over-limit page (limit a named constant with its source, verified at first publish). Supplied 30ca5920 onto #374; template clause → rust-ui-dev-02 + language-culture-en, release.yml upload + check → devex-tooling, both as supplies to #374. Same day, 2.ii became every distinct header of the compiled files (28de1cba).

**A 2026-10-22 (iii), header digests (j289, REQUEST 01a164e0 from p2p-network-dev-01):** no word scan over free prose finds every licence a header names (a review-class enumeration on #381 ruled it; #374 went five rounds, #381 two). Decision 3 now pins the SHA-256 of each distinct header text, whitespace folded, in place of the header count; an unlisted digest refuses, a listed digest no header carries is reported; the 2.ii scan is a tripwire only. Supplied as fe798521 on `for/p2p-network-dev-01/adr-0058-header-digests`; #381 lands first, the pinning is their next PR with the amendment as its first commit. Measured by the judge: 77 digests, 27 entries, 40 distinct texts, 0 of 50 real crates refused.

*References: android-no-store-no-play-services*

*Observed 2026-10-21 (architect-cto)*
