// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
package org.interweave.human

/**
 * The native library's entry points (`apps/human-android/src/jni.rs`).
 * Every call that BLOCKS is made off the main thread; the Service runs
 * start and stop on one executor, so they never overlap.
 */
internal object Native {
    init {
        System.loadLibrary("interweave_human_android")
    }

    /** The profile's effective availability: the runtime is bound to the app. */
    const val FOREGROUND_ONLY = 0

    /** The profile's effective availability: the person chose Stay reachable. */
    const val STAY_REACHABLE = 1

    /**
     * Start the runtime, the store and the facade under [appDataDir], the
     * app's data directory. BLOCKS. Answers the effective availability,
     * [FOREGROUND_ONLY] or [STAY_REACHABLE], or a negative refusal: the
     * reason is in the log, and the window shows the service as not
     * running.
     */
    @JvmStatic external fun start(appDataDir: String): Int

    /**
     * start's refusal when the platform refuses the runtime its sockets
     * (`EmbeddedRefused::NetworkDenied`): a race with the grant or another
     * restriction, answered with the network-denied posture as a withheld
     * permission is (human-client-android.md, "Runtime permissions and
     * the network-denied state", step 6).
     */
    const val REFUSED_NETWORK_DENIED = -12

    /** startWithoutNetwork's answer: the posture runs. */
    const val NETWORK_DENIED = 2

    /**
     * Start the network-denied posture under [appDataDir]: the store and
     * the view hub, no runtime and no endpoint lease. BLOCKS. Answers
     * [NETWORK_DENIED], or a negative refusal (in the log).
     */
    @JvmStatic external fun startWithoutNetwork(appDataDir: String): Int

    /**
     * Wait at most [timeoutMs] for the person to ask, from the window, for
     * the network access the platform withholds; true when they did.
     * BLOCKS.
     */
    @JvmStatic external fun waitNetworkAsk(timeoutMs: Long): Boolean

    /** waitEnded: nothing runs. */
    const val ENDED_NOT_RUNNING = 0

    /** waitEnded: an admin port or a stop asked the runtime to stop. */
    const val ENDED_ASKED = 1

    /** waitEnded: the runtime stopped on its own; no one asked. */
    const val ENDED_RUNTIME = 2

    /** waitEnded: the effective availability changed; restart in it. */
    const val ENDED_AVAILABILITY = 3

    /**
     * Wait until the client ends or must restart. BLOCKS. One of the
     * ENDED_ values above.
     */
    @JvmStatic external fun waitEnded(): Int

    /** Stop the runtime, its store and its facade. BLOCKS for the grace. */
    @JvmStatic external fun stop(): Boolean

    /**
     * Wait at most [timeoutMs] for the count of messages no window has
     * read to change: the new count, 0 meaning withdraw the notice, or -1
     * when it did not change. BLOCKS.
     */
    @JvmStatic external fun waitNotice(timeoutMs: Long): Int

    /**
     * The Service's network callback was registered: no network is known
     * yet, which is offline until one is reported (plan section 20 step
     * 5). Never waits, nor do the four below
     * (`a_network_report_never_waits_on_a_start`, android-platform's
     * `tests/service.rs`).
     */
    @JvmStatic external fun networkWatching()

    /**
     * The network [network] (its `Network.getNetworkHandle`), the whole of
     * it: its [addresses] and its [dnsServers], each address its length
     * (4 or 16) then its `InetAddress.getAddress` bytes, and its Private
     * DNS host when Private DNS is strict on it, else null.
     */
    @JvmStatic external fun networkLink(
        network: Long,
        addresses: ByteArray,
        dnsServers: ByteArray,
        privateDns: String?,
    )

    /** The default network is [network]; 0 when there is none. */
    @JvmStatic external fun networkDefault(network: Long)

    /** The network [network] is gone. */
    @JvmStatic external fun networkLost(network: Long)

    /** The callback was unregistered: the platform's view is unknown. */
    @JvmStatic external fun networkUnwatched()

    /**
     * The platform's touch values for a row's swipe (the message swipe
     * spec, sections 4, 6 and 7): the touch slop and the left and right
     * system gesture insets in dp, a right-to-left layout, the system's
     * animations off, the recommended time a snackbar with a control
     * stays up, in ms, and whether the person set 24-hour time, which every
     * time the app shows follows. Never waits.
     */
    @JvmStatic external fun touch(
        slop: Float,
        edgeLeft: Float,
        edgeRight: Float,
        rtl: Boolean,
        reducedMotion: Boolean,
        recommendedMs: Long,
        hour24: Boolean,
        longPressMs: Long,
    )

