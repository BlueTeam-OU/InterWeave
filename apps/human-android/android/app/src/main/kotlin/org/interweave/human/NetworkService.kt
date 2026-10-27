// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
package org.interweave.human

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.net.ConnectivityManager
import android.net.LinkProperties
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.Binder
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.os.IBinder
import android.os.Looper
import android.util.Log
import java.io.ByteArrayOutputStream
import java.net.InetAddress
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

/**
 * The network service: it owns the embedded runtime, the store and the
 * session's endpoint lease (human-client-android.md, "Service ownership
 * and local session"). An Activity destroyed or rotated keeps them; the
 * Service's end releases them.
 *
 * The Activity binds it while it is shown. In foreground-only mode that
 * binding is all that keeps it, so the runtime stops when the app leaves
 * the screen. When the profile's effective availability is Stay
 * reachable, the Service also starts itself as a remoteMessaging
 * foreground service with its notification (ADR-0041), and outlives the
 * Activity until the person turns it off or the platform stops it.
 */
class NetworkService : Service() {
    private val binder = Local()
    private val main = Handler(Looper.getMainLooper())
    @Volatile private var alive = false
    @Volatile private var relaying = false
    // UNANSWERED until the start answers: no answer, refusal or mode, is
    // ever this value (review of #255: -1 was also NO_IDENTITY).
    @Volatile private var mode = UNANSWERED
    // A start command reached this Service: only Stay reachable sends one,
    // so a posture answer then is step 5's broken promise.
    @Volatile private var startCommanded = false
    // Show my phrase holds the runtime stopped while the words are shown
    // (design/2026-10-19-android-identity.md section 2): nothing here
    // starts it again until the window says the words are gone.
    @Volatile private var held = false
    private val networkOffPosted = AtomicBoolean(false)
    // The network callback's own thread: its reports, and the
    // unregistration's, reach the runtime in the order they were made.
    private val networkThread = HandlerThread("interweave-network")
    private val networks = Networks()
    private val defaultNetwork = DefaultNetwork()

    /** What the bound Activity calls on the Service. */
    inner class Local : Binder() {
        /**
         * The person came back to the window, perhaps from the permission
         * prompt or the settings page: if the posture runs and network
         * access is now held, start again as the app does.
         */
        @Synchronized
        fun recheck() {
            // Claimed once: a grant arrives as a permission result and a
            // resume together, and two restarts would let the first
            // start's end-waiter stop the second runtime. `mode` leaves
            // NETWORK_DENIED here, before the restart is queued, and the
            // start writes its own answer.
            if (!held && mode == Native.NETWORK_DENIED && networkGranted()) {
                mode = UNANSWERED
                Log.i(TAG, "network access granted: restarting with the runtime")
                lifecycle.execute { Native.stop() }
                startClient()
            }
        }

        /**
         * Show my phrase: stop the runtime and keep it stopped until
         * [online]; [done] hears, off the main thread, whether it stopped.
         * The Stay-reachable notification goes with it: nothing is
         * reachable while the words show.
         */
        @Synchronized
        fun offline(done: (Boolean) -> Unit) {
            held = true
            lifecycle.execute {
                val stopped = Native.stop()
                main.post { leaveForeground() }
                done(stopped)
            }
        }

        /**
         * The store's screens asked: start the client again, as a launch
         * does -- after Try again, or a discard that recreates the store.
         * Not while Show my phrase holds the service offline.
         */
        @Synchronized
        fun restart() {
            if (held) return
            mode = UNANSWERED
            startClient()
        }

        /** The words are gone: start again, as a launch does. */
        @Synchronized
        fun online() {
            if (!held) return
            held = false
            mode = UNANSWERED
            startClient()
        }
    }

    override fun onCreate() {
        super.onCreate()
        alive = true
        channels()
        watchNetworks()
        startClient()
    }

