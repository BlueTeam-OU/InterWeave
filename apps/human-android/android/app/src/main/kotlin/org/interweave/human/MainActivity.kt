// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
package org.interweave.human

import android.Manifest
import android.annotation.SuppressLint
import android.app.NativeActivity
import android.hardware.biometrics.BiometricManager.Authenticators
import android.hardware.biometrics.BiometricPrompt
import android.os.CancellationSignal
import android.view.WindowManager
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.content.pm.PackageManager
import android.content.res.Configuration
import android.database.ContentObserver
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.provider.Settings
import android.text.format.DateFormat
import android.view.HapticFeedbackConstants
import android.view.View
import android.view.ViewConfiguration
import android.view.ViewTreeObserver
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.view.accessibility.AccessibilityManager

/**
 * The window: the native library's `android_main` draws the views and
 * attaches them to the network service's session. The Activity keeps
 * the service bound while it is shown, which is all foreground-only mode
 * runs on.
 *
 * Network access first (human-client-android.md, "Runtime permissions and
 * the network-denied state"): on API 37+, where INTERNET is a runtime
 * permission, the Activity asks for it BEFORE it binds the Service, and
 * asks for notifications only once network is granted -- network is the
 * one that matters. Denied, the Service still starts, in the
 * network-denied posture, and the window's one action comes back here
 * as an ask: the platform's prompt while it still offers one, else the
 * app's settings page; returning to the window checks again.
 */
class MainActivity : NativeActivity() {
    @Volatile private var shown = false
    // Show my phrase took the service offline and its words are this
    // instance's to show: nothing binds or starts the service until they
    // are gone, a return from Home included (design section 7).
    @Volatile private var phraseShown = false
    // The person declined the Unlock prompt in this Activity's life (or
    // left while it showed): a return to it raises no prompt by itself;
    // the locked notice and its Unlock wait (ux-designer-01, REPLY
    // 01a155b3). Kept across a recreation in this process (onCreate); a
    // cold launch, in a new process, prompts again.
    private var unlockDeclined = false
    /** Scan a code's camera, opened for the viewfinder and closed with it (add-a-peer 5.5). */
    private var camera: CodeCamera? = null
    /**
     * A scan wants the camera: set at CAMERA_START while the Activity is
     * shown, cleared at CAMERA_STOP and at the Activity's stop. A grant
     * opens the camera when it is set (#418 F1).
     */
    private var cameraWanted = false
    /** Open settings was pressed: the return is told to the window. */
    private var inSettings = false

    /** The request the photo picker open now answers, or -1 (j68 P3-2). */
    private var photoRequest = -1L

