# ADR-0068 — amendment history

### Amendment 2026-11-15 — Rule 7 is accepted by the owner as written

The record landed on 2026-11-14 as a promotion of rules already in
force, and marked what it stated for the first time: rule 7 (the
keepalive is liveness, never authentication or authorization) and
three of the alternatives. The Status line read "rule 7 and three of
the alternatives are stated here for the first time and are marked
so", and rule 7 opened "(stated with this record; ...". The owner
accepted rule 7 as written on 2026-11-15, in the session. The rule's
text is unchanged; its mark and the Status line now carry the
acceptance and its date. The three alternatives were not part of that
word and keep their mark, "recorded with this promotion".
Propagation: the Status line; rule 7's parenthesis; this note; the log
row; the digest entry.

### Amendment 2026-11-18 — The phone's idle cost is measured once and the interval stays

As the record landed, its consequence said of the ping on a phone:
"The phone holds at the same interval as every profile until a device
measurement says otherwise, and that measurement is owed before a
release note calls the phone's idle cost known." p2p-network-dev-02
measured it on 2026-11-17 and 2026-11-18
(`apps/human-android/device-runs/2026-11-17-pixel-9a-samsung-a40-hold-ping-cost.md`,
InterWeave #712): under Stay reachable a Pixel 9a (Android 17), idle
with its screen off on Wi-Fi, kept one held peer's connection through
2 h 54 min of deep Doze at the 15 s interval, with 1822 more kernel
wakeups and 38 mAh more drawn (263 against 225) than a control window
with no peer online. Under foreground-only, the default, there is no
foreground service, so nothing is held with the screen off. A Samsung
A40 (Android 11) froze the app's process in Doze despite its
foreground service, so it held nothing and gave no cost.

The consequence now states that figure and its bounds: one window of
each condition on different days, one peer, Wi-Fi, a debug build. The
figure is not one that the revisit trigger ("a device measurement of
the phone's idle cost that the current interval makes unacceptable")
names, so the interval stays and rule 8 is unchanged; the trigger
stands for the cases not measured, which the consequence lists. The
A40's freeze is not a cost of the hold and is not this record's: it is
a limit of Stay reachable, carried in the plan's §20 closing record.

Propagation: the consequence bullet; `CONNECTIVITY.md` §14
A 2026-11-13's "Not proved"; this note; the log row; the digest entry.
