---
role: "ux-designer"
class: index
description: "What ux-designer knows and where it lives."
tier: 1
distilled_at: "2026-10-24"
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

- [`../agent-fabric/memory/domains/ux-designer/domain/contrast-on-every-surface.md`](../agent-fabric/memory/domains/ux-designer/domain/contrast-on-every-surface.md) — Measure a mark's contrast against every surface it can sit on (snackbar, notice, bubble, selection), not only the screen background
- [`../agent-fabric/memory/domains/ux-designer/domain/focus-table-in-spec.md`](../agent-fabric/memory/domains/ux-designer/domain/focus-table-in-spec.md) — When a spec names focus targets, list every case once in a single focus table in the acceptance section, each with its check, instead of scattering focus rules across sections
- [`../agent-fabric/memory/domains/ux-designer/domain/labels-as-whole-templates.md`](../agent-fabric/memory/domains/ux-designer/domain/labels-as-whole-templates.md) — Propose visible and spoken text as one whole template per case, never a prefix, a joined string or a template with optional parts, so a translation can reorder it
- [`../agent-fabric/memory/domains/ux-designer/domain/late-result-keeps-newer-feedback.md`](../agent-fabric/memory/domains/ux-designer/domain/late-result-keeps-newer-feedback.md) — When an action's feedback waits for an asynchronous answer, a newer action's feedback (and its Undo) is never replaced by the late result
- [`../agent-fabric/memory/domains/ux-designer/domain/refused-action-and-refused-undo.md`](../agent-fabric/memory/domains/ux-designer/domain/refused-action-and-refused-undo.md) — Every action that commits through a fallible store has a refused path, and so does its Undo; a refusal arriving after the visual commit returns the item in the state the backend reports then
- [`../agent-fabric/memory/domains/ux-designer/domain/screen-reader-text-matches-what-is-shown.md`](../agent-fabric/memory/domains/ux-designer/domain/screen-reader-text-matches-what-is-shown.md) — Compose a screen-reader description against the element's label and the screen: never repeat what the label says, speak what a symbol only shows, and say what the sighted person sees
- [`../agent-fabric/memory/domains/ux-designer/domain/thresholds-as-formulas.md`](../agent-fabric/memory/domains/ux-designer/domain/thresholds-as-formulas.md) — Write a compound threshold as an explicit formula (the larger of 40 % of the width and 96 dp) and give every 'that' its antecedent; a builder and a second-language reader both read a parenthesis two ways
- [`../agent-fabric/memory/domains/ux-designer/domain/undo-by-reverse-not-deferral.md`](../agent-fabric/memory/domains/ux-designer/domain/undo-by-reverse-not-deferral.md) — When specifying a snackbar Undo for a destructive action on mobile, run the action at once and undo by the reverse action, rather than deferring it until the snackbar closes
- [`../agent-fabric/memory/domains/ux-designer/domain/undo-restores-identity.md`](../agent-fabric/memory/domains/ux-designer/domain/undo-restores-identity.md) — An Undo that brings back something another system may already have seen restores the same record (its ids and bytes), never a new copy; ask the architecture role what the receiver deduplicates on
- [`../agent-fabric/memory/domains/ux-designer/domain/wording-pass-keeps-conditions.md`](../agent-fabric/memory/domains/ux-designer/domain/wording-pass-keeps-conditions.md) — Read every language/English pass on a spec as a diff for meaning: a reworded if/else can drop a case

## solution

- [`.agent-fabric/memory/ux-designer/solution.md`](.agent-fabric/memory/ux-designer/solution.md) — Which message kinds the InterWeave human client shows and which per-row actions each has — the base for any gesture/swipe design

## workflow

