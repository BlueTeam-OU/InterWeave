---
role: "ux-designer"
class: index
description: "What ux-designer knows and where it lives."
tier: 1
distilled_at: "2026-10-31"
---

# ux-designer — knowledge index

A session is given the charter and brief (in the launch prompt),
and the project's remit with a pointer to this index (from the
session-start hook) — nothing below. Open a slice when its cue
matches what you are doing; a `workflow` slice says how a kind of
work is done here, so read the matching ones before that work.
Paths are relative to this working copy; `../agent-fabric/` is the
control plane checked out beside it.

## charter

- [`../agent-fabric/identities/roles/ux-designer/charter.md`](../agent-fabric/identities/roles/ux-designer/charter.md) — The fleet's user-experience designer: how a person meets, understands and operates what a project ships — flows, interactions, feedback, accessibility — specified for the developer who builds it and checked against the person who uses it.

## brief

- [`../agent-fabric/identities/roles/ux-designer/brief.md`](../agent-fabric/identities/roles/ux-designer/brief.md) — How ux-designer works day to day, in any project: design starts from the person's goal, is specified completely enough to build without guessing, rests on named evidence, and is verified in the running build against its own acceptance.

## domain

- [`../agent-fabric/memory/domains/ux-designer/domain/announce-once-undo-findable.md`](../agent-fabric/memory/domains/ux-designer/domain/announce-once-undo-findable.md) — One act is announced once: when focus moves to an element that states the result, no live announcement repeats it; then the Undo the announcement no longer names gets a hint placed by reading order, not by a visual landmark
- [`../agent-fabric/memory/domains/ux-designer/domain/awaited-answer-states.md`](../agent-fabric/memory/domains/ux-designer/domain/awaited-answer-states.md) — While an action waits for its answer: a waiting label that claims no outcome takes the button's place with focus, a progress mark that stays under reduced motion, and Back still closes the page, leaving the outcome to the next read
- [`../agent-fabric/memory/domains/ux-designer/domain/bounded-loss-is-a-choice.md`](../agent-fabric/memory/domains/ux-designer/domain/bounded-loss-is-a-choice.md) — when a memory bound must drop something the person cannot get back, no drop order fixes it; make crossing the bound the person's informed choice
- [`../agent-fabric/memory/domains/ux-designer/domain/case-tables-keyed-by-outcome.md`](../agent-fabric/memory/domains/ux-designer/domain/case-tables-keyed-by-outcome.md) — A spec's case table is keyed by the outcome observed, one row each, and its catch-all is checked both ways: no combination unnamed, no case another section requires swallowed
- [`../agent-fabric/memory/domains/ux-designer/domain/contrast-on-every-surface.md`](../agent-fabric/memory/domains/ux-designer/domain/contrast-on-every-surface.md) — Measure a mark's contrast against every surface it can sit on (snackbar, notice, bubble, selection), not only the screen background
- [`../agent-fabric/memory/domains/ux-designer/domain/declined-prompt-stays-declined.md`](../agent-fabric/memory/domains/ux-designer/domain/declined-prompt-stays-declined.md) — A system prompt the person cancelled is not raised again by itself on a return to the app; the blocked state shows why and keeps the action that raises it in view
- [`../agent-fabric/memory/domains/ux-designer/domain/every-count-variant.md`](../agent-fabric/memory/domains/ux-designer/domain/every-count-variant.md) — An element designed for many items (a notice above a list, a confirmation naming the item) is walked again with one item and with none: the container may not exist there, and the words it names may be barred in that mode
- [`../agent-fabric/memory/domains/ux-designer/domain/focus-table-in-spec.md`](../agent-fabric/memory/domains/ux-designer/domain/focus-table-in-spec.md) — When a spec names focus targets, list every case once in a single focus table in the acceptance section, each with its check, instead of scattering focus rules across sections
- [`../agent-fabric/memory/domains/ux-designer/domain/focus-target-not-drawn.md`](../agent-fabric/memory/domains/ux-designer/domain/focus-target-not-drawn.md) — Every focus rule has a fallback for a target that is not drawn when focus arrives (removed elsewhere, the list unreadable, the controls replaced): next row by last-drawn place, else the previous, else a fixed control; announce only what…
- [`../agent-fabric/memory/domains/ux-designer/domain/labels-as-whole-templates.md`](../agent-fabric/memory/domains/ux-designer/domain/labels-as-whole-templates.md) — Propose visible and spoken text as one whole template per case, never a prefix, a joined string or a template with optional parts, so a translation can reorder it
- [`../agent-fabric/memory/domains/ux-designer/domain/late-result-keeps-newer-feedback.md`](../agent-fabric/memory/domains/ux-designer/domain/late-result-keeps-newer-feedback.md) — When an action's feedback waits for an asynchronous answer, a newer action's feedback (and its Undo) is never replaced by the late result
- [`../agent-fabric/memory/domains/ux-designer/domain/picker-whole-word-prefixes.md`](../agent-fabric/memory/domains/ux-designer/domain/picker-whole-word-prefixes.md) — A narrowing word picker (type letters, pick from matches) must offer a typed whole word that is also a prefix of others, or such words can never be entered
- [`../agent-fabric/memory/domains/ux-designer/domain/refused-action-and-refused-undo.md`](../agent-fabric/memory/domains/ux-designer/domain/refused-action-and-refused-undo.md) — Every action that commits through a fallible store has a refused path, and so does its Undo; a refusal arriving after the visual commit returns the item in the state the backend reports then
- [`../agent-fabric/memory/domains/ux-designer/domain/refused-write-read-back.md`](../agent-fabric/memory/domains/ux-designer/domain/refused-write-read-back.md) — A refused write may already have happened: before a screen reports failure (or tells the person to discard something), it reads the stored state back
- [`../agent-fabric/memory/domains/ux-designer/domain/relative-day-rules-defined-once.md`](../agent-fabric/memory/domains/ux-designer/domain/relative-day-rules-defined-once.md) — A relative-date rule (Today, Yesterday, weekday) is defined once with its unit (calendar days, the device's time zone) and its future case; other sections refer to it, and every reference is true for every value
- [`../agent-fabric/memory/domains/ux-designer/domain/release-not-drop.md`](../agent-fabric/memory/domains/ux-designer/domain/release-not-drop.md) — a memory ceiling over rows that keep a stored copy is a silent release with load-on-reach gaps, never the limit row
- [`../agent-fabric/memory/domains/ux-designer/domain/screen-reader-text-matches-what-is-shown.md`](../agent-fabric/memory/domains/ux-designer/domain/screen-reader-text-matches-what-is-shown.md) — Compose a screen-reader description against the element's label and the screen: never repeat what the label says, speak what a symbol only shows, and say what the sighted person sees
- [`../agent-fabric/memory/domains/ux-designer/domain/strings-claim-only-what-is-true.md`](../agent-fabric/memory/domains/ux-designer/domain/strings-claim-only-what-is-true.md) — When specifying what a string must communicate, also write what it must not claim: no step the platform lacks, no scope wider than the action, no promise that will not come by itself
- [`../agent-fabric/memory/domains/ux-designer/domain/sweep-before-adopting-a-key.md`](../agent-fabric/memory/domains/ux-designer/domain/sweep-before-adopting-a-key.md) — Before specifying a text-matching rule (confusables, case folding, normalisation), sweep the whole data table; a fix proposed from a few examples hides gaps
- [`../agent-fabric/memory/domains/ux-designer/domain/thresholds-as-formulas.md`](../agent-fabric/memory/domains/ux-designer/domain/thresholds-as-formulas.md) — Write a compound threshold as an explicit formula (the larger of 40 % of the width and 96 dp) and give every 'that' its antecedent; a builder and a second-language reader both read a parenthesis two ways
- [`../agent-fabric/memory/domains/ux-designer/domain/undo-by-reverse-not-deferral.md`](../agent-fabric/memory/domains/ux-designer/domain/undo-by-reverse-not-deferral.md) — When specifying a snackbar Undo for a destructive action on mobile, run the action at once and undo by the reverse action, rather than deferring it until the snackbar closes
- [`../agent-fabric/memory/domains/ux-designer/domain/undo-restores-identity.md`](../agent-fabric/memory/domains/ux-designer/domain/undo-restores-identity.md) — An Undo that brings back something another system may already have seen restores the same record (its ids and bytes), never a new copy; ask the architecture role what the receiver deduplicates on
- [`../agent-fabric/memory/domains/ux-designer/domain/unknown-is-not-a-default.md`](../agent-fabric/memory/domains/ux-designer/domain/unknown-is-not-a-default.md) — A fact the client could not read (a failed read, a lost answer, a refused read) is shown as unknown with a line saying why, never as its default value; a retry that writes nothing keeps it unknown until a read shows it
- [`../agent-fabric/memory/domains/ux-designer/domain/wording-pass-keeps-conditions.md`](../agent-fabric/memory/domains/ux-designer/domain/wording-pass-keeps-conditions.md) — Read every language/English pass on a spec as a diff for meaning: a reworded if/else can drop a case

## solution

- [`.agent-fabric/memory/ux-designer/solution/solution-carried-2026-10-24.md`](.agent-fabric/memory/ux-designer/solution/solution-carried-2026-10-24.md) — Which message kinds the InterWeave human client shows and which per-row actions each has — the base for any gesture/swipe design

## workflow

- [`.agent-fabric/memory/ux-designer/workflow/answer-in-spec-before-message.md`](.agent-fabric/memory/ux-designer/workflow/answer-in-spec-before-message.md) — A design answer sent by message before it is written into the spec and reviewed gets corrected later, and the builder builds the wrong version
- [`.agent-fabric/memory/ux-designer/workflow/define-once-point-elsewhere.md`](.agent-fabric/memory/ux-designer/workflow/define-once-point-elsewhere.md) — In a spec, a state condition used in several sections is defined in one place; other sections point to it, never paraphrase it
- [`.agent-fabric/memory/ux-designer/workflow/device-run-check-focus.md`](.agent-fabric/memory/ux-designer/workflow/device-run-check-focus.md) — On the shared Pixel, a scripted device run checks which app has focus before every tap and stops if it is not InterWeave; blind tap sequences can hit system settings
- [`.agent-fabric/memory/ux-designer/workflow/never-blind-inbox-watch.md`](.agent-fabric/memory/ux-designer/workflow/never-blind-inbox-watch.md) — Never start gzcoord-inbox with output to /dev/null or with & disown; it swallows the next delivery
- [`.agent-fabric/memory/ux-designer/workflow/no-scope-widening-in-review-fixes.md`](.agent-fabric/memory/ux-designer/workflow/no-scope-widening-in-review-fixes.md) — during a PR's review rounds, fix only what a finding names; never settle a reviewer's "risk" by adding a new rule
- [`.agent-fabric/memory/ux-designer/workflow/read-builder-branch-before-open.md`](.agent-fabric/memory/ux-designer/workflow/read-builder-branch-before-open.md) — Before a spec says a figure is unmeasured or a question open, read the builder's branch for a record of it
- [`.agent-fabric/memory/ux-designer/workflow/review-fixes-open-new-questions.md`](.agent-fabric/memory/ux-designer/workflow/review-fixes-open-new-questions.md) — In a spec review loop, each fix of a rule (a new case, an invariant) breeds a P2/P3 in the paths it did not touch; grep every path that writes the same state before committing the fix
- [`.agent-fabric/memory/ux-designer/workflow/review-placement-against-field-list.md`](.agent-fabric/memory/ux-designer/workflow/review-placement-against-field-list.md) — In a device review, check each element's placement against the spec's field list before passing a row, not only its behaviour
- [`.agent-fabric/memory/ux-designer/workflow/review-rounds-mechanics.md`](.agent-fabric/memory/ux-designer/workflow/review-rounds-mechanics.md) — how to chain fabric-review rounds and post a subagent's report whole without retyping it
- [`.agent-fabric/memory/ux-designer/workflow/spec-cites-only-recorded-methods.md`](.agent-fabric/memory/ux-designer/workflow/spec-cites-only-recorded-methods.md) — a design note names an API method or IPC minor as settled only if it is in architecture/contracts on main; otherwise as a dependency with its owner and the acceptance it blocks
- [`.agent-fabric/memory/ux-designer/workflow/workflow-carried-2026-10-18.md`](.agent-fabric/memory/ux-designer/workflow/workflow-carried-2026-10-18.md) — How ux-designer reaches InterWeave's Android test phone (Pixel 9a) for a device review — the lease, pairing, cleanup

## threads

- [`.agent-fabric/memory/ux-designer/threads/android-identity-j27.md`](.agent-fabric/memory/ux-designer/threads/android-identity-j27.md) — j27: Android identity screens design note (create, show phrase, recover) for plan §20 steps 7-8 — where it is and what it waits on
- [`.agent-fabric/memory/ux-designer/threads/android-look-proposal.md`](.agent-fabric/memory/ux-designer/threads/android-look-proposal.md) — Owner asked (2026-10-16) for the Android client to look like an Android app with colour, not a prebeta; Material 3 proposal awaiting owner decisions
- [`.agent-fabric/memory/ux-designer/threads/b4c-reply-target.md`](.agent-fabric/memory/ux-designer/threads/b4c-reply-target.md) — B4c reply line for a target not held — ReplyPresent if reachable (held/released/stored), ReplyUnavailable only if neither held nor stored
- [`.agent-fabric/memory/ux-designer/threads/gzapp-no-remit.md`](.agent-fabric/memory/ux-designer/threads/gzapp-no-remit.md) — gzapp asks reach ux-designer-01, but the role has no gzapp remit or working copy; answer provisional and ask fabric-coordinator
- [`.agent-fabric/memory/ux-designer/threads/j29-contact-cards-plan.md`](.agent-fabric/memory/ux-designer/threads/j29-contact-cards-plan.md) — Contact-card UX spec (j29, REQUEST 01a15d2a from architect-cto-01) - written as design/2026-10-21-contact-cards.md, branch docs/contact-cards, PR after #333 merges
- [`.agent-fabric/memory/ux-designer/threads/j38-add-peer-trust-unread.md`](.agent-fabric/memory/ux-designer/threads/j38-add-peer-trust-unread.md) — j38 — add-a-peer note revision on #385 (trust unread on Peers, interim one-line add, Connect lines); state and follow-ups
- [`.agent-fabric/memory/ux-designer/threads/j39-add-peer-unknown-retries.md`](.agent-fabric/memory/ux-designer/threads/j39-add-peer-unknown-retries.md) — j39 state — add-a-peer unknown-retry rule and Peers row contact names, PR #388
- [`.agent-fabric/memory/ux-designer/threads/j51-identity-key-unopenable.md`](.agent-fabric/memory/ux-designer/threads/j51-identity-key-unopenable.md) — j51 state — design note for an identity key refused at open and the new-key action, PR #411
- [`.agent-fabric/memory/ux-designer/threads/j52-clipboard-unreadable.md`](.agent-fabric/memory/ux-designer/threads/j52-clipboard-unreadable.md) — j52 state — add-a-peer s5.5 ClipboardUnreadable line for an unreadable clipboard, PR #413
- [`.agent-fabric/memory/ux-designer/threads/j53-card-accept-announce.md`](.agent-fabric/memory/ux-designer/threads/j53-card-accept-announce.md) — j53 state — contact-cards CardPhotoNotUsed final text and Accept announced once, PR #416
- [`.agent-fabric/memory/ux-designer/threads/j54-address-superseded.md`](.agent-fabric/memory/ux-designer/threads/j54-address-superseded.md) — j54 state — add-a-peer s5.7 superseded refusal (IPC 2.10) of an address change or step-2 Trust, PR #425
- [`.agent-fabric/memory/ux-designer/threads/j55-proposal-actions.md`](.agent-fabric/memory/ux-designer/threads/j55-proposal-actions.md) — contact-cards proposal (s5.4/s5.4.4/s8): actions stacked at every scale; Accept swipe withheld beyond case D asked of the owner (O2)
- [`.agent-fabric/memory/ux-designer/threads/j58-proposal-menu-carried.md`](.agent-fabric/memory/ux-designer/threads/j58-proposal-menu-carried.md) — contact-cards proposal menu and photo removal (#435, merged 829dbccc); three items carried to the next contact-cards spec PR
- [`.agent-fabric/memory/ux-designer/threads/j60-same-name-keys.md`](.agent-fabric/memory/ux-designer/threads/j60-same-name-keys.md) — contact-cards 5.4.1 same-name rule became two keys (case key OR shape key); PR #438 with #435's carried items
- [`.agent-fabric/memory/ux-designer/threads/j61-card-updates-photo-removed.md`](.agent-fabric/memory/ux-designer/threads/j61-card-updates-photo-removed.md) — j61 CardUpdatesPhotoRemoved final English in the contact-cards strings table; #441, armed on architect-cto's word
- [`.agent-fabric/memory/ux-designer/threads/j62-case-e-no-photo.md`](.agent-fabric/memory/ux-designer/threads/j62-case-e-no-photo.md) — j62/j63 contact cards: CardKeepsNoPhoto after Remove photo, case S without CardSame; the editor photo step's focus and speech; PR #443
- [`.agent-fabric/memory/ux-designer/threads/j64-photo-subsampling.md`](.agent-fabric/memory/ux-designer/threads/j64-photo-subsampling.md) — j64 contact cards 5.2: chroma subsampling is the encoder's (image crate 0.25.10 writes 4:4:4); figures labelled; PR #447
- [`.agent-fabric/memory/ux-designer/threads/j65-photo-step-device-review.md`](.agent-fabric/memory/ux-designer/threads/j65-photo-step-device-review.md) — j65 contact cards 5.2: citation fix afe99571; Pixel review of #449's photo picker waits on rust-ui-dev-01's install (data kept)
- [`.agent-fabric/memory/ux-designer/threads/j71-silent-key-pending.md`](.agent-fabric/memory/ux-designer/threads/j71-silent-key-pending.md) — j71 identity-key note — silent first key also needs no pending own-card row; commit c9d361a5 awaiting rust-ui-dev-02's fold
- [`.agent-fabric/memory/ux-designer/threads/j73-own-card-row.md`](.agent-fabric/memory/ux-designer/threads/j73-own-card-row.md) — j73 own card row (contact-cards 5.5) delivered as #469 (merged f2295901); carried risks listed in a PR comment
- [`.agent-fabric/memory/ux-designer/threads/j84-b4-connect-span.md`](.agent-fabric/memory/ux-designer/threads/j84-b4-connect-span.md) — j84 PR #513 — Connect closes the conversation (start-window s7.1), window marks inside Accept's span (contact-cards s5.4.2)
- [`.agent-fabric/memory/ux-designer/threads/stage17-android-review-j26.md`](.agent-fabric/memory/ux-designer/threads/stage17-android-review-j26.md) — j26: plan §20 clause (j) — my on-device look-and-feel review of the Android app is a condition of Stage 17's close; status and what it waits on
- [`.agent-fabric/memory/ux-designer/threads/stage21-design-backlog.md`](.agent-fabric/memory/ux-designer/threads/stage21-design-backlog.md) — flows owed to design/ when Stage 21 opens at Stage 17's close — peer by address (ADR-0062) and key-change proposal (ADR-0063)
- [`.agent-fabric/memory/ux-designer/threads/swipe-actions-j1.md`](.agent-fabric/memory/ux-designer/threads/swipe-actions-j1.md) — Status of j1, swipe actions on messages in the Android client — spec merged, build merged; the device review is partial and blocked on the phone link, TalkBack items deferred (IW #297)

## recall

- [`../agent-fabric/identities/roles/ux-designer/recall.md`](../agent-fabric/identities/roles/ux-designer/recall.md) — Where ux-designer's knowledge lives — charter, remit, distilled slices, this agent's memory — and how to trace a claim to its sources.
