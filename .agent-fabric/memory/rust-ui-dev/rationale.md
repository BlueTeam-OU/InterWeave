---
role: "rust-ui-dev"
class: rationale
description: "The command-line human client's rulings (j41) - app name, one client at a time, keep at read time, listen for scripts"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8607220a14525f3a
  - a3b30d6f54e40cea
---

## where Android's Stay reachable choice lives (architect-cto DECISION 01a124ed, 2026-10-10) and what the shell must do with it

Ruled by architect-cto on 2026-10-10 (DECISION 01a124ed-a425), in answer to my QUESTION 01a124ec-8971.

Where the choice lives:
- Stay reachable is a PERSISTED OVERLAY entry in the profile's state directory, a sibling of the trust overlay.
- config.yaml `runtime.android.availability_mode` is only the authored default. It is never rewritten.
- One write path: an embedded-host API (EmbeddedHost::set_availability, or whatever name is agreed with p2p-network-dev-01).
- profile-config exposes `effective_availability_mode()` = overlay if present, else config. Everything reads that function: the Service's start decision, background_restart_requires_user_authentication, and the diagnostic.

What the shell must do:
- The switch writes only on the person's tap.
- Turning it off REMOVES the entry. It never writes foreground-only.
- The Service reads the effective mode at start, and on a change event from the host. A flip while running restarts the Service in the new mode.

Not ruled: an operator policy forbidding persistent reachability.

**Why:** two sources (a config field and an app preference) would disagree exactly where ADR-0041's explicit opt-in must be provable.

**How to apply:** architect-cto supplies the record text: human-client-android.md, an ADR-0041 amendment, and an ADR-0028 sentence. Until the host API lands, only the debug-only stand-in profile sets the mode, for the ADR-0041 run.

Related: [[embedded-host-seam]] [[android-shell-build]]

*References: android-shell-build, embedded-host-seam*

*Observed 2026-10-10 (rust-ui-dev)*

## The command-line human client's rulings (j41) - app name, one client at a time, keep at read time, listen for scripts

architect-cto-01 ruled on 2026-10-22 (REPLY 01a1626e-bb06-7a14-88e6-255a0aedbc0e, answering the j41 acknowledgement) on the command-line human client. It is the owner's top priority: batch 1 on main by 2026-10-29, with a daily line on where it stands. The request was 01a1626b-e820-7d50-9a12-a6743a290388.

- **Name.** The app is `apps/human-cli`, its binary `interweave-human`, and its branch `develop-qzapp/rust-ui-dev-02/feat/j41-human-cli`.
- **One client at a time.** The CLI holds the same store and endpoint lease as the desktop client (ADR-0041's one exclusive lease, the store's instance lock). The second client is refused by name, with its own exit code.
- **Keep at read time.** A one-shot verb sees what the store holds plus what arrives while it runs, so keep happens at read time: `read --keep <ids>`. A Keep of a message read in an earlier run cannot exist (RETENTION.md), and the README says why.
- **`listen`.** It holds the session open for a script, one JSON line per event.

The batches:
1. What main holds: identities, conversations, status and connectivity, store health, discard and recreate, trust.
2. After #359: the own card, share, proposals, contacts.
3. After p2p-network-dev-02's step 5: peers by address ([[admin-peers-address-methods]]).

Every verb needs a model-level test and one real-daemon run in `tests/desktop-e2e`, and every read verb a `--json` output.

*References: admin-peers-address-methods*

*Observed 2026-10-22 (rust-ui-dev)*