- [`.agent-fabric/memory/ux-designer/workflow/answer-in-spec-before-message.md`](.agent-fabric/memory/ux-designer/workflow/answer-in-spec-before-message.md) — A design answer sent by message before it is written into the spec and reviewed gets corrected later, and the builder builds the wrong version
- [`.agent-fabric/memory/ux-designer/workflow/define-once-point-elsewhere.md`](.agent-fabric/memory/ux-designer/workflow/define-once-point-elsewhere.md) — In a spec, a state condition used in several sections is defined in one place; other sections point to it, never paraphrase it
- [`.agent-fabric/memory/ux-designer/workflow/device-run-check-focus.md`](.agent-fabric/memory/ux-designer/workflow/device-run-check-focus.md) — On the shared Pixel, a scripted device run checks which app has focus before every tap and stops if it is not InterWeave; blind tap sequences can hit system settings
- [`.agent-fabric/memory/ux-designer/workflow/never-blind-inbox-watch.md`](.agent-fabric/memory/ux-designer/workflow/never-blind-inbox-watch.md) — Never start gzcoord-inbox with output to /dev/null or with & disown; it swallows the next delivery
- [`.agent-fabric/memory/ux-designer/workflow/review-fixes-open-new-questions.md`](.agent-fabric/memory/ux-designer/workflow/review-fixes-open-new-questions.md) — In a spec review loop, each fix of a rule (a new case, an invariant) breeds a P2/P3 in the paths it did not touch; grep every path that writes the same state before committing the fix
- [`.agent-fabric/memory/ux-designer/workflow/spec-cites-only-recorded-methods.md`](.agent-fabric/memory/ux-designer/workflow/spec-cites-only-recorded-methods.md) — a design note names an API method or IPC minor as settled only if it is in architecture/contracts on main; otherwise as a dependency with its owner and the acceptance it blocks
- [`.agent-fabric/memory/ux-designer/workflow/workflow-carried-2026-10-18.md`](.agent-fabric/memory/ux-designer/workflow/workflow-carried-2026-10-18.md) — How ux-designer reaches InterWeave's Android test phone (Pixel 9a) for a device review — the lease, pairing, cleanup

## threads

- [`.agent-fabric/memory/ux-designer/threads/android-identity-j27.md`](.agent-fabric/memory/ux-designer/threads/android-identity-j27.md) — j27: Android identity screens design note (create, show phrase, recover) for plan §20 steps 7-8 — where it is and what it waits on
- [`.agent-fabric/memory/ux-designer/threads/android-look-proposal.md`](.agent-fabric/memory/ux-designer/threads/android-look-proposal.md) — Owner asked (2026-10-16) for the Android client to look like an Android app with colour, not a prebeta; Material 3 proposal awaiting owner decisions
- [`.agent-fabric/memory/ux-designer/threads/j29-contact-cards-plan.md`](.agent-fabric/memory/ux-designer/threads/j29-contact-cards-plan.md) — Contact-card UX spec (j29, REQUEST 01a15d2a from architect-cto-01) - written as design/2026-10-21-contact-cards.md, branch docs/contact-cards, PR after #333 merges
- [`.agent-fabric/memory/ux-designer/threads/j38-add-peer-trust-unread.md`](.agent-fabric/memory/ux-designer/threads/j38-add-peer-trust-unread.md) — j38 — add-a-peer note revision on #385 (trust unread on Peers, interim one-line add, Connect lines); state and follow-ups
- [`.agent-fabric/memory/ux-designer/threads/j39-add-peer-unknown-retries.md`](.agent-fabric/memory/ux-designer/threads/j39-add-peer-unknown-retries.md) — j39 state — add-a-peer unknown-retry rule and Peers row contact names, PR #388
- [`.agent-fabric/memory/ux-designer/threads/stage17-android-review-j26.md`](.agent-fabric/memory/ux-designer/threads/stage17-android-review-j26.md) — j26: plan §20 clause (j) — my on-device look-and-feel review of the Android app is a condition of Stage 17's close; status and what it waits on
- [`.agent-fabric/memory/ux-designer/threads/stage21-design-backlog.md`](.agent-fabric/memory/ux-designer/threads/stage21-design-backlog.md) — flows owed to design/ when Stage 21 opens at Stage 17's close — peer by address (ADR-0062) and key-change proposal (ADR-0063)
- [`.agent-fabric/memory/ux-designer/threads/swipe-actions-j1.md`](.agent-fabric/memory/ux-designer/threads/swipe-actions-j1.md) — Status of j1, swipe actions on messages in the Android client — spec merged, build merged; the device review is partial and blocked on the phone link, TalkBack items deferred (IW #297)

## recall

- [`../agent-fabric/identities/roles/ux-designer/recall.md`](../agent-fabric/identities/roles/ux-designer/recall.md) — Where ux-designer's knowledge lives — charter, remit, distilled slices, this agent's memory — and how to trace a claim to its sources.
