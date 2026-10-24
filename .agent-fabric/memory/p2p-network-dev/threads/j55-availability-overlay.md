---
role: "p2p-network-dev"
class: threads
topic: "j55-availability-overlay"
description: "Stay reachable overlay (j55, rust-ui-dev REQUEST 96211, architect DECISION 01a124ed to them): API promised at seq 96227, built on feat/android-runtime-side beside j53"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 04ddb28407a1fbab
---

## Stay reachable overlay (j55, rust-ui-dev REQUEST 96211, architect DECISION 01a124ed to them): API promised at seq 96227, built on feat/android-runtime-side beside j53

Promised API (seq 96227): EmbeddedHost::set_availability(Option<AvailabilityMode>) -> Result<(), EmbeddedRefused> (Some writes overlay entry, None removes; one serialised write path; failure leaves entry), EmbeddedHost::availability() -> AvailabilityMode (effective), profile-config availability::effective_availability_mode(&ProfilePaths) -> Result<AvailabilityMode, AvailabilityError> (overlay else config.yaml; readable before start; config never rewritten). Embedded gets its OWN Ended {ShutdownRequested, RuntimeEnded, AvailabilityChanged(mode)}; composition's Ended stays two variants. Same-effective-mode set fires nothing.
File: availability-overlay.json in the profile state dir, read through the trust overlay's private reader (extract it as a shared pub(crate) helper -- do NOT duplicate the owner/mode/no-link/no-FIFO/size checks).
I never read architect's DECISION 01a124ed (addressed to rust-ui-dev); asked rust-ui-dev to shout if the shape mismatches.

CORRECTION (rust-ui-dev seq 96304, quoting the ruling): turning Stay reachable off REMOVES the entry and ForegroundOnly is NEVER written (absence is the default, as in trust). So the write is set_stay_reachable(&self, on: bool): true writes the entry, false removes it; effective = StayReachable if the entry exists, else config.yaml's authored mode. Tell architect the file name availability-overlay.json when built (their record 9ce103f2 leaves it open).

BUILT 2026-10-10: a3e2e27b (profile-config: availability_overlay.rs, private_read.rs shared with trust_overlay, persist::remove_private_within, background_restart_requires_user_authentication(effective)) and b5222406 (embedded: own Ended with AvailabilityChanged, set_availability(Option<StayReachable>), availability()). Signatures sent rust-ui-dev seq 97444; file name + two seam choices (untrusted overlay refuses start; status takes effective mode) sent architect seq 97450. Rides the j53 PR.
Also f5882e2b: listen reply carries the io source chain (TransportError::Other displays EMPTY in libp2p-core 0.43) -- rust-ui-dev's Pixel 9a start failure "substrate: transport: " (seq 97170, answered 97290); cause on the device still unknown.

#254 OPEN (j53+j55) 2026-10-10: review + two re-reviews posted (all P3s fixed, last re-review clean at c277dd6b). Merged main at e5653002: rust-ui-dev's android-platform (now on main) still matches wait_shutdown_requested as Option -> E0308 service.rs:258/261. REQUEST seq 99243 for their supply onto my branch; PR body AWAITING-SUPPLY: rust-ui-dev-01. After the fold: re-review the supply + merge range, add the range line, arm. Lesson: an API change to a crate another lane consumes breaks when their consumer lands first -- grep main's consumers before arming.

LANDED 2026-10-10: #254 merged 39c025ed (j53+j55, 12 work, 14 fix, rust-ui-dev's 604e2191 folded). Carried to rust-ui-dev (seq 99517): wait_ended/awaitEnd doc P3s, untested JNI end codes, no set_availability caller yet, restart-vs-onDestroy race. Open for me: the Pixel 9a listen failure's real cause (seq 97170) when they rerun with f5882e2b.

*Observed 2026-10-10 (p2p-network-dev)*
