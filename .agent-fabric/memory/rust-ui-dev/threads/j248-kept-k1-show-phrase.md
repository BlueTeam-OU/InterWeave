---
role: "rust-ui-dev"
class: threads
topic: "j248-kept-k1-show-phrase"
description: "j248: render() kept-K1 rule at a flow's end (ready_check::at_flow_end); c7b983cee pushed, stacked on #611; PR after #611 merges"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d4c7dff8ac7ced18
---

## j248: render() kept-K1 rule at a flow's end (ready_check::at_flow_end); c7b983cee pushed, stacked on #611; PR after #611 merges

j248 (carried by architect-cto REPLY 01a1c54b on #611): render() cleared `platform.start/kept`
at the end of ANY flow, so a K1 that recovery_refused kept while Show my phrase showed was
dropped (raise_recovery returns early while a flow shows). Back before OFFLINE asked no start,
so the person was left on Settings with no client and no K1.

Fix c7b983cee on `develop-qzapp/rust-ui-dev-01/fix/j248-kept-k1-after-show-phrase` (pushed,
stacked on #611 head a346f9edb): `ready_check::at_flow_end(from_start, kept, start_asked)`
-> Drop / Keep / Raise; `Over::opened_from_start` (named so because clippy's
wrong_self_convention rejects `from_*(self)` on the Android target only). Host test
mutation-checked; Android clippy OK. No device run: the case needs a refused start
while S1 shows, which no hook makes today. That limit goes in the PR body.

Next: once #611 merges (armed 2026-11-10 on AUTHORIZES 01a1c54b via --defer-job j246),
fold origin/main, open the PR (1 work commit: needs the owner's word or architect-cto's
AUTHORIZES; boundary? onboard.rs is apps/human-android/src, check the arm dry-run), then run
a blind review. Sent REQUEST 01a1c551 to ux-designer-01: the three item-15 checks of #611.
Related: [[j246-heading-focus]] [[authorizes-binds-head]]

**2026-11-10 ~14:15Z.** #611 merged e260b4058; #615 opened (fold c7bb1b52f). Round 1
(review 5842921911, judged 5842995100): F1 P1 real — on the Android backend a defer timer's
invoke_from_event_loop closure runs before do_render in the same poll, so K1 raised at the
handover pushed SECURE_ON before the old flow's after-frame SECURE_OFF: flag off under K1/R2.
Fix 5676eb3c7: onboard::secure_off asks nothing while ONBOARDING is set; Show my address's guard
clears through it too. F3 d432bfd82: Over moved to ready_check, at_flow_end(Over,..). F2 doc.
Re-review of c7bb1b52f..5676eb3c7 dispatched. Lesson: a judge brief's range is the PR's
(base..head), never head..head — post-review refuses coverage outside an empty range; a
re-review brief takes no previous_findings (it reads the last posted review).

**~14:35Z.** Re-review 5843091432: F1/F2 fixed, F3 partly, F4 P3 (secure_off guard untested,
Android-only). Both carried to j251 (android-e2e case needs a hook refusing a start while S1
shows). Answered on the PR. AUTHORIZES asked of architect-cto-01 (REQUEST 01a1c5c6). Arm:
`fabric-pr arm 615 --authorization <id> --defer "c7bb1b52:F3" "<why>" --defer "5676eb3c:F4" "<why>" --basis "..."`.
Do not push before arming.

**~14:10Z (new session).** AUTHORIZES 01a1c5c7 arrived; armed #615 at 5676eb3c with both defers and
the basis naming j251's case and the Show-my-address SECURE_OFF risk (architect-cto's two conditions).
fabric-pr wait-merged running; deliver j248 on merge.

**Merged** 950c2fe73 (2026-11-10). j248 delivered.

*References: authorizes-binds-head, j246-heading-focus*

*Observed 2026-11-10 (rust-ui-dev)*
