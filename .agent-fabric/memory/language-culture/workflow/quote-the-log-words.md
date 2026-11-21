---
role: "language-culture"
class: workflow
topic: "quote-the-log-words"
description: "when spec prose paraphrases a log line, quote the line's own words in code font, so its noun is not read as a second name"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 27889c8841b3fae1
---

## when spec prose paraphrases a log line, quote the line's own words in code font, so its noun is not read as a second name

When spec prose paraphrases what a program logged, quote the log's own words in code font. Do not paraphrase them into the prose. On InterWeave #663 (j313, 2026-11-13), human-client-android.md step 5 said the Service "logged that it would give one notice and then end". "notice" then read as a second noun for the notification. NetworkService.kt:385 logs "... one notice, then end", so the text now says "logged a line with the words `one notice, then end`". In the same reply, HUMAN-CHAT.md's Clusters note used "character" for a cluster, while the same paragraph used "characters" for code points. A run record's bare "drew" also hid the U+FFFD replacement.

**Why:** in a quoted log line, a reader takes each noun as the product's own name for a thing.

**How to apply:** before wording a sentence about a log, a record or a trace, look up the exact string in the code. Quote the string, or name the thing by its glossary noun. See [[new-string-same-text-as-another]] and [[observed-apart-from-expected]].

*References: new-string-same-text-as-another, observed-apart-from-expected*

*Anchors: `apps/human-android/android/app/src/main/kotlin/org/interweave/human/NetworkService.kt::one notice`*

*Observed 2026-11-13 (language-culture)*
