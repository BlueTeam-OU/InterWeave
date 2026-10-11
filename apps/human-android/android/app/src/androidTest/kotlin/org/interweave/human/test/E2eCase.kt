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
                JSONObject(run(case, args, targetContext.dataDir.absolutePath))
            } catch (e: Throwable) {
                JSONObject().put("case", case).put("result", "fail").put("detail", "the runner: $e")
            }
        for (key in out.keys()) status.putString("interweave.$key", out.get(key).toString())
        sendStatus(0, status)
        finish(Activity.RESULT_OK, status)
    }

    companion object {
        init {
            System.loadLibrary("interweave_human_android")
        }

        /** The case's result JSON (src/e2e.rs). BLOCKS for the case's length. */
        @JvmStatic external fun run(case: String, args: String, appDataDir: String): String
    }
}
