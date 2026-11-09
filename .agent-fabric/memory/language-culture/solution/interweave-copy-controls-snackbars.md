---
role: "language-culture"
class: solution
topic: "interweave-copy-controls-snackbars"
description: "InterWeave labels.rs: how copy names controls, retries, Undo snackbars, dialogs, paths and screen-reader places"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 664315035ffc95ca
---

## InterWeave labels.rs: how copy names controls, retries, Undo snackbars, dialogs, paths and screen-reader places

InterWeave human-client copy (crates/human/ui-model/src/labels.rs): controls, snackbars and placement.
- A control whose name is a verb phrase ("Paste an image") takes no trailing "again" after "choose <Control>": it reads as part of the name (ClipboardNoImage, j151, 8c3ec410, 2026-10-24).
- A retry line under a button names that button: "Choose Make a new key to try again", never a bare "Try again" unless the button itself is Try again (KeyNewFailed/KeyNewFailedGone, #411, f1ff931b; AddUnknown "Choose Add to try again").
- A path or other exact value in a sentence goes last, after a label ("Folder: {path}"), with no punctuation touching it: a path may hold commas, spaces and full stops (KeyDirUnsafe, #411, fe01b70c, 2026-10-24).
- A label before a value must be true for every thing the value can name: "Folder: {path}" called a file a folder when a file stood in its place; "Folder location: {path}" holds for a link, another account's folder and a file (KeyDirUnsafePath, #411, 37d09f5a, 2026-10-24).
- A control label that ends in "…" ("Open an image…") stays inside a sentence, never at its end: at the end it reads as trailing off, and a full stop after it gives "image…." (ClipboardUnreadable, #413, c3815c98, 2026-10-24).
- Screen-reader-only text names a place by a landmark every reader can find (the composer: "just above the box where you type"), not by "bottom": heard, "bottom" can mean the newest message. Never "message" for a snackbar in this app (CardUndoHint, #416, 40b970e4, 2026-10-24).
- An Undo snackbar is the button's verb in the past tense, with no object beyond the button's ("Remove photo" → "Photo removed", "Delete contact" → "Contact deleted"). The context is the page it appears on (#433, confirmed 2026-10-25).
- An unsaved-changes dialog uses Close, not Discard, Delete or Leave: "Close without saving your changes?" / "Continue editing" / "Close without saving" (#453, j203, 2026-10-26). The app shows no Discard. Delete was read as deleting the card (Delete contact is on the contact page). Leave was unclear when an identity switch opens the dialog.
- A way to a control is named by the controls' own labels, from labels.rs: "choose App options on the Conversations screen, then Contacts, then Try again". Never a description that matches no label ("InterWeave's menu"), and never "Chats", which is only the screen's name in the specs. Advice to use another conversation says "if you have one" when one may not exist for the identity in front. Review of #491 found both errors in my own texts (j229, 100412f4, 2026-10-27).
- A control never starts with "Read": read is a message-status word, and a cold read takes "Read the whole message" as a read receipt. The control is now "Open the whole message", which matches "Opening the whole message". rust-ui-dev-02 renamed the key to OpenWholeMessage (4177c66a3, j248, 2026-11-06).
Related: [[solution-carried-2026-10-10]]

*References: solution-carried-2026-10-10*

*Anchors: `crates/human/ui-model/src/labels.rs::MessageNotCopied`*

*Observed 2026-11-06 (language-culture)*
