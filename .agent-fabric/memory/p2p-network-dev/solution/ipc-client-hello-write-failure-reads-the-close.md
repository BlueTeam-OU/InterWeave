---
role: "p2p-network-dev"
class: solution
topic: "ipc-client-hello-write-failure-reads-the-close"
description: "ipc-client open(): a failed hello write reads the server's close before answering; test the window through greet() over a socket pair"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 27799aa22703397b
---

## ipc-client open(): a failed hello write reads the server's close before answering; test the window through greet() over a socket pair

ipc-server's accept loop refuses a connection past its slots with close{Overloaded}
and shuts it without reading the hello. So the client's hello write can fail
(EPIPE) after the refusal has already arrived. On AF_UNIX the frame stays readable
after the write fails. Since commit 4af396632 (j142, from rust-ui-dev-01's
observation 01a1a167), `open()` reads what has arrived with `Reader::next_now`
and returns the close's code. Before that it returned BackendUnavailable, which
turned a retryable Overloaded into a terminal-looking error.

You cannot force the connect→write window through `open()` itself. The seam is
`greet(stream, hello, event_queue)` over `UnixStream::pair()`: drop the peer
before greet, so the write fails on every run. Test:
`a_hello_write_failing_after_a_close_keeps_its_code`. Planting the old mapping
makes it fail. Same class as the session path's
`a_failed_write_keeps_the_servers_close_code`. The read is `next_now`, never a
waited `next()`. A server that shut its read half and stays silent would
otherwise hold `open()` until Timeout. Test: `a_failed_hello_write_does_not_wait`.
The waited-read mutation fails it.

*Anchors: `crates/local/ipc-client/src/connection.rs::greet`, `crates/local/ipc-client/src/connection.rs::a_failed_hello_write_does_not_wait`*

*Observed 2026-11-03 (p2p-network-dev)*
