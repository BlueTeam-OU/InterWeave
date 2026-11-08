---
role: "rust-ui-dev"
class: solution
topic: "android-e2e-dump-reads"
description: "android-e2e: every dumpsys read goes through platform::dump (cut dump = error); tests/dump_reads.rs fails on a raw \"dumpsys\" word (j214, 02155fad2)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b04caaaaeadb68a0
---

## android-e2e: every dumpsys read goes through platform::dump (cut dump = error); tests/dump_reads.rs fails on a raw "dumpsys" word (j214, 02155fad2)

Since #568 (p2p-network-dev-02) `tests/android-e2e/src/platform.rs` has `dump(&[args])`: it appends `; echo IW_END=$?` and panics on a dump not ending `IW_END=0`, carrying `DUMP TIMEOUT` or `Exception occurred while dumping`. A case that reads a negative ("no notice", "not focused") off a raw `adb shell dumpsys` reads a cut dump as that negative.
j214 (commit 02155fad2, 2026-11-05) moved app_lifecycle.rs onto it and added `tests/android-e2e/tests/dump_reads.rs`, a host test failing on any `"dumpsys"` string literal in the crate's src/ and tests/ outside `platform::dump_command` (platform.rs's `#[cfg(test)]` module excepted); mutation-checked with an uncompiled scratch .rs in src/.
**How to apply:** a new device case reads dumps with `platform::dump`, and shell/sdk/app_pid/adb from `platform`, never its own `Command::new("adb")` (platform::adb follows a moved serial).

*Observed 2026-11-05 (rust-ui-dev)*
