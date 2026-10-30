# ADR-0060 — amendment notes

### Amendment 2026-10-21 — A rollback refuses the encrypted profile by the profile-format marker, not by the file's name

SPIKE-011's desktop rows (rust-ui-dev, 0d007761) found that decision
6's "a rollback to a build that reads only plaintext must refuse the
encrypted file by name" had no mechanism: after step (c) the file is
`human.sqlite`, and a plaintext build's open fails `NotADatabase`, the
same error as a corrupt file or a wrong key, so such a build can only
offer its recovery mode and Discard. Ruled: the name is ADR-0028's
profile-format marker (A 2026-10-20, #318), which exists for this.
Step (a) raises the profile format before the rename, every build since
the marker refuses a profile whose format is above its own — naming the
file, both numbers and that a newer build wrote it, writing nothing —
and the plaintext open is never the path a rollback takes. A build from
before the marker reads the file as corruption; that is ADR-0028's
stated limit for every overlay, and nothing has shipped, so it binds no
release. Stage 19's rollback row (plan §22) tests the marker's refusal,
not a file name.

### Amendment 2026-10-30 — A headless custody mode stays available for an automated client's profile

agent-fabric's control plane rides each account's `fabric-control`
endpoint through the command-line client since its records of
2026-10-28 (its ADR-029 and ADR-064 amendments; the j422 design's R4,
accepted on the owner's word), and the design named one constraint for
this record: a lingering control agent cannot answer a passphrase, so
when SPIKE-007 lands ADR-0038's envelope the desktop store key must not
become reachable only through an interactive unlock. Decision 3's
desktop bullet now keeps `filesystem-only` as a selectable custody for
an automated client's profile and has the envelope unlock from a
credential source no person types (ADR-0038 amended the same day), so
a restart opens the store unattended as the Android bullet already
requires; SPIKE-007's adoption is bounded by it. Operational
implications say who runs the destructive operations on such a
profile: `store discard --confirm <scope>` is the operator's over the
host's administrative path, never the control agent's; a lost endpoint
key is ADR-0067 decision 5's new identity, minted by the deployment
with `transportctl endpoint init` and a roster re-render on the owner's
word (the key change the design still named was withdrawn by ADR-0067
on 2026-10-28). Propagation: decision 3; Operational implications; this
note; the log row; the digest entry; ADR-0038's amendment; SPIKES.md's
SPIKE-007 evidence line.