    /**
     * Report the platform's networks to the runtime (plan section 20 step
     * 5): registered before the start, so the runtime hears them at its
     * start, and a restarted runtime too, though the callback reports
     * each network only once (android-platform's `network`).
     */
    private fun watchNetworks() {
        networkThread.start()
        val handler = Handler(networkThread.looper)
        // Every network the platform reports to this app, with INTERNET or
        // without: an address of this host outside the view counts as on
        // no local network (ADR-0052 rule 10, A 2026-10-27), so a LAN the
        // request dropped would be hidden. What the INTERNET removal
        // admits, measured on the Pixel 9a (2026-10-27): not a Wi-Fi with
        // no uplink -- one carries INTERNET, unvalidated, and was heard
        // before -- but a network the platform marks LOCAL_NETWORK, which
        // has no INTERNET: this phone's own hotspot, reported to an app
        // only from target SDK 37 (ENABLE_MATCH_NON_THREAD_LOCAL_NETWORKS);
        // below it that address is rule 10's recorded limit. A VPN
        // included: the default request leaves it out. Restricted and
        // untrusted networks stay out, as the platform keeps them from an
        // app, and a background network is never heard (the platform adds
        // FOREGROUND to an app's listen).
        val request = NetworkRequest.Builder()
            .removeCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
            .removeCapability(NetworkCapabilities.NET_CAPABILITY_NOT_VPN)
            .build()
        handler.post { Native.networkWatching() }
        val connectivity = getSystemService(ConnectivityManager::class.java)
        connectivity.registerNetworkCallback(request, networks, handler)
        // Which network is the default: its resolvers and Private DNS are
        // the ones the runtime takes (CONNECTIVITY.md section 14).
        connectivity.registerDefaultNetworkCallback(defaultNetwork, handler)
    }

    /** The callback, unregistered; its last report, then the thread, end. */
    private fun unwatchNetworks() {
        val connectivity = getSystemService(ConnectivityManager::class.java)
        connectivity.unregisterNetworkCallback(networks)
        connectivity.unregisterNetworkCallback(defaultNetwork)
        Handler(networkThread.looper).post {
            Native.networkUnwatched()
            networkThread.quitSafely()
        }
    }

    /**
     * Each network's addresses and transports as it reports them, and its
     * loss. What the transports mean is android-platform's to decide.
     */
    private class Networks : ConnectivityManager.NetworkCallback() {
        // The platform calls this after onAvailable, before the first
        // onLinkPropertiesChanged, and again whenever the capabilities
        // change.
        override fun onCapabilitiesChanged(network: Network, capabilities: NetworkCapabilities) {
            Native.networkTransports(
                network.networkHandle,
                capabilities.hasTransport(NetworkCapabilities.TRANSPORT_VPN),
                capabilities.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR),
                // Named from API 35; no older phone has one.
                Build.VERSION.SDK_INT >= Build.VERSION_CODES.VANILLA_ICE_CREAM &&
                    capabilities.hasTransport(NetworkCapabilities.TRANSPORT_SATELLITE),
            )
        }

        // The platform calls this right after the first
        // onCapabilitiesChanged, and on every change of the network's
        // addresses: the whole set each time.
        override fun onLinkPropertiesChanged(network: Network, link: LinkProperties) {
            Native.networkLink(
                network.networkHandle,
                wire(link.linkAddresses.map { it.address }),
                wire(link.dnsServers),
                // The platform names a host in strict mode; opportunistic
                // mode has none.
                if (link.isPrivateDnsActive) link.privateDnsServerName else null,
            )
        }

        /** Each address its length, then its bytes (`addresses_from_wire`). */
        private fun wire(addresses: List<InetAddress>): ByteArray {
            val wire = ByteArrayOutputStream()
            for (address in addresses) {
                val bytes = address.address
                wire.write(bytes.size)
                wire.write(bytes)
            }
            return wire.toByteArray()
        }