    /**
     * Wait at most [timeoutMs] for a swipe to arm; true when one did, and
     * the Activity plays the haptic tick. BLOCKS.
     */
    @JvmStatic external fun waitHaptic(timeoutMs: Long): Boolean

    /** A person-facing text by its key ([Text]), with [count] filled in. */
    @JvmStatic external fun text(key: Int, count: Int): String

    /**
     * Whether the profile's identity opens ([IDENTITY_READY]), or where
     * the identity screens start: no record, or one the phone lost. Makes
     * the Keystore cipher the screens use. Call before binding the
     * service.
     */
    @JvmStatic external fun identityState(appDataDir: String): Int

    const val IDENTITY_READY = 0

    /**
     * The key needs the person and their last authentication is too old:
     * raise the prompt before binding; its answer goes to [prompted].
     */
    const val IDENTITY_AUTHENTICATION_NEEDED = 3

    /** The identity screens' next ask of the Activity ([Ask]), or 0. BLOCKS. */
    @JvmStatic external fun waitIdentityAsk(timeoutMs: Long): Int

    /**
     * The platform's Back, for the window: a screen with a way back takes
     * it, else [waitLeave] answers true and the Activity finishes.
     */
    @JvmStatic external fun back()

    /** FLAG_SECURE is on for Show my address: it may draw its addresses. */
    @JvmStatic external fun showSecured()

    /** Whether the phone has a camera at all: Scan a code is offered only then. */
    @JvmStatic external fun cameraAvailable(present: Boolean)

    /** One camera frame's luma plane, row by row with `rowStride` bytes a row. */
    @JvmStatic external fun cameraFrame(width: Int, height: Int, rowStride: Int, luma: java.nio.ByteBuffer)

    /** The camera permission refused at Scan a code; `forGood` when it will not be asked again. */
    @JvmStatic external fun cameraDenied(forGood: Boolean)

    /** The camera stopped under the scan (failed to open, or taken away). */
    @JvmStatic external fun cameraClosed()

    /** Back from the app's system settings, opened by Open settings. */
    @JvmStatic external fun settingsReturned()

    /**
     * The photo picker's answer: the image's bytes (empty when they could
     * not be read), or null when nothing was chosen.
     */
    @JvmStatic external fun photoPicked(bytes: ByteArray?)

    /** True when Back found no way back: finish. BLOCKS. */
    @JvmStatic external fun waitLeave(timeoutMs: Long): Boolean

    /** Whether the service stopped for Show my phrase ([Ask.OFFLINE]). */
    @JvmStatic external fun offline(stopped: Boolean)

    /** The credential prompt's answer, for the identity screens. */
    @JvmStatic external fun prompted(granted: Boolean)
}

/** What the identity screens ask of the Activity, as `onboard.rs` numbers it. */
internal object Ask {
    const val SECURE_OFF = 2
    const val PROMPT = 3
    const val BIND = 4
    const val FINISH = 5
    const val OFFLINE = 6
    const val ONLINE = 7
    const val UNLOCK = 8
    // The store's screens: start the network service again -- Try again
    // on the recovery screen, and after a confirmed discard, which
    // recreates the store (ADR-0060 decision 6).
    const val RESTART = 9
    // Show my address opened: FLAG_SECURE on, then Native.showSecured.
    const val SHOW_SECURE = 10
    // Scan a code: the camera on (CAMERA asked first if not held), off.
    const val CAMERA_START = 11
    const val CAMERA_STOP = 12
    // Open settings on the camera's denied-for-good line.
    const val OPEN_SETTINGS = 13
    // The own card's Add a photo or Change photo: the photo picker.
    const val PICK_PHOTO = 14
}

/** The keys of the texts the platform shows, as `jni.rs` maps them. */
internal object Text {
    const val REACHABLE_CHANNEL = 0
    const val REACHABLE_TITLE = 1
    const val REACHABLE_BODY = 2
    const val MESSAGES_CHANNEL = 3
    const val MESSAGES_WAITING = 4
    const val NETWORK_OFF_TITLE = 5
    const val NETWORK_OFF_BODY = 6
    const val PHRASE_PROMPT_TITLE = 7
    const val UNLOCK_PROMPT_TITLE = 8
}
