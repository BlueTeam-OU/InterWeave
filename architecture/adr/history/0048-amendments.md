# ADR-0048 — amendment history

### Amendment 2026-10-19 — The validator holds that an amended ADR's digest entry changed with it

**Trigger.** The review class, reading InterWeave #301's head 050547b8, found ADR-0041 amended with its body, history note and table row in place and the validator green, while the digest's `### 0041` entry still described the record as it stood before (round 1, finding 1). The check proved only that an entry existed. devex-tooling built the AMENDED phase on #307 (their j163): against the merge-base with `origin/main`, an ADR whose Amendments table gained a (date, title) key fails when its own digest entry is byte-identical to the base's; no resolvable base exits 2. Control: 050547b8 fails naming the 2026-10-19 amendment; be60c085 passes.

**Ruled.** Propagation item 2 gains a mechanical half: the digest entry must change with the amendment, which the validator holds. What it says is still the author's and the review's. A count of dated citations was considered and declined by devex-tooling on the facts — the digest cites in several forms, a withdrawn amendment is rightly folded into one bullet, and entries on `main` cite no date of some row without being stale — eight by the measure "no `Amendment(s) <date>` for a table date" (which counts 0026 and 0051, whose entries carry the date in another form), six by the plain measure of the date string absent from the entry (0015, 0019, 0037, 0047, 0050, 0053 at dc897c90); whether any of the six is stale rather than folded was not judged — and no citation convention is added: the digest is a current-state summary, not a ledger of dates.

**Not ruled.** Whether the digest should ever carry one bullet per amendment: today it carries what a reader needs, folded.
