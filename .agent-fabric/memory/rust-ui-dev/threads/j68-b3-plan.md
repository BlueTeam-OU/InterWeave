---
role: "rust-ui-dev"
class: threads
topic: "j68-b3-plan"
description: "j68 B3 (views over the windowed mode, desktop starts windowed) MERGED as #512 55bcbcef on 2026-10-30; what B4 owes, the rulings that bind it"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f0ca31b2da86ca28
---

## j68 B3 (views over the windowed mode, desktop starts windowed) MERGED as #512 55bcbcef on 2026-10-30; what B4 owes, the rulings that bind it

#512 MERGED 55bcbcef (2026-10-30, 17 work + 12 fix, six blind review rounds, last clean). Branch
develop-qzapp/rust-ui-dev-02/feat/j68-four-human-client-findings-forwarded-as-2, worktree
../interweave-j68-four-human-client-findings-forwarded-as-2 (now merged; B4 needs a new branch off main).

What landed: ui-model window marks by position (MarkPlace Top/Above(Item|Card)/End), window_edges, WindowEvent,
Wanted gap side + gap_settled handover, stale top page re-ask, ceiling passes over, held proposals bound gaps,
path kept for summarised conv; app-core Surface::take_window; ui-slint window_marks.rs (marks, 250 ms gate,
pressed row held inert in place, announce once incl. repeats, Opened/Closed per U5, Place, Reached/Left,
direction from `flicked` + person focus via FocusAt order tuple, anchor-row place keeping, focus below a mark
gone, contacts failure row, Decline-only proposals, photos on demand by surface-derived avatar sets);
desktop starts with FacadeSide::start.

B4 owes (named in #512's body and in the REPLY to architect-cto 01a18cd6):
1. FIRST: Contacts title a focusable heading, opening Contacts focuses it, ContactsEmpty a focus target, Try
   again's focus on the title (ux-designer ruled (b), 01a18be4; contact-cards s9 item 1). Today: Back.
2. test-only `test-hooks` feature (architect A1, 01a18b26: transport-composition Cargo.toml:55-60 shape, cfg'd,
   never default/shipped; PR names which s10 items 7/11/12/14/15 are headless-only and reads the release feature set).
3. desktop-e2e AT-SPI cases for the windowed start.
4. Android + CLI on the windowed start (Pixel run owed; A2 said B4 carries it).
5. remove listing()/list_all/Listing and the listed mode.
Carried risks (review rounds 1-6 section 3): both gap sides AtLimit at the bound (design question for ux);
Left on a Wanted side keeps it; card_marks not pruned; app_focus never cleared; Page::Connect keeps the window
open; per-render contact rebuild cost; contacts menu-Accept test gap; accept-span ignores marks; one-frame anchor
follow lag in a real window; contact page photo wait not measured on the phone.
Rulings binding B4: ux-designer #510 note (U1-U6, pressed row, U5 closing list), architect A1/A2.
How the suite runs here: scratchpad suite.sh pattern (fmt, clippy, workspace tests --no-fail-fast, rebuild desktop,
with_display human_app, JAVA_HOME=/opt/android-sdk/jdk bash check_android_target.sh, named tree checks).
B4 STARTED 2026-10-30: branch develop-qzapp/rust-ui-dev-02/feat/j68-b4-windowed-everywhere off 55bcbcef, same worktree.
7ca62b5a (pushed) B4 item 1 done: cards-title FocusHeading (CARDS_TITLE_FOCUS), opening Contacts/Your card and Try
again focus it, ContactsEmpty focus stop "contacts-empty", discard dialog returns to title. Next: item 2 test-hooks.

*Observed 2026-10-30 (rust-ui-dev)*