        override fun onLost(network: Network) {
            Native.networkLost(network.networkHandle)
        }
    }

    /** Which network is the default, and its going. */
    private class DefaultNetwork : ConnectivityManager.NetworkCallback() {
        override fun onAvailable(network: Network) {
            Native.networkDefault(network.networkHandle)
        }

        override fun onLost(network: Network) {
            Native.networkDefault(0)
        }
    }

    /** Start the client on the lifecycle executor, and act on its answer. */
    private fun startClient() {
        lifecycle.execute {
            // A start that throws counts as a refusal: it must still end
            // the foreground state below, not leave it to nobody.
            // Without network access (API 37+ makes INTERNET a runtime
            // permission) the store still opens: the network-denied
            // posture, and the same for a runtime the platform refused its
            // sockets (human-client-android.md, "Runtime permissions and
            // the network-denied state", steps 3 and 6).
            var answer =
                if (networkGranted()) call("start") { Native.start(dataDir.absolutePath) }
                else Native.REFUSED_NETWORK_DENIED
            if (answer == Native.REFUSED_NETWORK_DENIED) {
                answer = call("startWithoutNetwork") { Native.startWithoutNetwork(dataDir.absolutePath) }
            }
            mode = answer
            Log.i(TAG, "start answered $answer")
            if (answer == Native.NETWORK_DENIED && startCommanded) networkOff()
            if (answer == Native.STAY_REACHABLE) {
                // Started, so that it outlives the Activity's binding.
                startForegroundService(Intent(this, NetworkService::class.java))
            } else {
                // Not Stay reachable, or nothing runs: no foreground service
                // and no "Stay reachable is on" -- a sticky restart posts it
                // before the answer is known (review F1). A bound Activity
                // keeps the Service; nothing else does.
                main.post { leaveForeground() }
            }
            if (answer >= 0) {
                Thread({ awaitEnd() }, "interweave-end").start()
                if (!relaying) {
                    // Once per Service: a restart keeps the one loop.
                    relaying = true
                    Thread({ relayNotices() }, "interweave-notices").start()
                }
            }
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // A start command comes only for Stay reachable; the notification
        // goes up within the platform's limit for a started foreground
        // service, whatever the runtime is doing.
        startForeground(
            ONGOING_ID,
            ongoing(),
            ServiceInfo.FOREGROUND_SERVICE_TYPE_REMOTE_MESSAGING,
        )
        // The start may have answered already, before this command reached
        // the main thread: its withdrawal then ran first and found nothing
        // to withdraw, so withdraw here (review of #255, F1's ordering).
        // The platform requires startForeground once a foreground start was
        // asked, so it goes up first either way.
        startCommanded = true
        val answer = mode
        if (answer == Native.NETWORK_DENIED) networkOff()
        if (answer != UNANSWERED && answer != Native.STAY_REACHABLE) {
            leaveForeground()
            return START_NOT_STICKY
        }
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder = binder

    override fun onDestroy() {
        alive = false
        unwatchNetworks()
        // Off the main thread, and after any start still running: stop
        // waits for the grace, and a start after it finds the lock free.
        lifecycle.execute { Log.i(TAG, "stop answered ${Native.stop()}") }
        super.onDestroy()
    }

    /** [body]'s answer, a throw counting as a refusal that is logged. */
    private inline fun call(what: String, body: () -> Int): Int =
        try {
            body()
        } catch (e: RuntimeException) {
            Log.e(TAG, "$what threw", e)
            START_THREW
        }

    /** Whether the platform gives this app network access. */
    private fun networkGranted(): Boolean =
        checkSelfPermission(Manifest.permission.INTERNET) == PackageManager.PERMISSION_GRANTED

    /**
     * Step 5: Stay reachable restarted without network access. Post ONE
     * notification -- network access is off, open the app -- and end the
     * started state; no retry loop, no silent end.
     */
    private fun networkOff() {
        if (!networkOffPosted.compareAndSet(false, true)) return
        Log.i(TAG, "Stay reachable restarted without network access: one notice, then end")
        getSystemService(NotificationManager::class.java).notify(
            NETWORK_OFF_ID,
            Notification.Builder(this, REACHABLE_CHANNEL)
                .setSmallIcon(R.drawable.ic_stat_interweave)
                .setContentTitle(Native.text(Text.NETWORK_OFF_TITLE, 0))
                .setContentText(Native.text(Text.NETWORK_OFF_BODY, 0))
                .setContentIntent(opening())
                .setAutoCancel(true)
                .build(),
        )
        main.post { leaveForeground() }
    }

    /** Withdraw the Stay-reachable notification and the started state. */
    private fun leaveForeground() {
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    /** The runtime was asked to stop (its admin port, or a stop): end. */
    private fun awaitEnd() {
        val ended = Native.waitEnded()
        // A stop for Show my phrase is offline()'s, and its restart
        // online()'s: this waiter neither stops nor restarts it.
        if (!alive || held) return
        when (ended) {
            Native.ENDED_ASKED, Native.ENDED_RUNTIME -> {
                // Asked to stop, or the runtime ended on its own (not a
                // request, but the same answer): stop it, release what it
                // held, and end the started state; a bound Activity then
                // shows the service as not running.
                Log.i(TAG, if (ended == Native.ENDED_ASKED) "asked to stop" else "the runtime ended on its own")
                lifecycle.execute { Native.stop() }
                main.post { leaveForeground() }
            }
            Native.ENDED_AVAILABILITY -> {
                // The person's Stay-reachable choice moved the mode:
                // restart in it rather than wait for the next start
                // (ADR-0041 A 2026-10-10).
                Log.i(TAG, "the availability changed: restarting")
                lifecycle.execute { Native.stop() }
                startClient()
            }
        }
    }

    /** Post or withdraw the count of messages no window has read. */
    private fun relayNotices() {
        val manager = getSystemService(NotificationManager::class.java)
        while (alive) {
            val count = Native.waitNotice(NOTICE_WAIT_MS)
            when {
                count < 0 -> Unit
                count == 0 -> manager.cancel(MESSAGES_ID)
                else -> manager.notify(MESSAGES_ID, messages(count))
            }
        }
        manager.cancel(MESSAGES_ID)
    }

    private fun channels() {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(
                REACHABLE_CHANNEL,
                Native.text(Text.REACHABLE_CHANNEL, 0),
                NotificationManager.IMPORTANCE_LOW,
            ),
        )
        manager.createNotificationChannel(
            NotificationChannel(
                MESSAGES_CHANNEL,
                Native.text(Text.MESSAGES_CHANNEL, 0),
                NotificationManager.IMPORTANCE_DEFAULT,
            ).apply {
                // A count, never content; private on a locked screen.
                lockscreenVisibility = Notification.VISIBILITY_PRIVATE
            },
        )
    }

    private fun opening(): PendingIntent =
        PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_IMMUTABLE,
        )

    private fun ongoing(): Notification =
        Notification.Builder(this, REACHABLE_CHANNEL)
            .setSmallIcon(R.drawable.ic_stat_interweave)
            .setContentTitle(Native.text(Text.REACHABLE_TITLE, 0))
            .setContentText(Native.text(Text.REACHABLE_BODY, 0))
            .setContentIntent(opening())
            .setOngoing(true)
            .build()

    private fun messages(count: Int): Notification =
        Notification.Builder(this, MESSAGES_CHANNEL)
            .setSmallIcon(R.drawable.ic_stat_interweave)
            .setContentTitle(Native.text(Text.MESSAGES_WAITING, count))
            .setContentIntent(opening())
            .setAutoCancel(true)
            .setVisibility(Notification.VISIBILITY_PRIVATE)
            .build()

    companion object {
        private const val TAG = "interweave"
        private const val REACHABLE_CHANNEL = "reachable"
        private const val MESSAGES_CHANNEL = "messages"
        private const val ONGOING_ID = 1
        private const val MESSAGES_ID = 2
        private const val NETWORK_OFF_ID = 3
        private const val NOTICE_WAIT_MS = 1000L

        /** `mode` before the start answers; never an answer. */
        private const val UNANSWERED = Int.MIN_VALUE

        /** A start that threw, read as a refusal. */
        private const val START_THREW = Int.MIN_VALUE + 1

        /** Start and stop, in order, one at a time, for the process. */
        private val lifecycle: ExecutorService = Executors.newSingleThreadExecutor()
    }
}
