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
hash list, and `{crate_notices}` holds the link and the hash; the
native libraries' notices, about 50,000 characters, stay in the body;
and the publish step refuses a page over the body limit, the limit a
named constant with its source, verified at the first publish. The
alternative, a committed notices page per release, was heavier and
further from the artifact carrying its own notices. Propagation:
decisions 1 and 5; this note; the log row; the digest entry; ADR-0057
decision 5's template sentence (rust-ui-dev-02's and
language-culture-en's copy); `release.yml`'s upload (devex-tooling's).
