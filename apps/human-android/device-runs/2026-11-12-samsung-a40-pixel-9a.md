# Samsung A40 and Pixel 9a device runs, 2026-11-11 and 2026-11-12: the reader's open cost, an A/B of its opening, and Back while it opens

Devices: the **Samsung A40** (SM-A405FN, Android 11, **API 30**) of
`2026-11-09-samsung-a40.md`, on this login's own adb server (port 15065)
under the `samsung-a40` lease; the **Pixel 9a** (API 37) through
`.agent-fabric/bin/phone-run` under the `pixel-9a` lease. Both over
wireless debugging; no adb process of this login remained after any run
(`pgrep -u "$(id -u)" -a adb` empty). Every time below is UTC.

The case is `tests/android-e2e/tests/long_message.rs` as in
`2026-11-09-samsung-a40.md`: a desktop daemon on this host's loopback
sends each shape of `tests/android-e2e/src/long.rs` to the app over adb
reverse, and the case presses the row's Open the whole message by key
and reads the debug build's trace (`reader=drawn`, `reader=end`). Since
cd9b93b2c the shapes add two stress shapes, distinct headings and
distinct links, whose rows are all different text, so no measure is
reused (j95, design/2026-11-04-long-message.md A5).

## The batch's debug build, one run each (2026-11-11 and 2026-11-12)

