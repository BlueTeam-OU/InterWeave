# endpoints

EndpointId/direct destination/directory grammar and canonical response fixtures.

`endpoint-id-grammar-v1.json` — 18 verdict vectors over `^[a-z][a-z0-9._-]{0,63}$`: the conventional names, both length bounds, and the rejections (leading digit/hyphen/dot, any uppercase, space, slash, colon, non-ASCII, trailing newline).

A verdict set repeats its results by design, so distinctness is off for this algorithm.

`endpoint-directory-v1-frame.json` — 5 framing vectors for `/interweave/endpoints/1.0.0`: the one-byte request, an empty directory, a single entry, the largest legal response (32 entries, 1710 bytes), and a refusal. Byte order is big-endian, pinned in `transport/libp2p/ENDPOINTS.md`. Every entry is a derived EndpointId (ADR-0067), so 52 bytes, and the frames were regenerated when the label grammar left the wire: the 2094-byte frame of 64-byte labels this file froze before is no legal frame now, and 2094 stays the codecs' read bound alone. **The ids are TEST-ONLY**, each the derived id of the key whose seed is `SHA-256("interweave/test-only/endpoint-key/v1" || 0x00 || name)` (`tests/support`'s `endpoint_keys::endpoint_id(name)`), the names in `endpoints_test_keys`, one per entry in the entries' order, as `fixtures/direct-v3/` names its keys; the verifier derives each id from its name and refuses an entry its name does not derive, a duplicate and an unsorted list.

`endpoint-id-derivation-v2.json` — ADR-0067's EndpointId: the libp2p PeerId derivation applied to an endpoint's Ed25519 public key (`00 24 08 01 12 20` followed by the key, canonical base58btc, 52 characters). Six `derive-*` vectors map a public key (four from stated TEST-ONLY seeds, the all-zero seed giving the same text as the `ed25519-bip39-entropy-v1` golden, plus the all-zero and all-0xff keys) to the id, and eleven `verdict-*` vectors judge a text: valid derived ids, a `Qm` sha256 PeerId, a label, wrong lengths, a changed case, a glyph outside the alphabet, another key type and an extra leading byte. The seeds are public test material and never a real key. `endpoint-id-grammar-v1.json` stays for the transitional label path.
