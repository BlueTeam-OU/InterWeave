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

### Amendment 2026-10-22 (iii) — A scan counts its new lines against the bound

Decision 4 said a scan "counts every line against the bound". Building
`admin.peers.add_addresses` (IPC 2.9, p2p-network-dev's j62),
p2p-network-dev-02 asked (QUESTION 01a1650c) whether that means every
scanned line or only the lines the overlay would gain, since their
branch counts the latter: a line already held (authored or added) or
repeated earlier in the list is answered `unchanged` and counts
nothing. Ruled the latter, and the text now says so: the bound of 64
is the overlay's size (decision 2), a line already held grows it by
nothing, and the single `add_address` already answers a held line
`unchanged` at the bound, so a one-line scan and the single add agree;
under the other reading a person who rescans a code they already added
with 60 entries held is refused for no gain. The clause was written to
keep a scan from passing the bound as one operation — the new lines are
still counted as a whole and refused by name with nothing written —
not to count what is already there. Propagation: decision 4; this
note; the log row; the digest entry; `LOCAL-IPC.md`'s
`add_addresses` paragraph (p2p-network-dev's, on their branch) and
`LOCAL-CLIENT.md` §7 item 13's sentence for the method.

### Amendment 2026-10-23 — An address naming this node's own PeerId is refused by the method, `own_peer`

rust-ui-dev-01 found on #384, through its review, that
`admin.peers.add_address` stored an address whose PeerId is this
profile's own: none of decision 1's refusals covered it, so pasting
one's own Show page listed a row that could never be trusted and
reached no one. ux-designer-01 asked (QUESTION 01a165de) whether the
client alone should refuse it, the runtime, or neither. Ruled the
method, with the client in front: decision 1 makes the add the one
gate these entries pass, whoever the caller, and a row for oneself is
dead for `transportctl` and the command-line client as much as for
the screen; the client refuses first under the same name, since it
knows its PeerId from the session, and the method's refusal makes the
rule true for every other caller. The check is the address's final
PeerId, so a circuit route through this node's own relay to another
peer is not refused; removal is never refused, so an earlier row can
be taken out; below IPC minor 2.9 the refusal is answered as
`grammar`, a downgrade by version an older decoder can show, which
`LOCAL-IPC.md` states. The value joins the `refused` set in the 2.9
batch (p2p-network-dev's, 06dc78cc). Propagation: decision 1; this
note; the log row; the digest entry; `LOCAL-IPC.md`'s two paragraphs
(p2p-network-dev's, same branch); the add-a-peer design note (#390).

### Amendment 2026-10-26 — A carrier-assigned cellular address is not shown

rust-ui-dev-01's device row of 2026-10-26: the Pixel 9a's Show listed
the Wi-Fi address and two carrier-assigned `10.x/32` addresses on its
mobile-data interfaces, and a second phone on the same Wi-Fi scanned
all three, two of which it could never reach. Decision 4 said "the
classes the add admits", and a private range is one; the rule it
inherits now is ADR-0052 rule 10 (A 2026-10-26): a private-range
address of this host on no local network is no address of this node
toward anyone. The show side omits it as `own_addresses::select`
already omits the special-use classes, from the classification the
platform's view carries; nothing else in the decision moves.
Propagation: decision 4; the conformance cases; this note; the log
row; the digest entry; ADR-0052's own note names the specifications.

### Amendment 2026-10-27 — An address the platform's view does not hold is not shown

rust-ui-dev-01's device row of 2026-10-27, on the first build carrying
rule 10 (main bdc7ac89): the Pixel 9a's Show listed both carrier
addresses beside the Wi-Fi one as before. The platform had reported
neither carrier network to the bridge, so the view classified nothing
and the show side, reading the view's classification, omitted
nothing. ADR-0052 rule 10 (A 2026-10-27, its own note) now says what
such an address is — bound on a network the platform reports to no
app, it lies on no local network — and decision 4 inherits it as it
inherited the first case: the show side omits what the runtime holds
back, from the one off-local set. The limit the rule admits is named
here because it is the show side that a person meets: a phone hosting
a hotspot whose access point the platform lets no app read shows no
address for it, and the client's code connects the pair one way.
Gate (e)'s case is the same case, re-run on the build that carries
this. Propagation: decision 4; this note; the log row; the digest
entry; ADR-0052's note names the specifications.

