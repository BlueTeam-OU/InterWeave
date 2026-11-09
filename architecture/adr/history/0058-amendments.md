# ADR-0058 — amendments

### Amendment 2026-10-22 — A header without a copyright line is no notice; a header naming an unpublished licence file ships with the standard text

The first derivation over the Linux build (p2p-network-dev's j99: 461
third-party crates, 42 publishing no licence file) met two shapes
decision 2.ii did not settle. Seven crates carry a header that names
its licence and points at a LICENSE file the crate does not publish;
taken whole under 2.ii that ships a copyright line and no licence text,
which Apache-2.0 §4(a) does not satisfy. One crate carries only an SPDX
tag, no copyright line and no permission text. Ruled: a header counts
for 2.ii only with a copyright line, so the tag-only crate falls to
2.iii (its standard text committed like the others); a header that
names an unpublished file ships whole followed by the standard text of
the term decision 4 selects, with no invented copyright line, recorded
as the case header+text; a header carrying a full permission notice
ships alone. Decision 3's list records the three cases and is keyed by
name and version. The same day the review of the derivation's PR
found a P1 in the shape the first text allowed: "the leading comment
block of its root module, or of any of its source files" had been
read as the first header found, and nine crates carry other holders'
notices in other compiled files — three of them a BSD-style header
naming a file the crate does not publish, whose binary clause the
shipped notice then failed. So 2.ii is every distinct header of the
compiled files (`src/`, `build.rs`; not `tests/`, `examples/` or
`benches/`), deduplicated by text, root first; a header naming a
licence the notice does not otherwise carry gets that licence's
standard text, recorded in the list per crate; and the list carries
the header count so an added header is looked at again. Propagation: decisions 2 and 3; this note; the log
row; the digest entry.

### Amendment 2026-10-22 — The APK page's crate notices have a placeholder of their own, `{crate_notices}`

Decision 5 said the APK page carries the crate notices "under
`{third_party_notice_texts}`". At the first fill (j99's
`release_notices.py --json crate_notices`, 379 crates), rust-ui-dev
supplied a placeholder of its own after that one, with an introducing
sentence, and p2p-network-dev asked which holds: one value joined by
the step, or two placeholders each filled by its own tool. Ruled the
second, as they recommended: no string joining in the release step,
and the page itself says which texts are the native C libraries' and
which the Rust crates'. Propagation: decision 5; this note; the log
row; the digest entry.

### Amendment 2026-10-22 (ii) — The Android crate notices are a release asset, linked and hashed from the page

