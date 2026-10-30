---
role: "architect-cto"
class: workflow
topic: "reflection-device-access-across-logins-is-per-key-per-port-per-lease"
description: "Phones on wireless adb are shared across logins by three facts: pairing admits one adb key per login (a code consumed in my shell paired MY key, the other lane still needed its own or an earlier pairing), one adb server holds host port…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4d5f03764c3aca68
---

## Phones on wireless adb are shared across logins by three facts: pairing admits one adb key per login (a code consumed in my shell paired MY key, the other lane still needed its own or an earlier pairing), one adb server holds host port 5037 at a time (kill-server before another login installs, or use the lease's own port), and the install is the signer's with install -r so data is kept

2026-10-24, the Stage 21 two-phone rows. The owner ran the Samsung's
pairing in MY session's shell, so the device was paired with
architect-cto-01's key and p2p-network-dev-02 still could not connect;
it then accepted their key from an earlier pairing (the gate-b runs),
so no second code was needed, but only a try showed it. While my adb
server was up, theirs could not take port 5037; they killed theirs so
rust-ui-dev-01 could install, who used the phone leases (`phone-run`,
the pixel-9a and samsung-a40 leases) on a port of their own and never
touched 5037. The install was `install -r` on both phones, data kept,
sha256s reported for the run files; nothing wiped until the owner's
word.

**Why:** a device reached over the network looks shared but every
layer is per login (key, server, port); the coordinator who relays the
owner's pairing is the login that gets paired.

**How to apply:** when the owner pairs a phone, have them do it in the
session of the login that will drive it, or say whose key it was and
that the other needs a code; after any adb use of mine, `adb
kill-server`; tell lanes to drive phones through the leases and their
own port; the signer installs, the measuring lane runs, the record
names both logins' parts. See [[j69-reboot-and-cache-rows-2026-10-25]].

*References: j69-reboot-and-cache-rows-2026-10-25*

*Observed 2026-10-25 (architect-cto)*
