---
role: "rust-ui-dev"
class: threads
topic: "j228-host-node-for-a17"
description: "j228: a host transport-daemon on develop-qzapp sends the Pixel one text so ux-designer-01's A17 can run; the Pixel's address and the steps"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c81ebe944f0fc5dc
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
- 2026-11-08 12:18Z: "A17" sent, `p1 accepted` (Pixel a private-network address:42057); REPLY 01a1bb17-b5dc to ux. Left: stop daemon + remove profile j228 when ux says A17/A16 done. Step 2 of phone order (j229) waits on p2p-network-dev-01 step 1; owner word for uninstall/clear given (01a1bb11).
- 2026-11-08 13:16Z: step 1 replaced the Pixel's app (test release, TEST identity, new PeerId). Node restarted DETACHED (setsid nohup, log appended to a scratch path); a later session stops it by exact pid (pgrep -u $(id -u) -af 'transport-daemon --profile j228'). Dial+send script: A scratch path <pixel multiaddr>. INFO 01a1bb4d to ux: send new address at 3b, or say A17 done. j228 blocked; next is j229 step 2.
- 2026-11-08 13:16Z ux REPLY 01a1bb4d-edad: A17 ran 12:19-12:28Z on the node; left for 3b: return within 60 s of unlock, kept message after unlock. Keep node up; ux sends the new address at 3b.
- 2026-11-08 16:45Z (j234, REQUEST 01a1bc0b): Pixel debug 3e3c6b902, new identity, address /ip4/a private-network address/tcp/42151/p2p/12D3KooWKPhVLKRhWmzVBNUiSfzmJ15AH4TgyhUPjHSFDdyxRUMP. "A17 two" p2 accepted 16:45:14Z, "A17 one" p3 accepted 16:45:19Z; REPLY 01a1bc0c to ux. Two concurrent `send`s on one profile: the second is refused by the store lock (exit 4) — send one after the other (a scratch path does). Node still up (pid 3149622) until ux says A17/A16 done.
- 2026-11-08 17:37Z: DONE. ux said A17/A16 done (OBSERVATION 01a1bc3c); node stopped by pid, profile j228 and a scratch path removed; findings queued as j235 (REPLY 01a1bc3d).

*References: device-review-run-method, phones-lan-2026-10-30*

*Observed 2026-11-08 (rust-ui-dev)*
