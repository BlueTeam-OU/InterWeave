# endpoints

EndpointId/direct destination/directory grammar and canonical response fixtures.

`endpoint-id-grammar-v1.json` — 18 verdict vectors over `^[a-z][a-z0-9._-]{0,63}$`: the conventional names, both length bounds, and the rejections (leading digit/hyphen/dot, any uppercase, space, slash, colon, non-ASCII, trailing newline).

A verdict set repeats its results by design, so distinctness is off for this algorithm.

`endpoint-directory-v1-frame.json` — 5 framing vectors for `/interweave/endpoints/1.0.0`: the one-byte request, an empty directory, a single entry, the 2094-byte ceiling (32 entries at 64 bytes each), and a refusal. Byte order is big-endian, pinned in `transport/libp2p/ENDPOINTS.md`.

`endpoint-id-derivation-v2.json` — ADR-0067's EndpointId: the libp2p PeerId derivation applied to an endpoint's Ed25519 public key (`00 24 08 01 12 20` followed by the key, canonical base58btc, 52 characters). Six `derive-*` vectors map a public key (four from stated TEST-ONLY seeds, the all-zero seed giving the same text as the `ed25519-bip39-entropy-v1` golden, plus the all-zero and all-0xff keys) to the id, and eleven `verdict-*` vectors judge a text: valid derived ids, a `Qm` sha256 PeerId, a label, wrong lengths, a changed case, a glyph outside the alphabet, another key type and an extra leading byte. The seeds are public test material and never a real key. `endpoint-id-grammar-v1.json` stays for the transitional label path.
