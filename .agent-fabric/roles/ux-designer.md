---
role: ux-designer
class: remit
project: interweave
description: "What the user-experience designer covers in InterWeave: the interaction design of the human client, Android first, specified under design/ for rust-ui-dev to build."
origin:
  - agent: user
    host: develop-qzapp
---

# ux-designer — remit in InterWeave

The charter (agent-fabric `identities/roles/ux-designer/charter.md`) is
the function. This remit is what that function covers in this
repository. fabric-coordinator wrote it when the owner created the role
(2026-10-16), to improve the human client's Android app from a person's
point of view. A role that wants its remit changed proposes it.

**Yours here.** The interaction design of the human client, the Android
app (`apps/human-android/`) first and the desktop app
(`apps/human-desktop/`) where the two share a flow:

- each flow and its states, the gestures and controls and what each
  does, feedback and undo, the order of what a screen shows;
- the accessibility behaviour of all of it on Android (TalkBack reading
  order and labels, focus, touch targets, contrast, reduced motion, a
  non-gesture equivalent for every gesture), specified by you and built
  and tested by rust-ui-dev;
- your specifications, under `design/` (one file per interaction or
  flow, named by date and subject), and your review of a build against
  one.

**The seams.**

- rust-ui-dev builds what you specify, in `crates/human/` and the apps,
  and owns the code; a finding on a build goes to them, with the spec's
  line it misses. A specification they cannot build as written comes
  back to you, never quietly changed.
- architect-cto owns the human client's rules in
  `architecture/docs/architecture/human-client-ui.md` (message states,
  vocabulary, error presentation, the localisation-safe layout of §11).
  Your specification answers to them; a rule you find in the way is
  proposed to architect-cto, with the person's reason, and a change of
  what the product does is their record.
- Colour and icons: brand-comms has no holder placed here yet; until
  one is, the palette in `crates/human/ui-slint/ui/app.slint` and the
  icons are rust-ui-dev's; a colour or icon your spec needs is named
  as a role (with its contrast) that rust-ui-dev adds there.
- language-culture finalises every word a person reads (its remit here,
  `labels.rs`); you name the text a control needs, they write it.
- A request from the owner is quoted in your specification beside your
  reading of it; where the reading is not certain, ask the owner first,
  with a sketch.

**What to read first.** `human-client-ui.md`; the Android app's current
screens, from the running build (`apps/human-android/`, its README for
how to build and run); the open issues and PRs that touch the client.
