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

    override fun onStart() {
        super.onStart()
        shown = true
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
        if (!identityReady) {
            // The identity screens come before the service: a profile with
            // no identity, or one the phone lost, binds nothing until the
            // screens say the identity is ready (onboard.rs).
            if (Native.identityState(applicationInfo.dataDir) != Native.IDENTITY_READY) return
            identityReady = true
        }
        if (!phraseShown) connect()
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
    }

    override fun onDestroy() {
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
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            onBackInvokedDispatcher.unregisterOnBackInvokedCallback(back)
        }
        window.decorView.viewTreeObserver.removeOnGlobalLayoutListener(layout)
        watchSettings(false)
        // A recreation for a configuration change keeps the binding: the
        // next instance finds it held, and foreground-only mode's runtime
        // and lease survive it (review F2). Any other stop lets go.
        if (bound && !isChangingConfigurations) {
            applicationContext.unbindService(connection)
            bound = false
            local = null
        }
        super.onStop()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
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
            if (Native.waitHaptic(ASK_WAIT_MS) && shown) {
                runOnUiThread {
                    val tick =
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                            HapticFeedbackConstants.GESTURE_THRESHOLD_ACTIVATE
                        } else {
                            HapticFeedbackConstants.CONTEXT_CLICK
                        }
                    window.decorView.performHapticFeedback(tick)
                }
            }
        }
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
            if (Native.waitLeave(ASK_WAIT_MS) && shown) runOnUiThread { finish() }
        }
    }

    /** What the identity screens ask, carried out on the main thread. */
    private fun relayIdentity() {
        while (shown) {
            when (Native.waitIdentityAsk(ASK_WAIT_MS)) {
                // FLAG_SECURE for as long as a word is on screen: no
                // screenshot, no recents thumbnail (android-key-custody.md).
                Ask.SECURE_ON -> runOnUiThread { window.addFlags(WindowManager.LayoutParams.FLAG_SECURE) }
                Ask.SECURE_OFF -> runOnUiThread { window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE) }
                Ask.PROMPT -> runOnUiThread { prompt() }
                Ask.BIND -> {
                    identityReady = true
                    runOnUiThread { connect() }
                }
                Ask.FINISH -> runOnUiThread { finish() }
                // Show my phrase: with no service bound, none runs.
                Ask.OFFLINE -> {
                    phraseShown = true
                    local?.offline { Native.offline(it) } ?: Native.offline(true)
                }
                Ask.ONLINE -> {
                    phraseShown = false
                    // Unbound since (Home on S2): bind as a launch does.
                    runOnUiThread { local?.online() ?: connect() }
                }
            }
        }
    }

    /**
     * The device credential or a strong biometric, whatever the profile's
     * unlock policy, before the phrase is shown on demand (ADR-0042, the
     * custody doc). A cancel or an error shows nothing.
     */
    private fun prompt() {
        BiometricPrompt.Builder(this)
            .setTitle(Native.text(Text.PHRASE_PROMPT_TITLE, 0))
            .setAllowedAuthenticators(Authenticators.BIOMETRIC_STRONG or Authenticators.DEVICE_CREDENTIAL)
            .build()
            .authenticate(
                CancellationSignal(),
                mainExecutor,
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult?) {
                        Native.prompted(true)
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence?) {
                        Native.prompted(false)
                    }
                },
            )
    }

    /** The window's "Allow network access", carried out on the main thread. */
    private fun relayAsks() {
        while (shown) {
            if (Native.waitNetworkAsk(ASK_WAIT_MS) && shown) runOnUiThread { ask() }
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

    /** API 33+: a notification is posted only with the person's leave. */
    private fun askNotifications() {
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

    companion object {
        /** The first API level where INTERNET is a runtime permission (j59, measured on API 37). */
        private const val RUNTIME_INTERNET_SDK = 37
        private const val NETWORK_REQUEST = 2
        private const val NOTIFICATIONS_REQUEST = 1
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
        private val connection =
            object : ServiceConnection {
                override fun onServiceConnected(name: ComponentName?, service: IBinder?) {
                    local = service as? NetworkService.Local
                }

                override fun onServiceDisconnected(name: ComponentName?) {
                    local = null
                }
            }
    }
}
