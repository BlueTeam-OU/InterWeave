# ADR-0049 — amendment history

### Amendment 2026-11-25 — The two checks run under `cargo xtask checks`

Operational implications said `validate_contracts.py` and `verify_fixture_vectors.py` "run before pushing and are listed in `AGENTS.md` §9". As this amendment was written, §9 listed no check by name: it had been cut to the one sentence that `cargo xtask checks` runs every tree check, because a list of scripts in prose goes stale while the scripts do not. The citation therefore pointed at a list that was not there.

The sentence now names what a reader runs: `cargo xtask checks` runs both with every other tree check, and CI invokes each by name. Nothing about the checks, their `--root` flag or what they assert changed.
