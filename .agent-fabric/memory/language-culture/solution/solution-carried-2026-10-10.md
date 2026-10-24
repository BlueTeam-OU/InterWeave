---
role: "language-culture"
class: solution
topic: "solution-carried-2026-10-10"
description: "The English conventions and glossary the InterWeave human client's copy holds (labels.rs), set at the first read 2026-10-09"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2bc5099f332af2c0
  - 86d0192f0ef3cc97
  - b83c70900eb93ad9
---

## The English conventions and glossary the InterWeave human client's copy holds (labels.rs), set at the first read 2026-10-09

InterWeave human-client copy (crates/human/ui-model/src/labels.rs), as set
by the first English read (delivered to rust-ui-dev-01, REQUEST
01a12001-199d-75d9-8e8d-cceb01ea9bbf, 2026-10-09):
- en-US spelling ("Canceled"), the remit's en-US supply rule.
- Labels, buttons, headings: sentence case, no final period. A complete
  sentence ends with a period; every ErrorClass text is a full sentence,
  because it fills "Not sent: {reason}" and "Could not open the session: {reason}".
- Glossary: "transport daemon" (local component, never "local transport"),
  "app" (this client), "list of trusted peers", "Keep" (human-client-ui.md §6's
  verb), "may have reached the peer" (never "received": §5 keeps it for inbound),
  "might" for possibility (never "may not": it reads as permission).
- labels.rs tests bind the words: no delivery label contains
  read/seen/processed/delivered (substring); all UiTexts distinct, so two
  actions cannot share one text (Retry is "Send again", TryAgain "Try again");
  Offline must say "offline", Unknown must not.
- A new value drafted by its caller carries a `PLACEHOLDER (en)` line comment (line end, or the line above when it wraps), defined in labels.rs module doc on #242 (rust-ui-dev-01, 2026-10-09); grep for it to find work; the commit taking final text removes it.
- Error texts are ONE sentence: #242 review F3 caught a two-sentence Busy text of mine; count sentences after filling the templates.
Related: [[cold-read-without-meanings]]
- Snackbar texts name what just happened and get their own UiText keys, even where a status label has the same words: "Kept", "Hidden", "No longer kept", "Sending canceled" (never "Send canceled": "send" as a noun reads foreign), "Action undone" after any Undo (set 2026-10-16, design/2026-10-16-message-swipe.md §9, a22531bf).
- design/ specs: the prose is British like the rest of the repository, and the quoted UI strings are en-US. language-culture reviews every design/ spec's English before it merges (remit, 26524c45).
- labels.rs keys for the swipe snackbar (delivered to rust-ui-dev-01 for #289, 2026-10-16, REPLY 01a144ec-6faa): Hide, HideConversation, ConfirmKept/Hidden/Unkept/ConversationHidden, Undo final as drafted; Restored -> "Action undone"; two new cancel snackbar keys asked for, not reuse of the status labels. Owner ruled 2026-10-16: a MESSAGE action is "Dismiss" / snackbar "Dismissed" (it never comes back); a CONVERSATION keeps "Hide" / "Conversation hidden" (spec 2ff5dc96).
- One noun for one thing: an item in the list is a "conversation"; the list title stays "Conversations", never "Chats" ("Chats" is only the spec prose name of the screen). The composer field and its placeholder share `Composer` => "Message". A "You: " preview is a template "You: {message}", never a prefix (Android look spec ad4cd40b, 2026-10-16).
- Dates and times: the owner decided day before month on 2026-10-21 (design/2026-10-16-android-look.md 826fafd4): list row "Sat 17 Oct" / "Fri 17 Oct 2025" (short weekday and month, 3 letters, no period), day separator and row accessible label "Saturday, 17 October 2026" (DateFull). This replaces the en-US "{month} {day}, {year}" of 1614b5ce (2026-10-17). "{hour}:{minute}" (24h, 2-digit hour); AM and PM as two whole templates; never a numeric date.
- A file-dialog filter name names the type, noun first, formats in parentheses: CodeImageFilter "Images (PNG or JPEG)" (j150, REPLY 01a16d0f-a77e, 2026-10-24). No "*.png" patterns in the name: some dialog toolkits append them, and then they show twice; that choice is the caller's, asked, not assumed.
- A control whose name is a verb phrase ("Paste an image") takes no trailing "again" after "choose <Control>": it reads as part of the name (ClipboardNoImage, j151, 8c3ec410, 2026-10-24).

*References: cold-read-without-meanings*

*Observed 2026-10-24 (language-culture)*
