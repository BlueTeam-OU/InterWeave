---
role: "language-culture"
class: solution
topic: "interweave-spec-prose-wording"
description: "InterWeave design/ specs: English conventions for the prose (dates, stale references, test steps, acceptance items, defined terms)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0cf3c82fae54a703
  - 69b7bf5976a078d5
---

## InterWeave design/ specs: English conventions for the prose (dates, stale references, test steps, acceptance items, defined terms)

InterWeave design/ specs: the prose's English, as set in language-culture's passes on design PRs.
- A date in project prose is the UTC date, as the commits carry it: my local clock read 2026-10-25 while the commits said 2026-10-24 UTC, and the s9 lead had to be corrected (#425, fb808f0f).
- When a design PR drops a term from a section ("This is a dependency" in add-a-peer s5.7, #432), grep the file for references to it ("§5.7's dependency"). They become stale outside the sections the caller named. Fix them in the same supplier commit, and say so in the REPLY (j170, cde996fa).
- A state that began in the past and still holds is "has been on `main` since #N", never "is on `main` since" (j170).
- On a second pass over a caller's fix, diff it against the fold of my last commit: a rewrite can drop my earlier fixes ("and have been on main since") or a qualifier ("refused `superseded`" lost its word, which widened the claim). Restore the wording and flag the qualifier to the author (#432, 697993f2 → 5e1f80a6).
- An appositive after a subject ("The app's side, this paragraph and X, has been ...") reads as a list of three things. Use "which ... describe" (#432).
- "refusals that prove no other change" reads as "prove that nothing changed". Write "that do not prove another change was made". In the same way, "nothing may have changed" reads as permission; write "it is possible that nothing changed" (#432, 9233a1f7).
- In an acceptance item, state the rule first and give the test setup in a sentence of its own. A setup in brackets inside the rule's sentence hid what is dragged and which Accept the Undo belongs to (#432, 6c4894f3).
- When a wording pass rewrites a test's steps, replay the state after each step. My "The test is the same, and then the person chooses ... the Undo" (6c4894f3) could not produce the withheld drag it tested, because the first test's drag starts without the warning. The author caught it (#432, 3cbdd6e2).
- A withheld swipe moves the pointer, not the row: a test "starts a right drag on the row" and "moves the pointer past the threshold"; never "drags the row" (#432, 64391450).
- "once" closing a clause before a comma ("press X at the row once, open Y") reads as the conjunction "once [you] open"; write "a single time" (#513, 2026-10-30). In a spec that uses "page" for screens and for loads of rows, a load is "a page of older rows".
- In a spec that defines "held" as "holds and shows", say a hidden row "stays in memory", never "is held", in the same sentence that counts it as not held (#518, 2026-10-31).
- A time window in a test step is "between <event noun> and <event noun>" ("between the clear of the log at the start of this step and R4's continue"). "after the log was cleared at the start of this step and before R4's continue" lets the second bound bind to the first clause's verb (#633, 3860f1db0, 2026-11-12).
- One verb for two acts in one step reads as one act: "a failed clear of the marker" next to "the log was cleared". Name the marker's act by what the code does ("fails to remove the marker"), and keep "clear" for the log (#633, 3860f1db0).
- A participle after a quoted text attaches to the quote: "a line containing `X` logged after ..." -> "a line that contains `X` and was logged ..." (#633, 3860f1db0).
- "the identity note's marker" was ambiguous: design/2026-10-19-android-identity.md has two markers, the void-paper marker and the connect-owed marker. Name the marker every time (add-a-peer §5.1, j292, c0e25c5de, 2026-11-12).
- A test step that holds a read back for N s is checked against the clock its rule counts from. In add-a-peer item 12, "held back 0.5 s, nothing shows" sits next to a 1 s that counts from the press and includes the service start. Ask the author about it; do not reword it (j292).
- A test step that times something from an event (a grant, a seal's success) needs a trace line for that event. Check that item 12's dependency names one. If it does not, ask the author; do not add one (j297, 5819762fe: the grant).
- A sighting cited by a time of day ("01:49 phone time") needs its date and its record file. Take the date from the record (the ux-designer thread file), and ask the author when the run crossed midnight (#659, 81ffb562b, 2026-11-13).
- "the prompt cancelled leaves the screen X" reads as "goes away from X". Write "When the prompt is cancelled, X stays" (#659, 81ffb562b).
- "left up" for a system prompt is an idiom. Write "stays on screen for N s and is then granted" (j297).
Related: [[solution-carried-2026-10-10]] [[that-is-and-only-once]]

*References: solution-carried-2026-10-10, that-is-and-only-once*

*Anchors: `design/2026-10-19-android-identity.md::fails to remove the marker`*

*Observed 2026-11-13 (language-culture)*
