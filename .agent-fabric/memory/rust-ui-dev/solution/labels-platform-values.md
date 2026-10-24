---
role: "rust-ui-dev"
class: solution
topic: "labels-platform-values"
description: "decided key shape for the eight labels.rs values naming the transport daemon (Android has none) — implement on the j51 app-shell branch"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 82feae324277834c
---

## decided key shape for the eight labels.rs values naming the transport daemon (Android has none) — implement on the j51 app-shell branch

Decided 2026-10-10 (j49), sent to language-culture-en as relay 01a12495-13a2 in reply to their OBSERVATION 01a12491-1bb6.

The eight values: UiText NoDaemon, TrustUnavailable, TrustNotPermitted, TrustRefused, TrustIncompatible, TrustUnconfirmed; ErrorClass::TransportUnavailable; Connectivity::Offline (crates/human/ui-model/src/labels.rs, placeholder_en).

Shape: platform values. One key per situation; the table takes where the runtime lives (separate daemon = desktop, embedded in the app's service = Android), chosen once at each app's composition root. Views never branch; an exhaustive match forces an Android text. Desktop values unchanged.
**Why:** the remedy differs (desktop: start a separate program named "transport daemon", the glossary keeps it desktop-only; Android: the app runs its own service). A neutral word would weaken the desktop text.

**How to apply (on the j51 branch):**
- Android placeholders use "the network service" (their proposal), marked PLACEHOLDER (en).
- Send language-culture-en each key with its situation, the person's state and the remedy, once the Activity/Service lifecycle is agreed with p2p-network-dev-01 (§20 step 2).
- TrustNotPermitted and TrustIncompatible are expected unreachable on Android (one APK; the settings path holds LocalAdminPort). This is not verified. If confirmed, map them to TrustFailed's text and ask for no words.

Related: [[labels-english-supplier]] [[embedded-host-seam]]

*References: embedded-host-seam, labels-english-supplier*

*Observed 2026-10-10 (rust-ui-dev)*
