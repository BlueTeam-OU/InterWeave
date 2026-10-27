// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
package org.interweave.human.test

import android.app.Activity
import android.app.Instrumentation
import android.os.Bundle
import android.util.Base64
import org.json.JSONObject

/**
 * The android-e2e cases' runner (tests/android-e2e's src/adb.rs): one
 * case per `am instrument -w -r -e case <name> -e args <base64 JSON>`,
 * run in the app's own process by the native library's debug-only entry
 * (apps/human-android/src/e2e.rs). Each key of the result JSON comes back
 * as the status `interweave.<key>` (the seam agreed with
 * p2p-network-dev-02, 01a12771). No test framework: nothing beyond the
 * platform is on the androidTest classpath.
 */
class E2eCase : Instrumentation() {
    private var case = ""
    private var args = "{}"

    override fun onCreate(arguments: Bundle?) {
        super.onCreate(arguments)
        case = arguments?.getString("case").orEmpty()
        args = arguments?.getString("args")?.let { String(Base64.decode(it, Base64.DEFAULT), Charsets.UTF_8) } ?: "{}"
        start()
    }

    override fun onStart() {
        super.onStart()
        val status = Bundle()
        val out =
            try {
                if (case == DROP_STORE_KEY) dropStoreKey()
                else JSONObject(run(case, args, targetContext.dataDir.absolutePath))
            } catch (e: Throwable) {
                JSONObject().put("case", case).put("result", "fail").put("detail", "the runner: $e")
            }
        for (key in out.keys()) status.putString("interweave.$key", out.get(key).toString())
        sendStatus(0, status)
        finish(Activity.RESULT_OK, status)
    }

    /**
     * A device row's precondition, not a case: the store key's Keystore
     * entry deleted, as a lost or wiped Keystore leaves it, so the next
     * start shows the recovery screen and its discard can be run by a
     * person (j138's device rows). Destroys the store's readability on
     * the phone it runs on; runs only when asked by name.
     */
    private fun dropStoreKey(): JSONObject {
        val answer = org.interweave.human.KeystoreCipher.retire(STORE_KEY_ALIAS)
        return JSONObject()
            .put("case", DROP_STORE_KEY)
            .put("result", if (answer.firstOrNull() == 0.toByte()) "pass" else "fail")
            .put("detail", "retire $STORE_KEY_ALIAS answered ${answer.firstOrNull()}")
    }

    companion object {
        const val DROP_STORE_KEY = "drop-store-key"

        /** keystore.rs: the store's own alias, `interweave.store.<profile>`. */
        const val STORE_KEY_ALIAS = "interweave.store.human-android"

        init {
            System.loadLibrary("interweave_human_android")
        }

        /** The case's result JSON (src/e2e.rs). BLOCKS for the case's length. */
        @JvmStatic external fun run(case: String, args: String, appDataDir: String): String
    }
}
