// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
package org.interweave.human

import android.Manifest
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log

/**
 * The debug build's test hooks, fired from adb (unlock note section 8: a
 * refusal the phone cannot produce on demand is stood in for by a hook,
 * named in the run's record). The release build's DebugHooks registers
 * nothing; this source set is the debug variant's alone.
 *
 * [RESTART] runs the start a platform restart under Stay reachable runs,
 * in the running process:
 *
 *     adb shell am broadcast -a org.interweave.human.debug.RESTART -p org.interweave.human
 *
 * [REFUSE_STORE] makes the next start refuse the store as one that needs
 * recovery, once, with its files left as they are (A16's discard): fire
 * it, then a start -- [RESTART], or a launch -- and the recovery screen
 * shows; its discard is a real one.
 *
 *     adb shell am broadcast -a org.interweave.human.debug.REFUSE_STORE -p org.interweave.human
 *
 * The Service registers the receiver while it runs, asking the sender for
 * DUMP, which adb's shell holds.
 */
internal object DebugHooks {
    const val RESTART = "org.interweave.human.debug.RESTART"
    const val REFUSE_STORE = "org.interweave.human.debug.REFUSE_STORE"
    private const val TAG = "interweave"

    /** The native side of [REFUSE_STORE]: the debug library's alone. */
    @JvmStatic private external fun refuseNextStoreOpen()

    /** Hear the hooks on the main thread while [service] runs; the unregistration. */
    fun register(service: Service, restart: () -> Unit): () -> Unit {
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent) {
                when (intent.action) {
                    RESTART -> restart()
                    REFUSE_STORE -> refuseNextStoreOpen()
                }
            }
        }
        val filter = IntentFilter(RESTART).apply { addAction(REFUSE_STORE) }
        val main = Handler(Looper.getMainLooper())
        // Exported, since adb's shell is another uid; DUMP is a
        // signature-level permission, which the shell holds.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            service.registerReceiver(receiver, filter, Manifest.permission.DUMP, main, Context.RECEIVER_EXPORTED)
        } else {
            service.registerReceiver(receiver, filter, Manifest.permission.DUMP, main)
        }
        Log.i(TAG, "debug hooks: listening for $RESTART and $REFUSE_STORE")
        return { service.unregisterReceiver(receiver) }
    }
}
