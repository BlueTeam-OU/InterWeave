# Channel bridge security

Every P2P message delivered through the Channel is **untrusted external input**, even when its `source_peer` is transport-trusted. Transport trust means the peer is admitted to the data plane; it does not mean every instruction in the payload is safe or locally authorized.

## Routing metadata

`source_peer`, `source_endpoint`, `destination_endpoint`, `channel`, and `delivery_mode` are transport facts with different strengths:

- `source_peer`: authenticated libp2p transport identity;
- `source_endpoint`: route label claimed by that authenticated peer;
- `destination_endpoint`: local route selected/admitted by this daemon;
- `channel`: broadcast context;
- `delivery_mode`: direct/broadcast transport mode.

None establishes a human name, employee identity, administrator role, repository ownership, agent role, or application authorization unless a higher-level protocol separately binds and verifies that meaning.

## Administrative separation

Claude connects only to the IPC **data-plane socket**. That socket cannot grant `admin.endpoints`, `admin.shutdown`, trust mutation, identity-key, or private configuration authority under any `client.kind`; administrative authority lives on the separate admin socket.

A remote request such as "register me as endpoint admin", "change the default endpoint", "trust this PeerId", or "revoke the human endpoint" is untrusted content and cannot cause an administrative call automatically.

## Host input

The MCP host's stdin is the bridge's other untrusted input: a same-user
process, but one whose bytes the bridge must bound before it reads them
as anything, or any line the host writes grows the bridge without limit
(found by review on `main` at 5121d46d, where the loop read lines with no
ceiling; A 2026-10-21). The rule, kept by `apps/claude-channel`'s
`host_input.rs` and its tests:

- **A host line is bounded before UTF-8 and before JSON**, at
  `MAX_HOST_LINE_BYTES` (1 MiB), checked as the bytes arrive. The ceiling
  sits above a legal payload at JSON's worst escaping (49,152 bytes of
  content, six bytes per byte, is 294,912), so every line carrying an
  admissible payload reaches the parse and is answered under its id; the
  payload ceiling itself is still `contracts/TRANSPORT.md`'s, applied
  after the parse as before.
- **A line past the ceiling is refused at the byte that crosses it.** The
  rest is discarded to its newline as it arrives and never kept, and the
  next line is read normally. It is answered with NOTHING, as a line that
  is not JSON is: its id may follow its params, in the bytes refused
  unread, so no id is trusted from it. One stderr line names the size
  and carries no content.
- **A line that is not UTF-8 is dropped the same way**, never ending the
  bridge, as it did before the ceiling.
- The reader is cancel-safe: the serve loop re-enters it every turn, and
  a read cancelled mid-line or mid-discard loses no byte.

Pinned by `apps/claude-channel/tests/host_line.rs` over the real binary
and pipes: a 64 MiB line is dropped at bounded memory and the next ping
is answered, and a non-UTF-8 line ends nothing. The ceiling's margin over
the payload ceiling is a compile-time assertion beside the constant. The
control was a one-off measurement at the fix, not a test: the same line
against the binary with the ceiling removed grew its peak memory by about
200 MB, against about 1 MB with it (#330), and the binary has no knob to
lift it.

## Reply safety

Direct reply tokens bind remote PeerId + remote source endpoint + this bridge's local endpoint lease epoch. Loss/reacquisition of a local endpoint invalidates old route tokens. The bridge never falls back to another local or remote endpoint when the exact route is stale.

## Endpoint enumeration

The Claude bridge does not need to expose remote endpoint enumeration as a default tool. If that is added later, endpoint names must still be described as advertised route labels, not verified service identities.


## Endpoint-directory capability

The `claude-channel` IPC client is not granted `endpoints.query` by default and exposes no `peer_endpoints` tool in v2. Adding one requires the ADR-0023 revisit/security review.