The debug pair of cd9b93b2c: native library at the release opt-level
with debug assertions on, as on 2026-11-09. On the A40 this login's pair
(app d0d830e8, test 4db8c691, signer `26d1eb18`), 2026-11-11 14:39Z,
passed in 736.7 s. On the Pixel rust-ui-dev-01's pair of the same commit
(app 59efcb9a, test 1efb4b15, signer `f8b1b43e`, installed by them with
`install -r` so the owner's profile stayed), 2026-11-12 09:26Z, passed
in 562.8 s; the profile's `config.yaml`, which the case's provision step
replaces, was copied aside inside the app before and back after, its
sha256 1351b961... both times.

| shape | A40 open | Pixel open | A40 End formatted / source | Pixel End formatted / source |
|---|---|---|---|---|
| paragraph | 150 ms | 74 ms | 58 / 21 ms | 19 / 17 ms |
| list | **559 ms** | 454 ms | 59 / 17 ms | 13 / 6 ms |
| code | 476 ms | 324 ms | 25 / 23 ms | 11 / 14 ms |
| table | 480 ms | 337 ms | 42 / 21 ms | 17 / 10 ms |
| headings | **504 ms** | 354 ms | 49 / 21 ms | 11 / 14 ms |
| images | **543 ms** | 384 ms | 50 / 22 ms | 15 / 15 ms |
| heading autolinks | **685 ms** | 432 ms | 38 / 18 ms | 12 / 15 ms |
| reference link | **1,031 ms** | **626 ms** | 42 / 25 ms | 11 / 9 ms |
| distinct headings | **1,836 ms** | **830 ms** | 64 / 20 ms | 32 / 12 ms |
| distinct links | **1,481 ms** | **661 ms** | 53 / 24 ms | 33 / 16 ms |

Every End is within A5's 1 s in one frame. Against 2026-11-09 the A40's
images, heading autolinks and reference link opened 0.1 to 0.8 s
sooner, while list, headings, table and code opened 32 to 126 ms later:
list and headings crossed 0.5 s. That build stepped the reader's rows
32 KiB a timer turn; the A/B below measures what the turns cost.

## The opening, A/B on the A40 (2026-11-12)

One source, 6c8765943, built three times with the opening's two budgets
patched (`OPENING_FIRST_GO`, `OPENING_TURN` in
`crates/human/ui-slint/src/lib.rs`), the debug APKs built with
`CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=false` and
`CARGO_PROFILE_DEV_OVERFLOW_CHECKS=false` (the native library at the
release opt-level, the trace still in), signer `26d1eb18`, the same test
APK 4db8c691:

- candidate (app 8c092c06): 400 ms at the first go, then 50 ms a turn;
- stepped (bd118a83): no budget, one 32 KiB step a turn, as at cd9b93b2c;
- immediate (0e423cef): no limit, the whole build in one go.

Five rounds, 09:20 to 12:15Z, each round running the three in turn
(`install -r` between them), keyguard not showing: 15 runs, 15 passed
(611 to 714 s each). Open, ms, median (min-max) of five:

| shape | candidate | stepped | immediate |
|---|---|---|---|
| paragraph | 151 (144-162) | 160 (156-164) | 152 (150-154) |
| list | 370 (358-377) | 557 (549-562) | 370 (367-373) |
| code | 354 (352-355) | 457 (454-464) | 354 (345-359) |
| table | 353 (347-356) | 469 (464-473) | 357 (353-358) |
| headings | 368 (367-371) | 484 (483-488) | 368 (357-370) |
| images | 401 (400-410) | 533 (531-536) | 402 (399-404) |
| heading autolinks | **515** (509-515) | **660** (655-663) | **516** (511-518) |
| reference link | **754** (745-761) | **991** (987-996) | **754** (752-758) |
| distinct headings | **1,614** (1,597-1,626) | **1,685** (1,677-1,687) | **1,414** (1,406-1,418) |
| distinct links | **1,298** (1,296-1,312) | **1,350** (1,338-1,353) | **1,106** (1,100-1,108) |

The candidate equals the immediate opening on every shape built within
its first 400 ms, and is about 200 ms slower on the two stress shapes,
whose builds run past it and then yield every 50 ms. The stepped opening
is 10 to 235 ms slower than the candidate on every shape: a turn between
32 KiB steps costs a wake and a frame. The five shapes under 0.5 s on
2026-11-09 (paragraph, list, code, table, headings) are all under it
with the candidate; images moves under it too.

## Back while the reader opens (2026-11-12)

`on_a_device_back_while_the_reader_opens_closes_it_to_its_conversation`
(582de714b), once per arm, 12:24 to 12:33Z: the list shape, its Open the
whole message pressed by key and Back sent 0.2 s later from the same
phone shell, the screen recorded across the two keys (`screenrecord`,
looked at frame by frame by this login). All three passed: the reader's
page, then the conversation's, focus back on the row's Open the whole
message.

| arm | rows drawn before it closed | reader shown | opening bar |
|---|---|---|---|
| candidate | yes, 347 ms after the press | about 0.26 s, its rows in | none |
| stepped | no | about 0.17 s, empty | none |
| immediate | yes, 366 ms | about 0.26 s, its rows in | none |

The list builds within the candidate's first go, so its Back is handled
once the rows are in, not mid-build.

## What it shows

| case | result |
|---|---|
| End reaches the body's last line within 1 s, every cut shape, both phones | yes, one frame each |
| the reader's first frame within 0.5 s, the candidate on the A40 | six of ten shapes; heading autolinks 0.52 s, reference link 0.75 s and the two stress shapes over |
| the same on the Pixel, the debug build of cd9b93b2c | seven of ten; reference link and the two stress shapes over |
| a shape under 0.5 s on 2026-11-09 over it with the candidate | none |
| Back about 0.2 s into an opening closes the reader, not lost, no bar | yes, every arm |

## What it does not show

- The release build's times: the trace is the debug build's, here with
  its assertions off in the A/B and on in the one-run tables.
- The pre-batch code built the A/B's way: 2026-11-09's figures are the
  debug build with assertions on, so they are slower than the A/B's.
- The Pixel with the A/B's builds, or more than one run on it.
- A Back inside an opening that runs past the first go (a stress shape).
- That scrolling shows no blank screen or seam (A5's other half): a
  person looks.
- TalkBack (A6, A8): blocked on the accessibility adapter.
