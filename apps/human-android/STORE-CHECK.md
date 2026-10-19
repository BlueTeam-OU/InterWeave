<!-- SPDX-License-Identifier: Apache-2.0 -->
# Store-listing check: the Android client against Google Play's policy

Plan §20 step 10. This check closes ADR-0041's Play-policy condition,
which SPIKE-008 left open: whether the `remoteMessaging` foreground
service this client declares is one Play accepts. It quotes the policy
as read, with the date, and says where the client meets it and where it
does not. What it concludes is architect-cto's, not this file's: the
question went as OBSERVATION 01a1527e (2026-10-19), and the ruling is
recorded under "Decision".

Re-read the sources whenever this check is cited for a release: Play
policy changes, and a quote here is only as current as its date.

## What the client declares

From `android/app/src/main/AndroidManifest.xml` and
`android/app/build.gradle.kts`:

| | |
|---|---|
| applicationId | `org.interweave.human` |
| minSdk / targetSdk / compileSdk | 30 / 36 / 36 |
| versionCode / versionName | 1 / `0.0.0` (release versioning is plan §22's) |
| foreground service | `.NetworkService`, `foregroundServiceType="remoteMessaging"`, not exported |
| permissions | `INTERNET`, `ACCESS_NETWORK_STATE`, `FOREGROUND_SERVICE`, `FOREGROUND_SERVICE_REMOTE_MESSAGING`, `POST_NOTIFICATIONS` |
| backup | `allowBackup="false"` and `dataExtractionRules` excluding every domain from cloud backup and device-to-device transfer |

What the service is for: under **Stay reachable** the person keeps the
network service running while the app is in the background, so other
people's peers can reach them over P2P. It runs with a visible ongoing
notification, the person turns it on and off, and without it the client
is reachable only while open (`human-client-android.md`).

## The policy, as read on 2026-10-19

**Play Console Help, "Understanding foreground service and full-screen
intent requirements"** (`support.google.com/googleplay/android-developer/answer/13392821`):

> When your apps target Android 14 and above, you'll need to declare any
> foreground service types that you use in a new declaration on the App
> content page (Monitor and improve > App content) in Play Console.

For each type: a description of the functionality, the user impact if
the task is deferred or interrupted by the system, a link to a video
demonstrating the feature, and a use case chosen from a list that the
page calls "non-exhaustive", or entered by hand.

Its row for `TYPE_REMOTE_MESSAGING`:

> Relay text communication to another device
>
> Assists without continuity of a user's messaging tasks when they
> switch devices. For example, allow users to send text messages via a
> web client by scanning a QR code and establishing a connection
> between the web client and the phone. Not limited to SMS.
>
> Show the user initiate the syncing or transfer of text messages and
> transferring between the phone and an external web client or device.

**Device and Network Abuse policy, "Permissions for Foreground Services
(FGS)"** (`support.google.com/googleplay/android-developer/answer/13315670`):

> Apps are only allowed to declare a foreground service permission if
> the use:
>
> - provides a feature that is beneficial to the user and relevant to
>   the core functionality of the app
> - is initiated by the user or is user perceptible (for example, audio
>   from playing a song, cast media to another device, accurate and
>   clear user notification, user request to upload a photo to the
>   cloud)
> - can be terminated or stopped by the user
> - can't be interrupted or deferred by the system without causing a
>   negative user experience or causing the user anticipated feature to
>   not work as intended (for example, a phone call needs to start
>   immediately and can't be deferred by the system)
> - runs only for as long as necessary to complete the task

**Android Developers, "Foreground service types"**
(`developer.android.com/develop/background-work/services/fgs/service-types`,
page last updated 2026-10-16), Remote messaging:

> Transfer text messages from one device to another. Assists with
> continuity of a user's messaging tasks when they switch devices.

## Where the client stands

| Criterion | The client | Holds? |
|---|---|---|
| Beneficial and core | Receiving messages is the product's core | yes |
| Initiated by the user or perceptible | Stay reachable is the person's switch; an ongoing notification shows while it runs | yes |
| Can be stopped by the user | The same switch, in the app; the notification itself has no stop action (`NetworkService.kt`) | yes |
| Cannot be deferred or interrupted without harm | A deferred service misses messages sent meanwhile | yes |
| Runs only as long as necessary | It runs for as long as the person wants to stay reachable, which has no task end | **contested** |
| Type fits `remoteMessaging` | Play describes the type as relaying the person's OWN messages to another of their devices; this service receives OTHER people's messages | **gap** |

Two rows do not clearly hold, and they are the condition ADR-0041 left
open. A hand-entered use case is allowed by the declaration form, so the
gap is not a refusal on its face. It is also not evidence of acceptance:
how Play's review judges a P2P messenger's declaration cannot be
measured from here.

## Decision

ADR-0041 A 2026-10-19 (architect-cto, REPLY 01a15282, commit 432a354f
on InterWeave #301), recorded here as ruled:

- `remoteMessaging` stands on the platform's definition: a text message
  is transferred from one device (the peer's) to another (this phone),
  and this service is what receives it. The gap is in Play's example,
  not in the type.
- The Play declaration states the use case by hand: the service keeps
  the person reachable for text messages from their peers' devices
  while the app is in the background, with no server to hold them;
  deferral means the person is unreachable.
- Play's acceptance of that declaration happens at the store submission
  (plan §22). This check closes the target-SDK half of SPIKE-008's P1;
  **the Play half stays open by name** until then. No build claims
  Play-store background reachability before acceptance.
- If Play refuses: `specialUse` with its
  `PROPERTY_SPECIAL_USE_FGS_SUBTYPE` explanation. ADR-0041's revisit
  fires only if Play refuses both.

Not covered by the ruling, and put to architect-cto: the criterion
"runs only for as long as necessary to complete the task", which Stay
reachable meets only if "the task" is staying reachable for as long as
the person chooses.

The declaration's and the demo video's wording are written with the
store listing in plan §22, with language-culture.

Builds distributed outside Play are not affected by this check.
