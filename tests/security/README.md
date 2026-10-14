# security

Stage 18's network adversarial suite (plan section 21, steps S1-S8):
real peers over real sockets, attacking the limits a profile ships
with. It covers pre-Noise floods, trusted-peer direct floods, GossipSub
poisoning, EndpointId probing, address poisoning of trusted PeerIds,
Kademlia poisoning and eclipse bounds, relay smuggling and AutoNAT
dial-back SSRF.

Only what no lower layer can prove completely goes here. The gate is
[`../SECURITY-MATRIX.md`](../SECURITY-MATRIX.md), which also names the
lower-stage tests guarding each threat. The local half (sockets,
transportctl and the stores) is [`../security-local`](../security-local).
