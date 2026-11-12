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

### Amendment 2026-11-10 — The tag key was rotated before v0.3.0

**Trigger.** On 2026-11-10 the owner could not sign `v0.3.0`: the passphrase
of the pinned tag key, `220728A8B44731A93CFF8969FA651DF333C31A0C` (pinned
2026-10-27 by 739594ca9), was lost. Nothing says the key was disclosed, and
without its passphrase it signs nothing more. It signed one tag, `v0.2.0`
on 32ade966c (checked this day: `git verify-tag` against the pin as it stood
reports that fingerprint as the valid signer). No `v0.3.0` tag was on the
remote when this note was written.

**What was done.** The owner made a second tag key in the owner's own
account: `7FB9C27BAA313CAE4C1F9485ACCFA5D9BC9B4F61`, Ed25519, sign and
certify, no expiry (read from the public block with `gpg --show-keys`). Its public block reached the tree as decision 3 says, a
pull request to `packaging/release-keys.toml` that replaces `[tag]` whole
(devex-tooling's, the one this note lands with); the first key is in no pin
from that merge on. `v0.3.0` is signed on a commit that holds the new pin,
since the `tag` job reads the tagged commit's own copy (decision 4). The
archive key did not change.

**Ruled.** Nothing of the decision changes; decision 1 already said a lost
tag key is replaced by a pull request and signs the next tag. Two
consequences the body now states where a reader looks for them. Decision 3:
a tag signed by a key that left the pin is verified against the pin of its
own tagged commit, which the file's history keeps, and no longer against
`main`'s — `main`'s pin never carries a key that is not in use, so a rerun
of an old tag's release is refused at the second reading, as decision 4
intends for a dropped key. Operational implications: a secret key whose
passphrase is lost is a lost key, so the backup the owner keeps is of the
key and of what unlocks it.
