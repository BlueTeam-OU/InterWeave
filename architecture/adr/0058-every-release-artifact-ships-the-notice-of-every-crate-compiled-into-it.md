# Every release artifact ships the notice of every crate compiled into it, read from the crate's own publication

**Status:** Accepted — the owner's word of 2026-10-21, on architect-cto's decision of the same day on p2p-network-dev-01's finding (OBSERVATION 01a15c55); proposed on 2026-10-21 and accepted before the pull request that lands this record merged, so it never bound less than the fail-closed refusal already in the tool (decision 6)

## Context

The Linux release archive (Stage 19, plan §22) ships `THIRD-PARTY-NOTICES.md`, written by `tools/ci/release_notices.py` from the licence files each compiled crate publishes. The review of #324 found that a crate publishing a licence *expression* and no licence *file* was listed by the expression and its repository URL alone. Around forty of the crates compiled into the Linux programs are like that, and most are MIT or BSD-3-Clause, whose terms put the copyright and permission notice in every copy. The libp2p family (twenty-two crates), `multistream-select` and `rw-stream-sink` carry that notice only in the header of `src/lib.rs`; others (`taffy`, `uuid-simd`, `base45` among them) carry it nowhere in the crate. Nothing has shipped: the release run has never run, and since 56d0ae4a the tool refuses such a crate rather than drop its notice. The Android APK compiles the same crates, and its release page's `{third_party_notice_texts}` carried the native libraries' notices alone.

What this repository already holds: CLAUDE.md §8 forbids stripping an upstream notice and keeps `deny.toml`'s licence list as an allow-list of exactly what resolves; ADR-0051 decision 5 requires a vendored crate's licence file and recorded provenance before it is built from the tree. Neither says how a *shipped binary* carries the notice of a crate that published no file. This record does.

## Decision

1. **Every release artifact ships the notice of every third-party crate compiled into it**: the Linux archive beside its programs, the Android APK on its release page, and any later artifact the same way. The set is the build's own resolve for that artifact's target and packages, never the workspace-wide one, as `release_notices.py` already computes it. Workspace members are covered by `LICENSE`; a vendored crate under `third_party/` is listed like any other.

2. **The notice is read from the crate's own publication, in this order, all from the registry copy the build compiled** — the tarball whose checksum `Cargo.lock` pins, so what is shipped is what was built:
   1. the licence files the crate publishes (`LICENSE*`, `LICENCE*`, `COPYING*`, `NOTICE*`, `COPYRIGHT*`, `UNLICENSE*`, a `LICENSES/` directory, or its `license-file`), as today;
   2. failing any, **a copyright and permission notice in the crate's own source headers** — the leading comment block of its root module, or of any of its source files — extracted whole. A header is the crate's own publication under the same pinned checksum as a file would be, and counts as the same evidence;
   3. failing both, **the standard text of the licence the crate declares**, under a copyright line built from the crate's `authors` metadata — or, when that is empty, "the `<crate>` authors" — followed by one sentence saying the crate publishes no notice of its own. This is the honest maximum: a notice the author never wrote cannot be copied, and a release withheld over it serves the author no better.

3. **A crate whose notice is derived (2.ii or 2.iii) is named in a committed, reviewed list** in `tools/ci/`, with its licence expression and which of the two cases it is. A compiled crate that falls into either case and is not listed fails the artifact; a listed crate that now publishes a file is reported and its entry removed. The derivation is mechanical; the list is where a person has looked once at each crate it applies to, and it is an allow-list for the reason `deny.toml`'s licence list is one.

4. **No licence election is recorded.** An `OR` expression is satisfied by the term whose text the crate publishes (both, when it publishes both); under 2.iii, by the first term the expression lists. This record relicenses nothing and widens `deny.toml` by nothing.

5. **Android owes the same notices.** The APK's release page carries, under `{third_party_notice_texts}`, the notices of every crate compiled into the native library for the Android target, produced by the same tool from the same rule, beside the native libraries' notices it already carries. ADR-0057 decision 5 is amended to say so.

6. **Until the derivation lands, the tool stays fail-closed** (56d0ae4a): a crate with no licence file refuses the artifact, and no release runs. A release that ships a crate without its notice is the failure this record exists to prevent; a release that does not ship is the lesser one.

## Alternatives considered

- **List the crate by its expression and repository URL** (the state the review found). Rejected: MIT and BSD put the notice in the copy, not a pointer to it.
- **Hand-copied notice files in the tree, one per crate, with provenance** as `license_exempt.txt` records vendored code. Rejected: forty files that go stale on every bump, copying text the pinned tarball already holds; the list of decision 3 keeps the review step without the copies.
- **A recorded election of Apache-2.0 for dual-licensed crates.** Rejected: Apache-2.0 §4 asks for the licence text and the notices just the same, and most of the file-less crates are MIT alone.
- **Refuse forever.** Rejected for the crates of 2.iii: the project would withhold its own release to avoid shipping a notice the author never published, which protects no one.

## Consequences

`THIRD-PARTY-NOTICES.md` carries a text for every compiled crate, and a reviewed list in `tools/ci/` names every crate whose text was derived. A dependency bump that brings a file-less crate fails the release until a person adds the entry. The Android release page grows by the crate notices.

## Security implications

The tool reads the pinned registry copy and nothing else: a header scan is a read of text, never an execution, and a crate cannot inject anything into the notices beyond its own words. The pinned checksum is what makes the shipped text the built text.

## Operational implications

The release run fails at the notices step with the crate named when a new file-less crate arrives; the fix is an entry in the list after a look at the crate, in the PR that brought the crate. The list's stale entries are reported the same way.

## Implementation implications

`tools/ci/release_notices.py`, its list and its tests are p2p-network-dev's (the Linux archive, plan §22), with a regression test over `libp2p-core` (2.ii) and one 2.iii crate. The Android page's placeholder is fed from the same tool for `aarch64-linux-android` by the release run (rust-ui-dev's page, p2p-network-dev's run). Until both land, decision 6 holds.

## Revisit conditions

A crate of 2.iii publishing a notice upstream (its entry moves or goes); a tool such as `cargo-about` adopted for the whole derivation, which would retire the tool and keep the list; a store or distributor with its own notice format.
