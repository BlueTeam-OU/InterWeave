---
role: "language-culture"
class: workflow
topic: "observed-apart-from-expected"
description: when a text keeps an observed case apart from an expected one, check every sentence nearby for an unconditional claim that merges them again
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0ddd66bbb621bafc
---

## when a text keeps an observed case apart from an expected one, check every sentence nearby for an unconditional claim that merges them again

On #659 (j307, my 28ae535b8 on 4691e8000), the author's fix split the launcher bullet in §11
into what was observed (the j98 run, Stay reachable, no restart seen) and what is expected
(foreground-only, checked by A16). The same bullet still opened with "and nothing restarts
by itself" for every mode, and line 677 above it said "No restart runs after such a network
grant". Both claims were unconditional, and both merged the two cases again. I dropped the clause
in the passed sentence. Line 677 was outside it, so I sent it to the author as a question,
and they reworded it (901bba35e). The cold read added a second point: "checks that nothing
restarts" states a result after "is expected", so the text became "checks whether anything
restarts".

**Why:** a fix that separates evidence from prediction is only as good as the
least-qualified sentence around it. A reader takes the unconditional sentence as the rule.

**How to apply:** when a request asks to keep "observed" apart from "expected", read the
whole paragraph, not only the changed lines. Mark each claim as observed, expected or
unconditional. Fix the unconditional ones in the passed text, and name the rest to the
author by line. A test "checks whether" something happens; it does not "check that" it
does not. Related: [[qualifier-is-a-claim]], [[word-against-every-state]].

*References: qualifier-is-a-claim, word-against-every-state*

*Anchors: `design/2026-11-05-unlock-policy-pending.md::A network grant that ends the process`*

*Observed 2026-11-13 (language-culture)*
