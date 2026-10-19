---
role: "architect-cto"
class: rationale
description: "Android accessibility (j141): the owner's 2026-10-18 decision — TalkBack carried to the release gate, the AccessKit bridge built after everything else — and the facts the next step rests on"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a77428b94014de26
---

## Android accessibility (j141): the owner's 2026-10-18 decision — TalkBack carried to the release gate, the AccessKit bridge built after everything else — and the facts the next step rests on

Decided 2026-10-18 by the owner, in session ("talkback is a fringe feature
that we can leave after everything else"): TalkBack on Android is carried to
the owner's release gate under plan §20 gate (f); the bridge is built after
the rest of Stage 17. Record: plan §20 "Decided 2026-10-18" subsection, IW
#297 (f0610d02). DECISIONs sent to rust-ui-dev-01 (01a14e7d-f392) and
ux-designer-01 (01a14e7d-fc8a).

Facts the build will rest on when asked for:
- Not an ADR-0039 revisit: the gap is one backend's missing adapter.
- rust-ui-dev-01's prototype 839e5ac7 (scratch branch, nothing landed):
  app-side adapter on accesskit_android 0.7.5 over i-slint-core's doc-hidden
  tree API feeds the platform tree and follows focus within 50 ms; ~440
  lines; gaps: no clickable, no row description, no scroll action.
- Explore-by-touch needs a one-line backend patch (HoverMove Unhandled);
  Slint Royalty-free 2.0 permits a modified copy, so it lands vendored in
  the ADR-0051/0053/0054 shape with an ADR of mine.
- Gestures cannot be injected over adb; the finger run was voided by the
  "Allow TalkBack to make and manage phone calls?" dialog. They are the
  build's acceptance tests, by a person, with that dialog answered first.
- Phone practicalities: adb pairing is per login; one adb server per host
  (port 5037), so never start one from an unpaired account while another
  login drives the device.

**Why:** the decision is deferral, not refusal; a later session must not
re-open the measurement or treat the gap as unknown.
**How to apply:** when the owner asks for the bridge, write the vendored
backend ADR first and REQUEST rust-ui-dev with it; §20's closing record
cites the subsection for the carry.

*Observed 2026-10-18 (architect-cto)*
