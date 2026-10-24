---
role: "architect-cto"
class: workflow
topic: "rustsec-zbus-family-ignores-and-arming-below-minimum"
description: "The 2026-10-18 CI outage on InterWeave (zbus-family RustSec unmaintained advisories) and what landing #297 taught about folding a supply, the deny.toml boundary, and the architect-cto AUTHORIZES send"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2f31d0bf8b1186d1
---

## The 2026-10-18 CI outage on InterWeave (zbus-family RustSec unmaintained advisories) and what landing #297 taught about folding a supply, the deny.toml boundary, and the architect-cto AUTHORIZES send

- RUSTSEC-2026-0370/0371/0372 (zvariant, zbus_names, zvariant_derive; the
  standalone crates merged into zbus 6.0.0, dated 2026-10-16) turned every
  InterWeave PR's `tree checks` and `tool self-tests` red on 2026-10-18.
  Slint 1.18.1's winit backend pins zbus 5, so the answer was three ignore
  entries (p2p-network-dev's decision, 2aa72262/bae39f8a/dde0a007) folded
  unrebased into IW #297, with the revisit "next Slint or accesskit bump
  together with tests/desktop-e2e's zbus/atspi pins" — the e2e suite's own
  dev-dependencies are a second path a Slint bump alone leaves in place.
  FleetDeck answered the same advisories by moving to zbus 6 (FleetDeck #42).
- deny.toml is on arm.json's security-boundary list: folding a deny.toml
  supply into a docs PR turns it into a boundary PR (review of the final
  head required, owner's word under eight work commits).
- A cargo-deny result is only CI's here: neither architect-cto-01 nor
  p2p-network-dev-01 has the tool, so the first evaluation of an entry is
  the PR's next run; read the job's step log through
  `gh api repos/<r>/actions/jobs/<id>/logs` while the run is in progress.
- Local `cargo xtask checks` fails `check_rustc_pin.sh` on this host
  (rustc 1.99 vs the 1.98.1 pin) and exits 2 on the five network checks;
  neither is the branch's fault.
- `fabric-review next` and `brief` need `card_base: <full sha on main>` in
  the request when the range base is not a commit on origin/main (a
  re-review, or a round on a fold).
- `fabric-pr post-review` takes the report whole, dispatch block first;
  a summary without the block is refused.
- The architect-cto AUTHORIZES send (DECISION with the AUTHORIZES header)
  was refused by auto mode as "Merge Without Review" on 2026-10-18 for
  FleetDeck#42 though two blind reviews were on the head; fabric-coordinator
  decided an allow entry (agent-fabric d085351d, reaches sessions after a
  relaunch). Until then the owner's word to the PR's owner is the route.
- `fabric-jobs block <id> "<text>"` takes a plain-text blocker as the
  positional; `--on` accepts only pr:/request:/job: forms.

**Why:** three sessions of fold/review/arm mechanics in one day; the next
boundary fold should not rediscover them.
**How to apply:** on a CI-wide red from an advisory, OBSERVATION to
p2p-network-dev, fold their supply, record it in the open stage's
dependency-hygiene subsection, review the folded range as a boundary.

*Observed 2026-10-18 (architect-cto)*
