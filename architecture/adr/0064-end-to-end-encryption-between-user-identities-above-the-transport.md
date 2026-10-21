# End-to-end encryption between user identities, above the transport: pairwise sessions with forward secrecy, sender keys for channels, one identity on several nodes, and ciphertext a forwarder may hold

**Status:** Proposed — the owner's direction of 2026-10-21 ("A matter is peer level encryption and another should be the end2end encryption"; "create also adr proposed state for the full implementation"). Nothing inherits from this record until the owner accepts it; the one piece taken now is ADR-0063, the identity key and its place in the card.

## Context

The transport encrypts links between nodes and authenticates PeerIds (Noise, `transport/libp2p/SECURITY.md`); a direct message is confidential against a relay and read at the node; a broadcast is read by every trusted forwarder; group encryption is deferred by that document. ADR-0061 made a user identity an EndpointId under the node, ADR-0063 gave each identity a signing key carried in its card. What the design still cannot give: a message confidential against the node that forwards it, a channel confidential against the trusted peers that relay it, the same identity live on two nodes (ADR-0043 forbids cloning the PeerId and offers no other way), and a message held for an offline recipient by anyone (ADR-0020 refuses an offline store, rightly, while the holder could read it). These four are one layer.

## Decision

1. **A second layer, above the transport, between identity keys.** The transport is unchanged: Noise, trust on the PeerId, routing by EndpointId, the 48 KiB payload ceiling. Inside a HumanChatV2 envelope a new kind, `sealed`, carries ciphertext and the protocol's header; the receiving identity's client decrypts it into the `text` or `contact_card` it wraps, which then follows every existing rule. A node that cannot decrypt a `sealed` kind holds opaque bytes and nothing else.

2. **Pairwise sessions with forward secrecy.** Between two identities the protocol is an authenticated initial agreement from ADR-0063's identity keys and signed prekeys (X25519 keys the identity publishes, signed by its identity key; the prekey bundle travels in the card and in a `prekeys` kind on request), followed by a double ratchet: a new message key per message, a DH step per round trip, so a compromised key reveals neither past nor future messages. The construction is a maintained library on the licence allow-list or one its ADR argues for (ADR-0038's rule: no bespoke construction); the choice is SPIKE-013's, measured on the Stage 17 reference device.

3. **Channels under sender keys.** A broadcast channel's members each hold a sender key, distributed pairwise under decision 2 to each member identity; a channel message is encrypted under the sender's key and signed by the identity key, so a trusted forwarder carries ciphertext. Membership stays what it is (ADR-0029: GossipSub validation, the trust set as membership); the sender-key distribution adds the identity-level roster the channel never had, and a member leaving means a new sender key for everyone else.

4. **One identity on several nodes.** An identity's key may live on more than one node (a phone and a desktop): the identity publishes a **device list** — its PeerIds, signed by the identity key — in its card and on change, and a counterparty's client sends to every listed node, each with its own session. ADR-0043's rule stands: nodes never share a PeerId; what they share is the identity key, moved between the person's own devices by the device-to-device transfer of ADR-0060 decision 8, over InterWeave itself, never through a provider.

5. **Ciphertext a forwarder may hold.** With the payload confidential against its holder, a trusted peer or a relay the person chose may keep a `sealed` message for an offline recipient identity, bounded in count, bytes and time, deleted on delivery or expiry, never readable: the one offline store ADR-0020 and ADR-0044 can admit, because the store cannot read what it holds. It is opt-in per identity and per holder, the holder is named in the identity's device list, and the transport's non-durable promise stays true for everything that is not `sealed` and chosen.

6. **Verification.** A person compares two identities' full keys through a QR code or a number string on a verification screen; a verified identity is marked, and a key change on a verified identity is a louder event than on an unverified one.

7. **Recovery of identity keys.** An identity's key is restored only from the person's own backup: the device-to-device transfer, or an identity export the person makes (a sealed file under a passphrase, ADR-0038's envelope). A lost key is a new identity key and a key change at every counterparty; the record says so rather than inventing a recovery that weakens the key.

8. **Every action has a command-line form**, the fleet's agents included: an automated client (ADR-0055) may hold an identity key and speak sealed, so that the control plane's messages are confidential against the nodes that carry them.

## Alternatives considered

- **Encrypt at the transport with per-identity Noise keys.** Rejected: Noise authenticates the node; identities per connection would multiply connections and still read at the node.
- **A static identity-to-identity box (no ratchet).** Rejected: no forward secrecy; the compromise of one key reads everything ever sent.
- **MLS for every conversation.** Considered for channels; decided with SPIKE-013, where the library landscape and the phone's cost are measured.
- **Store-and-forward on the transport without this layer.** Rejected, as ADR-0020 rejected it: a holder that can read is a server.

## Consequences

If accepted: HUMAN-CHAT.md gains the `sealed` and `prekeys` kinds and the card's prekey bundle and device list; a new contract for the protocol's header and the sender-key messages; STATE.md gains sessions, prekeys, sender keys and device lists, all in the encrypted store; ADR-0043 is amended (one identity key on several PeerIds); ADR-0020 and ADR-0044 are amended for the opaque, chosen, bounded holder; ADR-0055 gains the agent's identity key; a stage of its own in the plan after Stage 22, with SPIKE-013 as its prerequisite. Until then nothing moves, and ADR-0063's key is the only prerequisite that ships early.

## Security implications

What the layer gives: confidentiality against the node, the relay, the forwarder and the holder; forward secrecy and post-compromise security per session; a verifiable identity independent of the node. What it does not give: metadata — who talks to whom, when, how much — stays visible to the nodes on the path and to the relay, as the transport's trust model already states; the node still holds the plaintext after decryption, in the encrypted store at rest; a compromised device reads its own identities. Every key is under the custody of ADR-0042 and ADR-0038. The protocol is not designed here: a maintained construction is selected and its security properties are cited, never derived.

## Operational implications

A new stage of work across the shared crates and the three clients, with the phone's cost measured first; the relay operator gains a bounded store they cannot read; an operator of an automated client gains a key file per agent.

## Implementation implications

SPIKE-013 first: the library, its licence, its size and cost on the reference device, the ratchet state's size in the store, a channel of 32 members under sender keys. Then the stage, in the order of the decisions. The fleet's carrier (ADR-0055) is the first deployment that would use decision 8.

## Revisit conditions

The owner's acceptance; SPIKE-013's result; a change in the transport's trust model; a store-and-forward need that cannot wait for this layer.