    /** The provider's open of the photo chosen, while it waits on it. */
    private var photoOpening: CancellationSignal? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        // A recreation the system made in this process (a language change,
        // a backgrounded instance rebuilt) is not a launch by the person:
        // it keeps the decline (#313 review 3). A bundle from another
        // process, restored after a process death, is a launch: it prompts
        // (review 4), so the decline counts only with this process's token.
        unlockDeclined = savedInstanceState?.let {
            it.getBoolean(UNLOCK_DECLINED) && it.getString(UNLOCK_DECLINED_IN) == PROCESS_TOKEN
        } ?: false
        super.onCreate(savedInstanceState)
        // Scan a code is offered only on a phone with a camera; the app
        // installs without one (add-a-peer 5.5, the camera not required).
        Native.cameraAvailable(packageManager.hasSystemFeature(PackageManager.FEATURE_CAMERA_ANY))
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putBoolean(UNLOCK_DECLINED, unlockDeclined)
        outState.putString(UNLOCK_DECLINED_IN, PROCESS_TOKEN)
    }

    override fun onStart() {
        // Before super.onStart(), which waits for the window's thread to
        // take the Start; that thread waits for this answer before it
        // chooses between the identity screens and the chats (onboard.rs).
        // After it, each would wait on the other (#309 review N1).
        val identity =
            if (identityReady) Native.IDENTITY_READY else Native.identityState(applicationInfo.dataDir)
        if (identity != Native.IDENTITY_READY && identity != Native.IDENTITY_AUTHENTICATION_NEEDED) {
            // Before the window can draw a word: the flow's whole length,
            // cleared once a frame without it was drawn (#309 F3).
            secure(true)
        }
        super.onStart()
        shown = true
        front = this
        started += 1
        Thread({ relayAsks() }, "interweave-network-ask").start()
        Thread({ relayHaptics() }, "interweave-haptic").start()
        Thread({ relayLeave() }, "interweave-leave").start()
        // Back goes to the window, whatever has focus: the views' backend
        // delivers it as a key, which reaches nothing when no control is
        // focused, and then finishes the Activity -- Back in a
        // conversation left the app (ux-designer-01, ffce28df). Ours is
        // registered after the backend's, at a higher priority, so it is
        // the one called.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            onBackInvokedDispatcher.registerOnBackInvokedCallback(
                OnBackInvokedDispatcher.PRIORITY_OVERLAY,
                back,
            )
        }
        window.decorView.viewTreeObserver.addOnGlobalLayoutListener(layout)
        systemBars(resources.configuration)
        watchSettings(true)
        touch()
        // The identity screens' asks: a first run's or a recovery's before
        // the service, Show my phrase's from Settings after it.
        Thread({ relayIdentity() }, "interweave-identity").start()
        // A key that needs the person: the prompt before anything binds;
        // a grant binds (Ask.BIND), a cancel leaves the window offline
        // with its Unlock (onboard.rs, the custody doc's failure list).
        if (identity == Native.IDENTITY_AUTHENTICATION_NEEDED) {
            if (!unlockDeclined) window.decorView.post { prompt(Text.UNLOCK_PROMPT_TITLE) }
            return
        }
        // The identity screens come before the service: a profile with no
        // identity, or one the phone lost, binds nothing until the screens
        // say the identity is ready (onboard.rs).
        if (identity != Native.IDENTITY_READY) return
        identityReady = true
        if (!phraseShown) connect()
    }

    /**
     * Stop the service for Show my phrase, and tell the flow whether it
     * stopped. No binding is not no service: under Stay reachable the
     * service is started and runs unbound, holding the profile's lock, so
     * it is bound and stopped through its binding. Only before the
     * identity is ready was none started.
     */
    private fun goOffline() {
        local?.let { service ->
            service.offline { Native.offline(it) }
            return
        }
        if (!identityReady) {
            Native.offline(true)
            return
        }
        offlineOnConnect = true
        if (!bound) bind()
        if (!bound) {
            offlineOnConnect = false
            Native.offline(false)
        }
    }

    /** Bind the network service, asking for access first where needed. */
    private fun connect() {
        if (bound) return
        when {
            networkGranted() -> {
                bind()
                askNotifications()
            }
            // The first launch asks before it binds (step 1). Every later
            // one opens the window in the posture, and only the window's
            // own action asks again: a settings page opened from here
            // would cover the window at every return to it.
            !askedNetwork() -> ask()
            else -> bind()
        }
    }

    override fun onResume() {
        super.onResume()
        // The animation scale or a screen reader's timeout may have been
        // changed in the settings meanwhile.
        touch()
        // Back from the prompt or the settings page: a grant restarts the
        // Service with its runtime.
        local?.recheck()
        // CAMERA granted in Settings, through Open settings or outside the
        // app, clears the refusal mark as a grant here does (#418 round 3).
        if (checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
            cameraHeld()
        }
        if (inSettings) {
            inSettings = false
            Native.settingsReturned()
        }
    }

    override fun onDestroy() {
        // The editor goes with the window: its photo's open is not wanted.
        cancelPhotoOpen()
        photoRequest = -1L
        // The words go with this instance; the service must not stay
        // offline for a window that no longer shows them.
        if (phraseShown) {
            phraseShown = false
            local?.online()
        }
        super.onDestroy()
    }

    override fun onStop() {
        shown = false
        // The camera is closed at the Activity's stop, and a grant answered
        // after it opens nothing; the window closes the viewfinder at the
        // same stop (activity.rs).
        cameraWanted = false
        closeCamera()
        if (front === this) front = null
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            onBackInvokedDispatcher.unregisterOnBackInvokedCallback(back)
        }
        window.decorView.viewTreeObserver.removeOnGlobalLayoutListener(layout)
        watchSettings(false)
        // A recreation for a configuration change keeps the binding: the
        // next instance finds it held, and foreground-only mode's runtime
        // and lease survive it (review F2). Otherwise the last started
        // instance lets go: the binding is the application's. Under
        // singleTask (the manifest) `started` is not expected above 1; the
        // count stays as the guard for a launch that forces another task,
        // which is not verified to reuse this one. Before singleTask a Back
        // that closed one instance over another let go of the binding the
        // other relied on, and nothing bound again (2026-10-20, the Pixel:
        // Show recovery phrase refused as Busy).
        started -= 1
        if (bound && started == 0 && !isChangingConfigurations) {
            applicationContext.unbindService(connection)
            bound = false
            local = null
            // A Show my phrase still waiting for the binding is answered
            // now, as not stopped: no binding will arrive to stop the
            // service, and a later bind must not stop it unasked (#313
            // review round 7, risk 1).
            if (offlineOnConnect) {
                offlineOnConnect = false
                Native.offline(false)
            }
        }
        super.onStop()
    }

    /**
     * The own card's photo (contact-cards design 5.2): the system photo
     * picker, images only, one item, which needs no permission; where the
     * phone has no picker, the document picker for images.
     */
    private fun pickPhoto(request: Long) {
        // A newer pick: the provider's open for the last is not wanted.
        cancelPhotoOpen()
        photoRequest = request
        val picker = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU ||
            android.os.ext.SdkExtensions.getExtensionVersion(Build.VERSION_CODES.R) >= 2
        val intent = if (picker) {
            Intent(PICK_IMAGES).setType("image/*")
        } else {
            Intent(Intent.ACTION_GET_CONTENT).setType("image/*").addCategory(Intent.CATEGORY_OPENABLE)
        }
        try {
            @Suppress("DEPRECATION") // NativeActivity has no result API of its own.
            startActivityForResult(intent, PHOTO_REQUEST)
        } catch (_: android.content.ActivityNotFoundException) {
            photoRequest = -1L
            Native.photoPicked(request, null)
        }
    }

    /** The provider's open of a chosen photo cancelled, if one waits. */
    private fun cancelPhotoOpen() {
        photoOpening?.cancel()
        photoOpening = null
    }

    @Deprecated("NativeActivity has no result API of its own.")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        @Suppress("DEPRECATION")
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != PHOTO_REQUEST) return
        // The editor that asked is gone (closed, or a window made since):
        // its answer is no one's.
        val request = photoRequest.also { photoRequest = -1L }
        if (request < 0) return
        val uri = data?.data
        if (resultCode != RESULT_OK || uri == null) {
            Native.photoPicked(request, null)
            return
        }
        // Too many opens stuck on providers that ignored their cancel:
        // this one is handed on empty rather than hold one more thread.
        if (photoOpens.get() >= MAX_PHOTO_OPENS) {
            Native.photoPicked(request, ByteArray(0))
            return
        }
        // Opened off the main thread under a signal that a newer pick, the
        // editor's close, the window's end or the deadline cancels; the
        // native side then reads the descriptor within its own deadline and
        // bound (src/photo_import.rs). Failed, it is handed on empty, which
        // the editor says cannot be used.
        val signal = CancellationSignal()
        photoOpening = signal
        val expire = Runnable { signal.cancel() }
        val main = Handler(Looper.getMainLooper())
        val openedAt = android.os.SystemClock.elapsedRealtime()
        main.postDelayed(expire, PHOTO_DEADLINE_MS)
        photoOpens.incrementAndGet()
        Thread {
            try {
                val opened = try {
                    contentResolver.openFileDescriptor(uri, "r", signal)
                } catch (_: Exception) {
                    null
                }
                main.removeCallbacks(expire)
                // Handed on whatever cancelled it: the native side and the
                // editor drop what is for a request no longer wanted, and a
                // deadline's is said as unusable.
                if (opened == null) {
                    Native.photoPicked(request, ByteArray(0))
                } else {
                    // One deadline for the open and the read: what the
                    // open left of it, nothing once it has passed.
                    val left = PHOTO_DEADLINE_MS - (android.os.SystemClock.elapsedRealtime() - openedAt)
                    Native.photoRead(request, opened.detachFd(), left.coerceAtLeast(0L))
                }
            } finally {
                photoOpens.decrementAndGet()
            }
        }.start()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == CAMERA_REQUEST) {
            val result = grantResults.firstOrNull()
            when {
                // Cancelled with no answer (the request torn down): the scan
                // ends as a closed one, nothing refused.
                result == null -> if (cameraWanted) Native.cameraClosed()
                result == PackageManager.PERMISSION_GRANTED -> {
                    cameraHeld()
                    if (cameraWanted) openCamera()
                }
                else -> {
                    // Reported for good when the last answer was a refusal
                    // too and the system says it will not ask again; a
                    // first refusal reports a plain one (#418 F2, F7).
                    val prefs = getPreferences(Context.MODE_PRIVATE)
                    val refusedBefore = prefs.getBoolean(CAMERA_REFUSED, false)
                    val asksAgain = shouldShowRequestPermissionRationale(Manifest.permission.CAMERA)
                    prefs.edit().putBoolean(CAMERA_REFUSED, true).apply()
                    if (cameraWanted) Native.cameraDenied(refusedBefore && !asksAgain)
                }
            }
            return
        }
        if (requestCode != NETWORK_REQUEST) return
        // Granted or not, the Service starts: in the posture when denied,
        // so the person's messages are never withheld for want of a
        // network.
        if (!bound) bind() else local?.recheck()
        if (networkGranted()) askNotifications()
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        systemBars(newConfig)
        touch()
    }

    /**
     * The system bars' icons are dark on the light scheme and light on the
     * dark one. The window is drawn edge to edge under the bars, so they
     * sit over the app's own surface; the platform's default, light
     * icons, could not be read over the light one (#304 F8). The views
     * follow the same night mode (Slint's color scheme), so one setting
     * decides both.
     */
    private fun systemBars(config: Configuration) {
        val light = (config.uiMode and Configuration.UI_MODE_NIGHT_MASK) != Configuration.UI_MODE_NIGHT_YES
        val both =
            WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS or
                WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
        window.insetsController?.setSystemBarsAppearance(if (light) both else 0, both)
    }

    /**
     * A screen reader's timeouts, the animation scale or the 12/24-hour
     * setting changed while the window stays shown -- beside Settings in split screen, or by `adb
     * shell settings put` -- with no resume to read them: a snackbar
     * opened after it still got the old time (ux review of 761d72b3, N3).
     * The AccessibilityManager learns a timeout from the system on its
     * own path, which may land after this notice, so the value is read
     * again a moment later; an unchanged one sends nothing.
     */
    private val settingsObserver =
        object : ContentObserver(Handler(Looper.getMainLooper())) {
            override fun onChange(selfChange: Boolean) {
                touch()
                window.decorView.postDelayed({ if (shown) touch() }, SETTLE_MS)
            }
        }

    private fun watchSettings(on: Boolean) {
        if (!on) {
            contentResolver.unregisterContentObserver(settingsObserver)
            return
        }
        for (uri in listOf(
            Settings.Secure.getUriFor(INTERACTIVE_UI_TIMEOUT),
            Settings.Secure.getUriFor(NON_INTERACTIVE_UI_TIMEOUT),
            Settings.Global.getUriFor(Settings.Global.ANIMATOR_DURATION_SCALE),
            Settings.System.getUriFor(Settings.System.TIME_12_24),
        )) {
            contentResolver.registerContentObserver(uri, false, settingsObserver)
        }
    }

    /** A rotation or a window resize moves the gesture insets. */
    private val layout = ViewTreeObserver.OnGlobalLayoutListener { touch() }

    /**
     * Hand the swipe its platform values: the touch slop and the system
     * gesture insets (the back-gesture zones) in dp, the layout's
     * direction, whether animations are off, the snackbar's recommended
     * time for content with a control, and the 12- or 24-hour setting
     * every shown time follows. Sent only when they
     * change.
     */
    private fun touch() {
        val density = resources.displayMetrics.density
        val insets = window.decorView.rootWindowInsets?.getInsets(WindowInsets.Type.systemGestures())
        val accessibility = getSystemService(AccessibilityManager::class.java)
        val values =
            Touch(
                slop = ViewConfiguration.get(this).scaledTouchSlop / density,
                edgeLeft = (insets?.left ?: 0) / density,
                edgeRight = (insets?.right ?: 0) / density,
                rtl = resources.configuration.layoutDirection == View.LAYOUT_DIRECTION_RTL,
                reducedMotion =
                    Settings.Global.getFloat(contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f,
                recommendedMs =
                    (
                        accessibility?.getRecommendedTimeoutMillis(
                            SNACKBAR_MS,
                            AccessibilityManager.FLAG_CONTENT_TEXT or AccessibilityManager.FLAG_CONTENT_CONTROLS,
                        ) ?: SNACKBAR_MS
                    ).toLong(),
                hour24 = DateFormat.is24HourFormat(this),
                // A message's menu opens after the platform's long-press time.
                longPressMs = ViewConfiguration.getLongPressTimeout().toLong(),
            )
        if (values == sentTouch) return
        sentTouch = values
        Native.touch(
            values.slop,
            values.edgeLeft,
            values.edgeRight,
            values.rtl,
            values.reducedMotion,
            values.recommendedMs,
            values.hour24,
            values.longPressMs,
        )
    }

    /** The touch values as last sent. */
    private data class Touch(
        val slop: Float,
        val edgeLeft: Float,
        val edgeRight: Float,
        val rtl: Boolean,
        val reducedMotion: Boolean,
        val recommendedMs: Long,
        val hour24: Boolean,
        val longPressMs: Long,
    )

    private var sentTouch: Touch? = null

    /** A swipe armed: the platform's tick, once per crossing. */
    private fun relayHaptics() {
        while (shown) {
            if (Native.waitHaptic(ASK_WAIT_MS)) runOnUiThread { front?.tick() }
        }
    }

    private fun tick() {
        val tick =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                HapticFeedbackConstants.GESTURE_THRESHOLD_ACTIVATE
            } else {
                HapticFeedbackConstants.CONTEXT_CLICK
            }
        window.decorView.performHapticFeedback(tick)
    }

    /**
     * Back below API 33, where no callback is registered. Lint's
     * GestureBackNavigation asks for AndroidX's dispatcher, which a
     * NativeActivity has not; on API 33+ the callback below takes Back,
     * gestures included, and below 33 there is no predictive gesture for
     * this to miss.
     */
    @Deprecated("The platform's own deprecation; API 33+ uses the callback")
    @SuppressLint("GestureBackNavigation")
    override fun onBackPressed() {
        goBack()
    }

    /**
     * Back on API 33+, to the window. Lazy: the callback's class exists
     * from API 33 only, and is made only inside the version checks.
     */
    private val back by lazy { OnBackInvokedCallback { goBack() } }

    /**
     * Back closes an open keyboard first, as the platform does, and only
     * then goes to the window: with the keyboard up the conversation may
     * have set its app bar aside, and Back would otherwise leave it
     * (#304 F9). Focus stays where it is.
     */
    private fun goBack() {
        if (window.decorView.rootWindowInsets?.isVisible(WindowInsets.Type.ime()) == true) {
            window.insetsController?.hide(WindowInsets.Type.ime())
            return
        }
        Native.back()
    }

    /** Back found no way back: leave, as the platform would. */
    private fun relayLeave() {
        while (shown) {
            if (Native.waitLeave(ASK_WAIT_MS)) runOnUiThread { front?.finish() }
        }
    }

    /**
     * The window's flags for the span a word can show: FLAG_SECURE, and on
     * API 33+ no Recents screenshot either (android-key-custody.md, the
     * recovery bullet, 80594810). On the main thread.
     */
    private fun secure(on: Boolean) {
        if (on) {
            window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        } else {
            window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) setRecentsScreenshotEnabled(!on)
    }

    /** What the identity screens ask, carried out on the main thread. */
    private fun relayIdentity() {
        while (shown) {
            val packed = Native.waitIdentityAsk(ASK_WAIT_MS)
            val ask = (packed and ASK_MASK).toInt()
            val arg = packed ushr Ask.BITS
            // With no instance started, this one carries it out, as the
            // last that was.
            if (ask != 0) runOnUiThread { (front ?: this).identityAsk(ask, arg) }
        }
    }

    /** One of the identity screens' asks, on the main thread. */
    private fun identityAsk(ask: Int, arg: Long) {
        when (ask) {
            // FLAG_SECURE goes on before any word can be drawn (onStart,
            // and OFFLINE below), and off only once the window drew a
            // frame without the flow: no screenshot, no recording, no
            // recents thumbnail (android-key-custody.md, #309 F3).
            Ask.SECURE_OFF -> secure(false)
            Ask.PROMPT -> prompt(Text.PHRASE_PROMPT_TITLE)
            Ask.UNLOCK -> prompt(Text.UNLOCK_PROMPT_TITLE)
            Ask.BIND -> {
                identityReady = true
                connect()
            }
            Ask.FINISH -> finish()
            Ask.OFFLINE -> {
                phraseShown = true
                // The flag first: S2's words are drawn only after
                // Native.offline, which runs after it.
                secure(true)
                goOffline()
            }
            Ask.ONLINE -> {
                phraseShown = false
                // Unbound since (Home on S2): bind as a launch does.
                local?.online() ?: connect()
            }
            Ask.RESTART -> local?.restart() ?: connect()
            // Scan a code (add-a-peer 5.5): the camera permission is asked
            // here, on a Scan press, when the app does not hold it.
            Ask.CAMERA_START -> {
                // An ask relayed after the stop that closed the viewfinder
                // opens nothing behind the app (#418 round 2).
                if (!shown) {
                    Native.cameraClosed()
                } else if (checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
                    cameraWanted = true
                    cameraHeld()
                    openCamera()
                } else {
                    cameraWanted = true
                    requestPermissions(arrayOf(Manifest.permission.CAMERA), CAMERA_REQUEST)
                }
            }
            Ask.CAMERA_STOP -> {
                cameraWanted = false
                closeCamera()
            }
            Ask.OPEN_SETTINGS -> {
                inSettings = true
                startActivity(
                    Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", packageName, null)),
                )
            }
            Ask.PICK_PHOTO -> pickPhoto(arg)
            Ask.PHOTO_CANCEL -> {
                cancelPhotoOpen()
                photoRequest = -1L
            }
            // Show my address: the flag before its addresses are drawn,
            // which the window does once told (add-a-peer section 5.4).
            Ask.SHOW_SECURE -> {
                secure(true)
                Native.showSecured()
            }
        }
    }

    /**
     * The device credential or a strong biometric, whatever the profile's
     * unlock policy, before the phrase is shown on demand (ADR-0042, the
     * custody doc). A cancel or an error shows nothing.
     */
    private fun prompt(title: Int) {
        BiometricPrompt.Builder(this)
            .setTitle(Native.text(title, 0))
            .setAllowedAuthenticators(Authenticators.BIOMETRIC_STRONG or Authenticators.DEVICE_CREDENTIAL)
            .build()
            .authenticate(
                CancellationSignal(),
                mainExecutor,
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult?) {
                        if (title == Text.UNLOCK_PROMPT_TITLE) unlockDeclined = false
                        Native.prompted(true)
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence?) {
                        // A cancel, or Home while it showed: either way, declined.
                        if (title == Text.UNLOCK_PROMPT_TITLE) unlockDeclined = true
                        Native.prompted(false)
                    }
                },
            )
    }

    /** The window's "Allow network access", carried out on the main thread. */
    private fun relayAsks() {
        while (shown) {
            if (Native.waitNetworkAsk(ASK_WAIT_MS)) runOnUiThread { front?.ask() }
        }
    }

    /** Whether the platform's prompt for network access was shown before. */
    private fun askedNetwork(): Boolean =
        getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE).getBoolean(ASKED_NETWORK, false)

    /**
     * The platform's prompt while it still offers one: before the first
     * ask, or while it says a rationale may be shown. After a refusal it
     * stops offering it, and only the settings page can grant.
     */
    private fun ask() {
        if (!askedNetwork() || shouldShowRequestPermissionRationale(Manifest.permission.INTERNET)) {
            getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE)
                .edit()
                .putBoolean(ASKED_NETWORK, true)
                .apply()
            requestPermissions(arrayOf(Manifest.permission.INTERNET), NETWORK_REQUEST)
        } else {
            if (!bound) bind()
            startActivity(
                Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", packageName, null)),
            )
        }
    }

    /**
     * API 33+: a notification is posted only with the person's leave.
     * Below it the permission does not exist and notifications are on;
     * asking anyway opened an empty system activity over the window at
     * every launch (the A40 at API 30, #436 round 4).
     */
    private fun askNotifications() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
        if (checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), NOTIFICATIONS_REQUEST)
        }
    }

    private fun networkGranted(): Boolean =
        Build.VERSION.SDK_INT < RUNTIME_INTERNET_SDK ||
            checkSelfPermission(Manifest.permission.INTERNET) == PackageManager.PERMISSION_GRANTED

    private fun bind() {
        bound = applicationContext.bindService(
            Intent(applicationContext, NetworkService::class.java),
            connection,
            Context.BIND_AUTO_CREATE,
        )
    }

    private fun openCamera() {
        val c = camera ?: CodeCamera(this, ::cameraLost).also { camera = it }
        c.open()
    }

    /**
     * A camera went away under the scan. Told to the window from the main
     * thread, and only for the camera this Activity holds now: one closed
     * since, whose failure arrives late, ends no newer scan.
     */
    private fun cameraLost(lost: CodeCamera) {
        runOnUiThread {
            if (camera === lost) {
                camera = null
                Native.cameraClosed()
            }
        }
    }

    /** CAMERA is held: the last answer was no refusal (#418 F7). */
    private fun cameraHeld() {
        val prefs = getPreferences(Context.MODE_PRIVATE)
        if (prefs.contains(CAMERA_REFUSED)) prefs.edit().remove(CAMERA_REFUSED).apply()
    }

    private fun closeCamera() {
        camera?.close()
        camera = null
    }

    companion object {
        /** The first API level where INTERNET is a runtime permission (j59, measured on API 37). */
        private const val RUNTIME_INTERNET_SDK = 37
        private const val NETWORK_REQUEST = 2
        private const val CAMERA_REQUEST = 3
        private const val PHOTO_REQUEST = 4
        /** MediaStore.ACTION_PICK_IMAGES, by its value: an API 33 constant also served by the R extension 2. */
        private const val PICK_IMAGES = "android.provider.action.PICK_IMAGES"
        /** How long a pick may take, its open and its read together: src/photo_import.rs's PHOTO_DEADLINE. */
        private const val PHOTO_DEADLINE_MS = 30_000L
        /** The opens a provider may hold past their cancel before a pick is refused (j68 P3-1). */
        private const val MAX_PHOTO_OPENS = 2
        private val photoOpens = java.util.concurrent.atomic.AtomicInteger()
        private const val ASK_MASK = (1L shl Ask.BITS) - 1
        /** The last CAMERA answer was a refusal: the next refusal the system will not ask again is "for good". */
        private const val CAMERA_REFUSED = "camera-refused"
        private const val NOTIFICATIONS_REQUEST = 1
        private const val UNLOCK_DECLINED = "unlock-declined"
        private const val UNLOCK_DECLINED_IN = "unlock-declined-in"
        /** This process's, minted once: tells a same-process recreation from a restore after a death. */
        private val PROCESS_TOKEN: String = java.util.UUID.randomUUID().toString()
        private const val ASK_WAIT_MS = 500L

        /** The snackbar's own time up, before a screen reader's setting lengthens it. */
        private const val SNACKBAR_MS = 6_000

        // The settings behind "Time to take action" and its companion
        // (Settings.Secure keys since API 29, not public constants).
        private const val INTERACTIVE_UI_TIMEOUT = "accessibility_interactive_ui_timeout_ms"
        private const val NON_INTERACTIVE_UI_TIMEOUT = "accessibility_non_interactive_ui_timeout_ms"
        private const val SETTLE_MS = 500L
        private const val PREFERENCES = "interweave.window"
        private const val ASKED_NETWORK = "asked_network"

        // The binding is the application's, not an Activity instance's,
        // so it can outlive one instance and pass to the next.
        private var bound = false

        // The identity opened, or the identity screens finished: the
        // service may be bound. The process's, as the binding is.
        @Volatile private var identityReady = false
        @Volatile private var local: NetworkService.Local? = null

        // The instances between onStart and onStop: the last to stop lets
        // the binding go. The main thread's.
        private var started = 0

        // The instance most recently started and not yet stopped: the one
        // the window shows in, and the one each relay's ask is carried out
        // on. One queue serves every instance, and a stopping instance's
        // relay can take an ask for up to ASK_WAIT_MS after its onStop:
        // after a recreation, the old one's relay can take an ask meant
        // for the new window. singleTask (the manifest) removed the stacked
        // instances, where a Back started the one below before the closing
        // one stopped; the overlap above remains. Run on the instance
        // whose thread took it, a SECURE_OFF could clear the closing
        // window's flag and leave the shown one's set, and an OFFLINE could
        // set the flag on a window the words are not drawn in. The main
        // thread's.
        private var front: MainActivity? = null

        // Show my phrase asked while the service was not bound yet: it is
        // stopped as soon as the binding arrives. The main thread's.
        private var offlineOnConnect = false
        private val connection =
            object : ServiceConnection {
                override fun onServiceConnected(name: ComponentName?, service: IBinder?) {
                    local = service as? NetworkService.Local
                    if (offlineOnConnect) {
                        offlineOnConnect = false
                        local?.offline { Native.offline(it) } ?: Native.offline(false)
                    }
                }

                override fun onServiceDisconnected(name: ComponentName?) {
                    local = null
                }
            }
    }
}
