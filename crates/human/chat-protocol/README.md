# chat-protocol

HumanChatV2 parsing/serialization/validation and application fixtures: the envelope codec, the contact card's parser, the bounded brotli decode path, the send-side encoder, and -- behind the `markdown` feature -- the render model (ADR-0050, plan §17 (3); the card ADR-0059 and ADR-0063, plan §29 step 1).

**Current status:** active workspace member since Stage 2; the encoder and the render model are Stage 14's, the contact card Stage 21's. Above transport and independent of it — no libp2p, no IPC, no UI.

## One library, three consumers

The desktop client, the Android client, and the Claude bridge all decode through this crate. That is what makes "decompression happens once, above transport" true rather than aspirational: the daemon never decompresses, and there is exactly one implementation of the ceiling.

## The cap aborts mid-stream

Measured brotli expansion on hostile input exceeds 87,000×, so 48 KiB of payload can name gigabytes of output. `decode_envelope_bytes` decompresses **incrementally** and stops the moment the 196,608-byte ceiling is passed — peak memory is bounded by the limit plus one 4 KiB chunk, whatever the stream claims to contain. Decompressing first and measuring after would have already allocated what the attacker asked for; the check would be a report, not a bound. A test feeds it 4 MiB of zeros compressed to under 4 KB.

There is deliberately no declared-length field to consult: a declared length is peer-asserted metadata the cap must override anyway, so honouring it would add an input to trust and no safety.

`encode_outbound` is the send side, and `sender_may_compress` its rule: an envelope that fits goes out raw, and only one over the payload limit is compressed. That bounds the *other* end too. The legal range is `max_payload_bytes < raw <= 196,608`; above the ceiling a message is too large **before** compression is considered, because a repetitive 300 KB document that compresses under the payload limit would be sender-conforming and refused by every conforming receiver.

## The render model is a feature, off by default

**The markdown subset is a rendering contract, not a validity verdict.** An out-of-subset construct, or input past a bound, falls back to plain-text display of the source rather than rejecting the message. `render` (feature `markdown`) pins the dialect: `pulldown-cmark` with CommonMark plus GFM tables and strikethrough, nothing else. The result is a block tree a client renders, with these rules:
- raw HTML is literal text;
- a link outside `https`/`mailto` is its text, inert;
- an image is a placeholder, never fetched;
- block nesting past 16 levels, inline nesting (emphasis, strong, strikethrough, links, images) past 16 counted apart, a table past 256 body rows or 32 columns, or input past the decoded ceiling gives the source as plain text.

The feature is off by default because the Claude bridge decodes and never parses (`CHANNEL-EVENT.md`); `check_bridge_default_features.sh` holds the bridge's default graph to no parser. The policy primitives, `is_allowed_link_scheme` and the bounds, stay outside the feature, so every consumer applies one rule.

`is_allowed_link_scheme` is an **allowlist**. A denylist has to anticipate every dangerous scheme and is wrong the moment a new one exists; an allowlist is wrong only about schemes that are safe, which costs a working link rather than an execution. A relative reference is not activatable either — there is no base to resolve it against in a chat message, and guessing one would invent a destination the sender never wrote.

## Envelope details

- **`text` is required and may be empty.** Absence and emptiness are different; only the first is malformed.
- **The schema is OPEN and this crate keeps it that way** — no `deny_unknown_fields`. It is the one place in this repository where openness is the specified behaviour rather than the fallback, because closing it would break the property the version number exists to provide.
- **An unresolvable `reply_to` stays valid** and triggers no lookup. Treating it as a fetchable pointer would turn a display hint into a remote-triggered request.

`tests/frozen_envelopes.rs` runs all 23 vectors from `fixtures/human-chat-v2/`, whose verdicts are recomputed independently by the Python verifier.

## The contact card

`classify` gives every HumanChatV2 text, received on a direct route, the three-way answer its consumers need (plan §29 step 1): `NotCard`, `Card`, or `MalformedCard` with one fixed `CardFault`, which carries no peer text. A text is a card when **any** of its top-level `kind` copies is `contact_card`, read apart from the strict parse; so a second `kind` cannot hide a card from a consumer that drops a malformed card but forwards what is not a card: it is `MalformedCard(DuplicateMember)`, never `NotCard`. `HumanChatV2::parse` still answers for messages only and refuses a card as `UnsupportedKind`.

A `Card` has verified under its identity key, strictly (`ed25519-dalek`'s `verify_strict`: a small-order key or R is refused, and a non-canonical S while the crate's `legacy_compatibility` is off in the graph), over `canonical_card_bytes` built from the route the card arrived on, never from what the envelope says: so a card signed for one PeerId or endpoint fails on another. Names are bounded and signed after NFC. The photo is base64-decoded and measured (`Card::photo_len`), never decoded as an image: a client does that after the person accepts the card.

`tests/frozen_cards.rs` holds `classify` to all 20 vectors of `human-chat-v2-contact-card.json` by name, compares `canonical_card_bytes` with the verifier's bytes, and guards the built cases: the small-order-key and small-order-R cards verify under the crate's non-strict `verify` and fail only `verify_strict`, and the S-plus-order card is the valid signature with S raised by the group order, still below 2^253, so it fails whenever `legacy_compatibility` is turned on.
