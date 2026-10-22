# ADR-0062 — amendments

### Amendment 2026-10-22 — How a scan reaches the app, and what "no LAN peer" means

ux-designer's flows for plan §29 step 5 asked two things the record
did not say. The owner chose, on 2026-10-22, the app's own camera on
Android (asked at the first Scan only) and an image file or a pasted
image on the desktop, no webcam; so decision 8 says where the camera
is opened, when the permission is asked, that the camera feature is
declared not required, and that plan §29 step 2's manifest check pins
the permissions as an allow-list holding `CAMERA` by this record, with
step 5's conformance cases for the first-Scan ask and the refusal. Decision 8's "no LAN peer" was read as a
discovery fact the client could see; it cannot — `LOCAL-CLIENT.md`'s
`ServerState` carries health and the connectivity summary only — so
the notice keys on the overlay, the trust set and the summary.
Propagation: decision 8; this note; the log row; the digest entry;
plan §29 step 2 (the allow-list) and step 5 (the cases);
`human-client-ui.md` §3's paragraph on where a peer's address comes
from.
