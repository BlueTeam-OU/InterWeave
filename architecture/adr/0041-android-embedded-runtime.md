# Android human client embeds TransportRuntime in a foreground service

**Status:** Accepted; platform-specific amendment to ADR-0015/0032/0037

## Context

The desktop standalone-daemon model maps poorly to Android background/process rules. Android nevertheless needs the same PeerId/EndpointId/network semantics and, when the user asks to remain reachable, a user-visible lifecycle owner independent of the Activity window.

## Decision

The Android first-party app embeds the Rust `TransportRuntime` inside an Android foreground service host rather than launching a standalone daemon or exposing local TCP/UDS IPC. The UI talks to the runtime through the neutral `LOCAL-CLIENT` in-process adapter. The service owns the `human` EndpointId lease while active.

Continuous background reachability is explicit user opt-in and uses the current Android `remoteMessaging` foreground-service category subject to SPIKE-008 target-SDK/Play-policy validation. **The Play-policy half of that condition (A 2026-10-19):** the platform defines the type as "Transfer text messages from one device to another. Assists with continuity of a user's messaging tasks when they switch devices." (Foreground service types, read 2026-10-19), and Play's row for `TYPE_REMOTE_MESSAGING` reads "Relay text communication to another device" with the example of a web client paired by QR code (Play Console Help answer 13392821, read the same day); the first sentence of the platform's definition is what a peer-to-peer messenger with no server does — a text message is transferred from the peer's device to this one — while the second sentence and Play's example both describe the person's own messaging continued across their own devices, which InterWeave is not; Play states its use-case list is non-exhaustive and takes a hand-entered one, so the Play declaration states InterWeave's use case by hand — the service keeps the person reachable for text messages from their peers' devices while the app is in the background, with no server to hold them — and names deferral as the person being unreachable. Play's policy criteria for any FGS type include that the service "runs only for as long as necessary to complete the task"; the declaration states the task as it is — being reachable for as long as the person keeps Stay reachable on, ended by their switching it off, by their stopping the app or by the OS — because an open-ended, person-ended task is what a serverless messenger's reachability IS, and no shorter true statement exists. That criterion applies to `specialUse` equally, so the fallback does not escape it. Step 10's store-listing check (plan §20) quotes both texts with the date read and records this gap; the condition itself is met only by Play's acceptance of that declaration at the Stage 19 store submission (§22), and no build may claim Play-store background reachability before it. If Play refuses the declaration — the risk being that a reviewer reads the type by its second sentence and Play's example rather than its first — the fallback is `specialUse` with its free-form `PROPERTY_SPECIAL_USE_FGS_SUBTYPE` explanation, which Play reviews the same way: a manifest and declaration change, not an architecture change — the service, the embedded runtime and the opt-in stand. This record is revisited only if Play refuses both. Sideloaded and other-store builds are unaffected. Foreground-only mode is supported. On API 37+ `INTERNET` is a runtime permission (A 2026-10-10): the Activity asks for network access before it binds the Service, a denial opens the store without a network in a Service that runs no runtime and holds no lease (reading, Keep and Unkeep work as ADR-0044 defines; nothing sends or joins), and a stay-reachable Service restarted without the permission posts one notification and ends (`human-client-android.md` §Runtime permissions). Where the opt-in LIVES (A 2026-10-10): `runtime.android.availability_mode` in the profile's `config.yaml` is the authored default, never rewritten by the host or the runtime; the person's choice is a persisted overlay in the profile's state directory under the trust boundary (the trust overlay's sibling, ADR-0028), written through one embedded-host call on the lifecycle seam, read as one effective mode (overlay, else config) by everything derived from it; turning it off removes the entry; a change while running restarts the service in the new mode. No centralized push wake-up dependency is added. `stay-reachable` may be combined with `user-presence` key unlock, but that combination must expose `background_restart_requires_user_authentication=true` and must not claim automatic reachability after process/service restart.

## Alternatives considered

Standalone daemon process; loopback TCP daemon; Kotlin/JVM networking implementation; WorkManager as permanent socket owner; FCM-backed wakeup; make Android foreground-only.

## Consequences

Network protocols remain identical to desktop, but the local process boundary differs. Activity recreation does not imply transport restart while the service lives. Service/process loss makes the endpoint offline; no mailbox appears.

## Security implications

Android in-process data/admin separation prevents confused-deputy wiring but cannot sandbox arbitrary code execution within the same APK process. Remote event handlers are constructed without admin capability. OS service state and notification interactions are local platform events, not network authority.

## Operational implications

A persistent notification is visible while stay-reachable mode runs. Android/OEM/background policy may still stop the process; the product must present availability honestly.

## Implementation implications

Build a Rust Android runtime adapter plus minimal platform glue for Service/notification/network callbacks. Do not reuse desktop socket keepalive inside the process; service/session lifetime revokes leases directly.

## Revisit conditions

Revisit if Android adds a better first-class persistent P2P/messaging execution primitive, Play refuses both the `remoteMessaging` and the `specialUse` declarations (A 2026-10-19), or process isolation becomes necessary enough to justify an Android Binder/service-process architecture.

## Amendments

Full notes: [`history/0041-amendments.md`](./history/0041-amendments.md).

| Date | Amendment | Effect |
|---|---|---|
| 2026-10-19 | Play's remoteMessaging use case is narrower than the platform's; the Play-policy condition is met at the Stage 19 submission, specialUse the fallback | Decision: the category stands on the platform's definition; the Play declaration states InterWeave's use case by hand (reachability for peers' text messages with no server, deferral = unreachable); step 10 quotes both texts and records the gap; the Play-policy condition is met only by Play's acceptance at the §22 submission, and no build claims Play-store background reachability before it; `specialUse` with its free-form subtype explanation is the fallback, a manifest change only; Play's "as long as necessary" criterion is answered by stating the task as person-ended, and binds the fallback too; revisit only if Play refuses both. |
| 2026-10-10 | The person's stay-reachable choice is a persisted overlay in the state directory; config.yaml stays the authored default | Decision: `runtime.android.availability_mode` is the authored default and no API rewrites `config.yaml`; the person's Stay-reachable act is a persisted overlay in the profile's state directory under the trust boundary (ADR-0028's trust overlay's sibling), written through one embedded-host call, read as one effective mode by the Service, the derived diagnostic and `background_restart_requires_user_authentication`; off removes the entry; a change restarts the service in the new mode. Not ruled: config as a ceiling. |
| 2026-10-10 | Network access is a runtime permission on API 37+: the Activity asks before binding, the denied state opens the store without a network, a reachable Service posts one notification and ends | Decision: on API 37+ (measured on the Pixel 9a) `INTERNET` is a runtime permission and a fresh install does not hold it; the Activity asks before binding the Service, notifications only after; a denial is the `network_denied` state — the Service opens the store and view hub, starts no runtime, holds no lease, so reading and Keep work as ADR-0044 defines and nothing sends or joins; a reachable Service restarted without the permission posts one notification and ends, and stay reachable requires the notification permission; the embedded runtime is to answer the `EPERM` as a new `EmbeddedRefused::NetworkDenied` (planned). The journey is in human-client-android.md. |
