# human-chat-v2

HumanChatV2 valid/invalid canonical application messages, including markdown-subset cases and decode-direction compression vectors with the mandatory cap-abort case (ADR-0050).

`human-chat-v2-envelope.json` — 23 verdict vectors over the envelope shape and grammar: version, kind, the 32-lowercase-hex ID forms, `reply_to` including an unresolvable one, the timestamp interval bounds, `from_endpoint`, an ignored unknown field (the schema is OPEN by design), and `text` required and possibly empty.

`human-chat-v2-brotli-decode.json` — 3 decode-direction compression vectors (ADR-0050, plan §17 batch 3):
- an envelope whose brotli stream decodes to its frozen raw form;
- exactly the 196,608-byte ceiling, which decodes;
- the mandatory cap-abort case, one byte past the ceiling, which a receiver must abort mid-stream.

The encode direction is not fixture-testable: brotli output is non-canonical. `verify_fixture_vectors.py` recomputes each result independently, and `tests/human-chat` runs them through the shared decoder.

`human-chat-v2-contact-card.json` — 18 verdict vectors over a received `contact_card` envelope (HUMAN-CHAT.md §Contact card, ADR-0059, ADR-0063), each with the connection it arrived on (`received_on`: the PeerId and source endpoint, which the canonical bytes take from there and never from the envelope), its `valid` verdict and the one `rule` a malformed vector breaks:
- valid, 4: a card at the bounds (name and surname at 64 scalar values after NFC, a 256 × 256 PNG of exactly 24,576 bytes carrying no metadata chunk); a name-only card; a name sent decomposed that verifies after NFC; identifier-like members inside `card`, ignored;
- malformed, 14: signed for another PeerId; the name changed after signing; a 31-byte key; a small-order key whose signature a cofactorless verifier accepts; the key unpadded and URL-safe; `text` and `reply_to` present; a name past the bound; a photo of 24,577 bytes and one of a third media type; whitespace and the URL-safe alphabet in `photo.data`; an unknown kind.

Every card but the small-order one is signed by one key pair whose private half is in the file: **a TEST-ONLY key, published, that no build may hold as an identity key**. Its `fingerprint_hex` (ADR-0063 decision 1) is on the valid card at the bounds. `verify_fixture_vectors.py` recomputes each verdict with strict verification, the receiver's `canonical_bytes_hex` where the card reaches the signature, and each test-key signature from the seed over the bytes `signed_over` names, so a malformed vector fails for its rule alone. `tests/human-chat` holds the valid cards to the envelope schema; the parser's verdicts on these vectors are plan §29 step 1's.

The markdown subset is deliberately NOT a validity verdict: out-of-subset markdown falls back to plain-text display rather than rejecting the envelope, so subset conformance is a rendering contract for `tests/human-chat`.
