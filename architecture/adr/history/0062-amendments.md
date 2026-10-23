# ADR-0062 — amendments

### Amendment 2026-10-22 — How a scan reaches the app, and what "no LAN peer" means

ux-designer's flows for plan §29 step 5 asked two things the record
did not say. The owner chose, on 2026-10-22, the app's own camera on
Android (asked at the first Scan only) and an image file or a pasted
image on the desktop, no webcam; so decision 8 says where the camera
is opened, when the permission is asked, that the camera feature is
declared not required, and that plan §29 step 2's manifest check pins
the permissions as an allow-list holding `CAMERA` by this record, with
§29's exit gate (e) holding the first-Scan ask and the refusal as
device rows and step 2's tree check holding the camera APIs to the
Scan module. Decision 8's "no LAN peer" was read as a
discovery fact the client could see; it cannot — `LOCAL-CLIENT.md`'s
`ServerState` carries health and the connectivity summary only — so
the notice keys on the overlay empty and the trust set empty, two
facts the client holds whole, and on no connection or path signal (the
round-1 wording read the connectivity summary, whose counts name no
peer; the round-2 wording read a session's per-peer path state, which
exists only for peers it has exchanged a message with, as #362's
round-3 review found).
Propagation: decision 8; this note; the log row; the digest entry;
plan §29 step 2 (the allow-list and the tree check) and its exit gate
(e) (the cases);
`human-client-ui.md` §3's paragraph on where a peer's address comes
from.

### Amendment 2026-10-22 (ii) — "Relay-only" is defined: no externally confirmed address exists at the read

Decision 4 used "when the node is relay-only" without saying what the
word meant; `LOCAL-IPC.md`'s `admin.peers.own_addresses` paragraph
(approved 2026-10-22, p2p-network-dev's row) defined it for the read
as "no confirmed-external row", and p2p-network-dev-02 asked before
the review whether the record read it otherwise. It does not: the
decision now says the circuit route is shown exactly when no
externally confirmed address exists at the moment of the read, AutoNAT's
state being `admin.status`'s and not the list's. The same change adds
`LOCAL-CLIENT.md` §7 item 14, the show side's conformance item, beside
item 13's. Propagation: decision 4; this note; the log row; the digest
entry; `LOCAL-CLIENT.md` §7.
