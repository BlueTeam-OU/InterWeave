// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
package org.interweave.human

import android.Manifest
import android.app.NativeActivity
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.IBinder
import android.provider.Settings

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

    override fun onStart() {
        super.onStart()
        shown = true
        Thread({ relayAsks() }, "interweave-network-ask").start()
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
        // Back from the prompt or the settings page: a grant restarts the
        // Service with its runtime.
        local?.recheck()
    }

    override fun onStop() {
        shown = false
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

    /** The window's "Allow network access", carried out on the main thread. */
    private fun relayAsks() {
        while (shown) {
            if (Native.waitNetworkAsk(ASK_WAIT_MS) && shown) runOnUiThread { ask() }
        }
    }

    /**
     * The platform's prompt while it still offers one: before the first
     * ask, or while it says a rationale may be shown. After a refusal it
     * stops offering it, and only the settings page can grant.
     */
    private fun askedNetwork(): Boolean =
        getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE).getBoolean(ASKED_NETWORK, false)

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
        private const val PREFERENCES = "interweave.window"
        private const val ASKED_NETWORK = "asked_network"

        // The binding is the application's, not an Activity instance's,
        // so it can outlive one instance and pass to the next.
        private var bound = false
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
