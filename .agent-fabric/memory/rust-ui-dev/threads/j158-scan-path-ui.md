---
role: "rust-ui-dev"
class: threads
topic: "j158-scan-path-ui"
description: "j158 — scan-path-ui supply delivered 2026-10-30 at e3234d2b (main merged in); read-only driver passes on Pixel + A40; REPLY 01a18b65"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 43f143fdacbb5782
  - d4f3d3bc0febf3ef
---

## j158 — scan-path-ui supply delivered 2026-10-30 at e3234d2b (main merged in); read-only driver passes on Pixel + A40; REPLY 01a18b65

j158 (REQUEST 01a16fef from p2p-network-dev-02), 2026-10-25:
- APK at 8f978966 installed on both phones ~05:05 local; REPLY 01a17099. Cache-on re-run done 12/12 (01a170b2).
- TAB/ENTER drive the app (A40 probe, REPLY 01a170b4). Split agreed (01a1700d); trace rides their PR (01a170b6-f80f); Cargo.lock line cleared (01a170d7).
- Branch develop-qzapp/rust-ui-dev-01/for/p2p-network-dev-02/scan-path-ui in worktree interweave-j158-…, pushed 2026-10-25 (not device-verified): 3b92b8d9 ui-slint trace, 4aba750e no menu line, 53c084fe human-android hook + verifyReleaseNative "ui-trace", 86ff7386 android-e2e src/screen.rs + tests/screens.rs.
- Device lessons: menu opens with no Rust render (no line; wait focus=app-trust). Peers moves focus to its title on open: judge SETTLED focus after a TAB, never the first focus line (Pixel run pressed Add for Show). One render writes page before add lines: waits scan from the press index, any order.
- Trace build APK in a scratch path (installed on A40 and Pixel; built before 4aba750e, still has menu line). Pixel run reached Peers; the fixed driver is not yet run on a device.
- Release mutation DONE: ui-trace forced into release -> verifyReleaseNative "the release native library carries ui-trace" BUILD FAILED; as committed BUILD SUCCESSFUL (logs a scratch path).
- 2026-10-25 ~17:10: rebuilt the trace APK at 86ff7386 in a scratch path (app sha256 d362e471..., androidTest unchanged 1d289d62...; .so carries ui-trace); run-screens.sh points at trace2. Worktree interweave-j158-... now checks out the for/ branch. Probe: A40 a private-network address:36456 refused (port moved), Pixel .54:44599 failed to connect; asked the owner for both addresses / a re-pair.
- Next: owner re-pairs phones -> run tests/screens.rs --ignored; push for/ branch; supply REPLY with case names (launch, open_peers, show, hold_show, close_show, open_add, leave_add, add_by_field, add_by_scan, trust); new APK at their PR head.

- 2026-10-30 DELIVERED: merged origin/main aec72790 into the for/ branch (clean), +3 work: cd3322fb trace names Connect/SupersededUnread, 21dd5917 Back hides the soft keyboard first (main's add page focuses the field; control: without it leave_add fails on the Pixel), e3234d2b mInputShown read as a token anywhere (API 30 prints it mid-line after mShowRequested=). screens.rs passes on Pixel and A40 at e3234d2b. APK in a scratch path (app 265870cc, androidTest 425c686e) on both phones. Supply REPLY 01a18b65. Next only if asked: APK at p2p-network-dev-02's PR head.

- 2026-10-31: released back (01a192e3-8501), landed in my #527 (8 work, 6 fix) merged 817ac44b with connect= trace line + launch wait; told p2p-network-dev-02. Not device-run after the merge.

*Observed 2026-10-31 (rust-ui-dev)*
