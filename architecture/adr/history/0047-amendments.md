# ADR-0047 — amendment history

### Amendment 2026-08-17 — HumanChat media type advanced to v=2

ADR-0050 superseded the HumanChatV1 envelope before any implementation existed, making `application/vnd.interweave-human-chat+json;v=2` (and its `;ce=br` compressed form) the implementation target. The canonical application/local identifier list read `application/vnd.interweave-human-chat+json;v=1`; a reader taking that list as current would implement the wrong version, so the entry is updated in place.

This amendment changes nothing this ADR decided: the `vnd.interweave-` vendor prefix, the display/machine namespace split, and every hash-participating identifier are untouched — the version parameter belongs to the HumanChat application protocol, and no frozen golden involves the HumanChat media type as anything but arbitrary sample bytes (the `fixtures/direct-v2/` vector that uses the v1 string as test input stays byte-identical for exactly that reason).

### Amendment 2026-11-19 — The direct protocol id is `/interweave/direct/3.0.0` from the Stage 22 release

ADR-0067 decision 7 moves the direct protocol's version, not its namespace. The canonical network identifier list read `/interweave/direct/2.0.0`; it now reads `/interweave/direct/3.0.0`. Nothing this record decided changes: the `interweave` namespace, the domain-separation prefixes and the four re-frozen goldens are untouched, since the protocol id takes part in no hash. Propagation: the list; this note; the log row; the digest entry.

### Amendment 2026-11-22 — The endpoint directory protocol id is `/interweave/endpoints/2.0.0` from the Stage 22 release

ADR-0067 decision 7 (A 2026-11-22) moves the endpoint directory protocol's version, not its namespace. The canonical network identifier list read `/interweave/endpoints/1.0.0`; it now reads `/interweave/endpoints/2.0.0`. Nothing this record decided changes: the `interweave` namespace, the domain-separation prefixes and the four re-frozen goldens are untouched, since the protocol id takes part in no hash. Propagation: the list; this note; the log row; the digest entry.
