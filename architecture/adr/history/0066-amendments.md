# ADR-0066 — amendment history

### Amendment 2026-11-10 — The first release under this record was v0.2.0

**Trigger.** Implementation implications closed on "The first release under
it is a PATCH (ADR-0065): `v0.1.1`, which also states the archive's minimum
platform; plan §22's record names it when it runs." No `v0.1.1` was cut. The
owner's next tag was `v0.2.0`, made 2026-10-27 on 32ade966 and published
2026-10-28: its tag is the first signed with the dedicated tag key and its
archive the first with a `.minisig` beside it, and its page is the first to
state the glibc floor (plan §22). Found while reading the tree before the
`v0.3.0` cut.

**Ruled.** Nothing of the decision changes. The sentence is past tense and
names `v0.2.0`, a MINOR by ADR-0065 decision 3. `v0.1.0`'s archive stays the
one published without a signature.
