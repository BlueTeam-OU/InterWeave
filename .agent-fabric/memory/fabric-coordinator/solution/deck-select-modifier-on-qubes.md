---
role: "fabric-coordinator"
class: solution
topic: "deck-select-modifier-on-qubes"
description: "FleetDeck alt+drag selection fails on Qubes because dom0 Xfce grabs alt+drag to move windows; super cannot replace it because a terminal's SGR mouse report carries only shift/alt/ctrl"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 1917d5290682ae54
---

## FleetDeck alt+drag selection fails on Qubes because dom0 Xfce grabs alt+drag to move windows; super cannot replace it because a terminal's SGR mouse report carries only shift/alt/ctrl

On 2026-11-08 (j539) the owner could not select text in a FleetDeck harness pane. Herdr #60's
alt+drag (`ui.mouse_select_modifier`, default "alt") was installed and live: FleetDeck 0.10.0, server
handed off from herdr at install. The cause was dom0's Xfce, whose "Key used to grab and move
windows" is Alt, so alt+drag moved the Qubes window. Setting the modifier to "super" did nothing.
ptyxis's SGR mouse report encodes only shift, alt and ctrl, so super never reaches the deck. The
deck's own test asserts this.

**Why:** a modifier the outer terminal cannot report cannot select; it only looks configurable.

**How to apply:** on Qubes, free Alt in dom0 (Window Manager Tweaks → Accessibility, or
`xfconf-query -c xfwm4 -p /general/easy_click -s Super`) and keep the deck on alt. Shift+drag is
ptyxis's own whole-screen selection. A changed client-side ui setting needs the client restarted:
`server reload-config` reaches the server only.

*Anchors: `herdr:src/raw_input.rs::parses_sgr_mouse_observable_modifiers`*

*Observed 2026-11-08 (fabric-coordinator)*
