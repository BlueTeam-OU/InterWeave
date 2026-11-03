---
role: "rust-ui-dev"
class: threads
topic: "j175-contacts-collation"
description: "InterWeave j175 (P3-3): contacts ordered by UTS #10 collation (icu_collator) at every site with the route tie; branch, the fold, what architect-cto-01 was told"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 742890d53437621e
---

## InterWeave j175 (P3-3): contacts ordered by UTS #10 collation (icu_collator) at every site with the route tie; branch, the fold, what architect-cto-01 was told

REQUEST 01a17e8a from architect-cto-01 (2026-10-27). Full text: `gzcoord-inbox --replay 624320`.
- The rule is on human-client-ui.md §3 "How contacts are ordered", at commit 5d4d3393 on develop-qzapp/architect-cto-01/for/rust-ui-dev-01/contacts-order (from main 1b37e0ef). Fold it unrebased.
- Branch from fabric-jobs: develop-qzapp/rust-ui-dev-01/feat/j175-p3-3-contacts-ordered-by-unicode (worktree interweave-j175-p3-3-contacts-ordered-by-unicode).
  - My REPLY 01a17e8c named it feat/j175-contacts-collation, which is WRONG. Give the real name in the PR REPLY.
- Build:
  1. icu_collator, workspace-pinned (Unicode-3.0 is already allowed; report what cargo deny says).
  2. One ui-model function that orders by display name under the CLDR root, then the smallest route (PeerId canonical form, then route label, by code point).
  3. Four sites use it: ui-slint cards.rs render_contacts (~l.1442); ui-model cards.rs contact_names_on (~l.1327); UiModel::contacts() over the store's ORDER BY display_name; the CLI's contacts. Also the standing queries SHARES_NODE, SAME_NAME, KEY_MATCHES outputs.
  4. Update the comment at ui-model cards.rs:1315-1317 and the "by name" doc of contacts().
- Tests: "ben", "Ben", "Chloe", "Åke", "Zoe" in the root order; a same-name tie broken by the smaller PeerId; the four sites agreeing on one fixture; widen ui-slint tests/cards.rs:1821 past Ada-before-Bob. Name the mutations.
- PR body: APK and Linux archive size deltas by build; prefer root data only.
- Paging: contact_directory_page has no caller. Agree the page order with rust-ui-dev-02 before batch B pages contacts.
- Also waiting: j174 (Connect late-answer announcement) after #491 merges.
Related: [[j155-connect-screen]]

**Progress 2026-10-27 (worktree interweave-j175-…):** fold 0a169c51; commits c04b4bf1 ui-model (order.rs: RootOnly provider, root data only, refuses tailoring+reordering markers; contacts_listed sorts; notes sorted in proposals()), ea7f1477 ui-slint, f88db1a3 store (ORDER BY contact_id), 76e621dd human-cli test. icu_collator/provider/normalizer pinned "~2.3" (plain "2.3" bumps every icu_* to 2.4). cargo deny clean, no deny.toml change. Scratch size test: full data +1.25 MB, root-only +0.3 MB. Paging QUESTION to rust-ui-dev-02 seq 624921 (ids in model, store serves rows by id). Size builds: scratchpad/size/measure.sh base|head.

**PR #494 (2026-10-27):** head 9c5a9305; 5 work + 4 review-fix (F1-F4 P3, all fixed, re-review clean, both posted). Boundary (Cargo.toml, store contacts.rs), under 8 → arming REQUEST to architect-cto-01 seq 626224 (01a17ed6) for AUTHORIZES #494@9c5a9305; arm with --authorization <id>; never push onto it after. Sizes in PR body: archive +711,753 gz, APK +280,080, stripped CLI +344,640 vs full-data control +1,240,896. Paging agreed with rust-ui-dev-02 (REPLY 01a17ea5): keep contact_id keyset, model sorts the whole. add-a-peer §5.2/§11 interim order text is ux-designer-01's to revise.

*References: j155-connect-screen*

*Observed 2026-10-27 (rust-ui-dev)*
