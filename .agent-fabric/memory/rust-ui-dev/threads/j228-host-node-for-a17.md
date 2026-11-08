---
role: "rust-ui-dev"
class: threads
topic: "j228-host-node-for-a17"
description: "j228: a host transport-daemon on develop-qzapp sends the Pixel one text so ux-designer-01's A17 can run; the Pixel's address and the steps"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e1c5f7ddf0a9cc95
---

## j228: a host transport-daemon on develop-qzapp sends the Pixel one text so ux-designer-01's A17 can run; the Pixel's address and the steps

REQUEST 01a1b9b9 from ux-designer-01 (2026-11-08); I replied 01a1b9ba that I take it after #590 is armed, in a fresh session.
- The Pixel's address, read by the owner 2026-11-08: /ip4/a private-network address/tcp/39349/p2p/12D3KooWBfgn5EGoNReGStRbVWWxHPeEydjh4mEeceqYp8uVhamy (INFO 01a1b9c5). If the app restarts, the port can move.
- Steps: build a daemon at main with its own profile. Send ux-designer-01 the node's address (its identity show output); the owner adds and trusts it on the Pixel. Then peers add the Pixel, trust allow the Pixel, and wait for ux to say the app is in front. Send "A17" and report when it is accepted.
- The Pixel runs foreground-only with user-presence unlock: the app must be in front, and the owner grants.
Related: [[device-review-run-method]] [[phones-lan-2026-10-30]]
- 2026-11-08 07:18Z (INFO 01a1ba05-9604): the Pixel's address will change at step 2 of the phone order; ux-designer-01 sends the new one at the start of step 3b, and adds and trusts my node on the Pixel by adb (no owner step). Send ux the node's address (peers show) when it runs, and be at hand during 3b (~30 min) to send "A17" when told.
- 2026-11-08 07:37Z: node RUNS. Profile j228 (config ~/.config/interweave/profiles/j228, listen 0.0.0.0:47228), binaries at main 7c2d069c4 in a scratch path, log a scratch path. PeerId 12D3KooWKqPd9SGtBXZYi5uBJxfmcQSkKhbkAJXvMnSLvaqRnosk, address /ip4/a private-network address/tcp/47228/... (VM NAT, Pixel cannot dial in). Sent INFO 01a1ba17 to ux. Next: Pixel's new address -> `interweave-human --profile j228 peers add <addr>`, `trust allow <PeerId>`, then `send <PeerId> A17` when told. After A17: stop the daemon; the profile dirs (~/.config, ~/.local/share, ~/.local/state interweave/profiles/j228) are mine to remove.

*References: device-review-run-method, phones-lan-2026-10-30*

*Observed 2026-11-08 (rust-ui-dev)*