The first derivation on the Android target measured what decision 5
had asked the page to carry: 379 crates, 281 distinct texts, about
790,000 characters of `{crate_notices}`, a page of about 847,000
characters where the body without it was about 57,000; a GitHub
release body takes about 125,000, so every tagged release would have
failed at `gh release create` (the review of the derivation's PR,
#374, a P1). The prior wording, "the APK's release page carries the
notices", is replaced: the crate notices are a Markdown file of their
own attached to the release beside the APK, its SHA-256 in the page's
hash list, and `{crate_notices}` holding the link and `{crate_notices_sha256}` the hash; the
native libraries' notices, about 50,000 characters, stay in the body;
and the publish step refuses a page over the body limit, the limit a
named constant with its source, verified at the first publish. The
alternative, a committed notices page per release, was heavier and
further from the artifact carrying its own notices. Propagation:
decisions 1 and 5; this note; the log row; the digest entry; ADR-0057
decision 5's template sentence (rust-ui-dev-02's and
language-culture-en's copy); `release.yml`'s upload (devex-tooling's).

### Amendment 2026-10-22 (iii) — Each header's digest is pinned in the list; the licence-word scan is a tripwire, not the closure

Decision 2.ii asked the tool to refuse a header naming a licence whose
text the notice does not carry, and decision 3 asked the list to hold
a count of distinct headers. Tightening that scan (#381, j108) went
two review rounds the way #374 had gone five: case-insensitive names,
SPDX ids matched by a fragment (Apache-1.1 answered by the Apache-2.0
text, BSD-4-Clause by BSD-3), a recognised name hiding an unrecognised
one beside it — each fixed, and a review-class enumeration then ruled
that no rule over free prose closes the class: a licence named without
the word ("GPLv3", "CC-BY-4.0", "the Frobnicate Agreement") and a
version written apart from its name ("Apache License, Version 1.1",
"the 4-clause BSD license") escape any word scan. A count catches a
header added by a version and nothing a header's text changed under
the same version, which a vendored crate's can. So the list pins what
was read: the SHA-256 of each distinct header text, whitespace folded,
in place of the count; a header whose digest is not listed refuses the
artifact and a listed digest no header carries is reported; and the
scan stays as a tripwire that closes nothing. Measured by the judge
against the list of the day: 77 digests across 27 entries, 40 distinct
texts, none of the 50 real crates refused. p2p-network-dev-01 asked
the ruling (REQUEST 01a164e0); the code is theirs, after #381.
Propagation: decisions 2.ii and 3; this note; the log row; the digest
entry; the README row.

### Amendment 2026-10-25 — The derivation landed and the first release ran under the fail-closed tool

Decision 6 opened "Until the derivation lands, the tool stays
fail-closed ... and no release runs". The derivation landed with #374
(65c3e4c7, 2026-10-22), and the first release, v0.1.0, ran on
2026-10-25 with the Linux archive's `THIRD-PARTY-NOTICES.md` and the
APK page's `THIRD-PARTY-NOTICES-android.md` written by the tool (plan
§22). The rule itself is unchanged — a crate whose notice cannot be
derived still refuses the artifact — so the sentence loses its
"until" and names the release that ran under it. Propagation: this
note; decision 6; the log row; the digest entry; the README row.

### Amendment 2026-11-09 — A build-dependency's compiled sources are the artifact's crates; a second list classifies every build-dependency; the notices are checked on the PR

Decision 1 said "every third-party crate compiled into it" and described
the set as "the build's own resolve for that artifact's target and
packages, never the workspace-wide one, as `release_notices.py` already
computes it". The tool walked normal dependencies only, and so did a
reader of the record: a build-dependency runs at build time, but a
`-src` crate's C sources are compiled into the program by its
dependent's build script — `sqlcipher-src`'s SQLCipher under
`libsqlite3-sys` 0.39.1 (which arrived with #602's `rusqlite` 0.41 bump,
295cf217a), `openssl-src`'s OpenSSL under `openssl-sys` — and neither
notice was in `THIRD-PARTY-NOTICES.md` or the Android crate notices
(measured by devex-tooling on 2026-11-09: the tool on `main` emitted
neither crate on either graph). The principle was right and its text
incomplete, so this is an amendment and not a superseding record:
decision 1 names a build-dependency whose sources are compiled in as a
compiled crate of the artifact and a tool build-dependency as owing
nothing; decision 2.i admits, for that class, the notice files beside the
sources inside the crate, by path, since its top-level files cover build
code that is not shipped; decision 3 gains the second reviewed list,
`tools/ci/notice_sources.json`, every build-dependency of a compiled
crate classified as a tool with its reason or as sources by name,
version, licence and files, an unclassified one or a version mismatch
refusing the artifact. Consequences said a bump "fails the release";
since the same day a tree check (`tools/checks/check_release_notices.sh`)
runs the tool over each shipped build's graph as the release does, so the
refusal is red on the PR that brought the crate. One classification
recorded as a judgement: `gl_generator` is a tool — it emits function
tables generated from the Khronos registry's data, and no registry text
is copied into the output. Propagation: this note; decisions 1, 2.i and
3, Consequences, Operational and Implementation implications; the
amendments row; the README row; the digest entry and keyword row.
