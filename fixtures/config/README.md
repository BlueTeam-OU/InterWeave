# config

Frozen valid/invalid schema-v2 configuration fixtures.

`config-v2-cross-field.json` — 16 verdict vectors over the endpoint cross-field rules: entries naming their key file as `<EndpointId>.key` (ADR-0067), unique IDs and aliases, a default (an alias or an id) naming an enabled endpoint, static-subset peers within profile trust, narrowing-not-widening (ADR-0012), and advertised entries within `directory.max_advertised`. Regenerated under ADR-0067 decision 7: each label became the alias of an entry whose id is a TEST-ONLY derived id (tests/support's `endpoint_keys`, a public seed), and the unknown-default vector names an id no entry has, since an unknown alias is refused when the document is read.

These are relationships BETWEEN fields, which is what JSON Schema cannot express — the gap these vectors cover beside `contracts/schemas/`.
