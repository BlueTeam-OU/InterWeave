---
role: "rust-ui-dev"
class: threads
topic: "owner-finds-ui-ugly"
description: "Owner judged the Android client \"aesthetically very bad\" (2026-10-18) while the pre-j98 build was on the Pixel"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8460f2f0231cf2eb
  - bef64c6b4da5a4ab
---

## Owner judged the Android client "aesthetically very bad" (2026-10-18) while the pre-j98 build was on the Pixel

2026-10-18, after the j104 TalkBack run, the owner said the interface is aesthetically very bad
and hoped it would improve. The build they were looking at was j97's 839e5ac7 (no j98 look:
no avatars, times, last-message lines). Main 400b42b6 carries the android-look spec (#295, #296).

**Why:** The visual verdict is a person's, and the owner's weighs most. A spec that has been
built is not the same as a client that looks good.
**How to apply:** Put main on the phone before anyone judges the look. Turn each specific
complaint into an observable problem and send it to ux-designer-01, whose specs say what the
look should be ([[j98-android-look]]). Don't restyle on my own taste.

2026-10-18 render of main 400b42b6 next to WhatsApp/Telegram conventions (a local side-by-side render, not kept;
scratch example under Xvfb, SLINT_SCALE_FACTOR=2, 412x915, deleted after). Biggest gaps, by size:
PeerId fragments as names and avatars ("…Z8Hh8wh4 / human", "h4"); a row of buttons under every
message ("Show source"; "Send again"/"Cancel" on every pending one); the word "Unread" inside each
received bubble; status words instead of ticks; a yellow "Reconnecting" banner plus the subtitle
"Network status not known yet". Names are a product question; the rest belongs to ux-designer.
Open with the owner (asked 2026-10-18, no answer yet): (a) is the phone free, so main 400b42b6 can
be installed (j98, as DECISION 01a14e7d item 2 says); (b) may the comparison go to ux-designer-01,
and item 1, names, to architect-cto-01 as a product question. Send nothing until they answer.

*References: j98-android-look*

*Observed 2026-10-18 (rust-ui-dev)*
