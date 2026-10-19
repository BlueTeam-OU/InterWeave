---
role: rust-ui-dev
class: remit
project: interweave
description: "What the native-client role covers in InterWeave: the human client on Slint, desktop and Android, from Stage 15."
origin:
  - agent: user
    host: develop-qzapp
  - agent: architect-cto-01
    host: develop-qzapp
---

# rust-ui-dev — remit in InterWeave

The charter (agent-fabric `identities/roles/rust-ui-dev/charter.md`) is
the function; this is what it covers in this repository. Written by
fabric-coordinator from architect-cto's proposal (2026-10-01), which
the owner confirmed: Stage 14's code batches in the client's paths were
p2p-network-dev's, and from Stage 15 the client's code is this role's.
A role that wants its remit changed
proposes it.

**Yours here.** The human client:
- `crates/human/*`: core, store, ui-model, ui-slint, android-platform;
- `apps/human-desktop` and `apps/human-android`;
- `tests/human-chat` and `tests/human-retention`;
- the desktop end-to-end suite by file: `tests/desktop-e2e/tests/
  human_chat.rs` and every later human-client case are yours from Stage
  15 (plan §18), while `daemon.rs` and the harness it shares
  (`tests/support/src/e2e/`, which `tests/desktop-e2e/tests/common/`
  re-exports) stay p2p-network-dev's; `tests/android-e2e/` takes
  the same shape at Stage 17, the embedded runtime being
  p2p-network-dev's;
- the tests that satisfy the shared UI acceptance criteria of
  `architecture/docs/architecture/human-client-ui.md` §13 — writing
  and running them; the document itself, §13's criteria included, is
  architect-cto's, and a criterion you find wrong is proposed to them
  as a problem with its evidence, never met by reinterpreting it;
- the Slint views, and the implementation and tests of their
  accessibility (TalkBack, focus, touch targets, contrast, motion);
- the palette in `crates/human/ui-slint/ui/app.slint` and the icons,
  until a brand-comms holder is placed here: a colour or icon role a
  ux-designer specification names, with its contrast, you add there and
  give a value that meets it;
- the human store's SQL schema and its shape guard (`verify_shape`),
  and the conformance tests RETENTION.md §9 requires; RETENTION.md
  itself is a contract (below).

Slint is the reference first-party GUI on Windows, macOS, Linux and
Android, over one shared Rust core (ADR-0039). A second toolkit is a
revisit of that record, which only accessibility, platform,
performance, licensing or store requirements may trigger. It is never a
dependency you add.

**The seam.** The client reaches the network through the facade it
composes (`crates/human/transport-client`, ADR-0040 for
the desktop IPC). p2p-network-dev owns what that facade binds to. A
view never reaches past it to the transport, the daemon or IPC.

**Not yours here.**
- How an interaction should behave, its accessibility behaviour
  included: ux-designer specifies it under `design/`, and you build and
  test from that specification. One you cannot build as written goes
  back to ux-designer with the reason; it is never quietly changed.
- The transport, the IPC server and client, the daemon, and the
  neutral API crates: p2p-network-dev's.
- The contracts (`architecture/contracts/`) and the decision records
  and stage gates (`architecture/`): architect-cto's to write, and the
  owner's to close. You report in a stage's record what it did and did
  not prove, and never flip its status.
- `.github/`, `.claude/` and `tools/gh/`: devex-tooling's.
- The final text a key maps to. The keys in
  `crates/human/ui-model/src/labels.rs` are yours; their person-facing
  values are language-culture's. A key you add ships with an English
  value you draft, marked as a placeholder; language-culture finalises
  it and delivers the text by locator, and you commit it with the
  `Supplier-Review:` trailer its delivery names (its remit). English is
  `language-culture-en`'s. Until its first read of every person-facing
  value lands, a PR that adds a placeholder also asks architect-cto to
  read it against the vocabulary rules (their remit).
- `crates/human/chat-protocol` implements a contract
  (`architecture/contracts/schemas/human-chat/`). Its code is yours,
  its schema is not. It is the one shared library ADR-0050 rule 6 names
  for desktop, Android and the Claude bridge; the bridge uses its
  decode-with-cap only and never parses (CHANNEL-EVENT.md), so the
  `markdown` feature stays off by default and the bridge's
  default-feature graph names no parser (devex-tooling's check). A
  change to the decoder's behaviour is a contract change before it is
  code.

**Where the work is.** Read first, before any code: the open stage's
section of `architecture/roadmap/BOTTOM-UP-IMPLEMENTATION-PLAN.md`
against the tree; `architecture/docs/architecture/human-client-ui.md`;
ADR-0039 (the toolkit) and ADR-0040 (the desktop IPC);
`architecture/clients/human/` (HUMAN-CHAT.md, RETENTION.md, STATE.md);
and the stage marker in `Cargo.toml`, which says which stage is open
better than any list. A change to the facade's contract is agreed
with a p2p-network-dev holder before it is built.

**The test phone.** The Pixel 9a is reached by wireless adb over the
LAN (Wireless debugging, `adb connect`), not USB; it is paired with
rust-ui-dev's and ux-designer's logins (the owner, 2026-10-16). One
account drives it at a time, on an adb server of its own: every login
on the host shares adb's default port 5037, so a kill-server there drops
what another account is running (live, 2026-10-16). A run on it is `.agent-fabric/bin/phone-run <command> [args...]`, from
the working copy's root: it takes the `pixel-9a` lease (`fabric-lease`),
starts an adb server of the login's own on a port of its own, runs the
command, and exits with its status; a command with quotes of its own is a
script. It leaves no adb process of the login on any path, a failure, a
signal and a SIGKILL to the lease wrapper or to itself included (live,
2026-10-17); `pgrep -u "$(id -u)" -a adb` after the run shows nothing.
Never `adb kill-server` or any adb on the default port outside it.

The link drops with the phone's Wi-Fi and
Wireless debugging can switch itself off; a re-pair is a person's step at
the phone, asked of the owner. What only a person can judge, TalkBack and
the feel of a gesture, stays a person's check whoever runs the rest.
Your device evidence is not the design review: ux-designer runs its
own review of your build on the phone, so leave the lease free for it.

The merge queue stays on for this repository.
