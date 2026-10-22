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
