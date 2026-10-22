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
name and version. Propagation: decisions 2 and 3; this note; the log
row; the digest entry.
