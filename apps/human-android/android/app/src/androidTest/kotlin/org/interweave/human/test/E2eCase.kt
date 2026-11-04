// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
package org.interweave.human.test

import android.app.Activity
import android.app.Instrumentation
import android.os.Build
import android.os.Bundle
import android.security.keystore.KeyInfo
import android.util.Base64
import java.security.KeyStore
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import org.json.JSONArray
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
                when (case) {
                    DROP_STORE_KEY -> dropKey(DROP_STORE_KEY, STORE_KEY_ALIAS)
                    DROP_SEED_KEY -> dropKey(DROP_SEED_KEY, SEED_KEY_ALIAS)
                    KEY_INFO -> keyInfo()
                    else -> JSONObject(run(case, args, targetContext.dataDir.absolutePath))
                }
            } catch (e: Throwable) {
                JSONObject().put("case", case).put("result", "fail").put("detail", "the runner: $e")
            }
        for (key in out.keys()) status.putString("interweave.$key", out.get(key).toString())
        sendStatus(0, status)
        finish(Activity.RESULT_OK, status)
    }

    /**
     * A device row's precondition, not a case: the Keystore entry at
     * [alias] deleted, as a lost or wiped Keystore leaves it. The store
     * key's ([DROP_STORE_KEY]) makes the next start show the store's
     * recovery screen, whose discard a person runs (j138's device rows);
     * the seed's ([DROP_SEED_KEY]) makes it open on K1 (TM-48's case).
     * Destroys what the key opened on the phone it runs on; runs only when
     * asked by name.
     */
    private fun dropKey(case: String, alias: String): JSONObject {
        val answer = org.interweave.human.KeystoreCipher.retire(alias)
        return JSONObject()
            .put("case", case)
            .put("result", if (answer.firstOrNull() == 0.toByte()) "pass" else "fail")
            .put("detail", "retire $alias answered ${answer.firstOrNull()}")
    }

    /**
     * A device row's reading, not a case: every `interweave.` alias in the
     * app's `AndroidKeyStore` -- the seed's, the store's, each identity's --
     * with what its `KeyInfo` says of where the key lives and what can
     * invalidate it (TM-54's and SPIKE-011's device rows). Changes nothing
     * and reads no key material. A key the Keystore refuses to hand over
     * is listed with the refusal's class, which is the invalidation row's
     * evidence.
     */
    private fun keyInfo(): JSONObject {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        val keys = JSONArray()
        for (alias in store.aliases().toList().filter { it.startsWith("interweave.") }.sorted()) {
            val entry = JSONObject().put("alias", alias)
            try {
                val key = store.getKey(alias, null) as SecretKey
                val factory = SecretKeyFactory.getInstance(key.algorithm, "AndroidKeyStore")
                val info = factory.getKeySpec(key, KeyInfo::class.java) as KeyInfo
                @Suppress("DEPRECATION")
                entry
                    .put("inside_secure_hardware", info.isInsideSecureHardware)
                    .put("user_authentication_required", info.isUserAuthenticationRequired)
                    .put("invalidated_by_biometric_enrollment", info.isInvalidatedByBiometricEnrollment)
                    .put("key_size", info.keySize)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) entry.put("security_level", info.securityLevel)
            } catch (e: Exception) {
                entry.put("refused", e.javaClass.name)
            }
            keys.put(entry)
        }
        return JSONObject()
            .put("case", KEY_INFO)
            .put("result", "pass")
            .put("api", Build.VERSION.SDK_INT)
            .put("keys", keys)
    }

    companion object {
        const val DROP_STORE_KEY = "drop-store-key"
        const val DROP_SEED_KEY = "drop-seed-key"
        const val KEY_INFO = "key-info"

        /** keystore.rs: the store's own alias, `interweave.store.<profile>`. */
        const val STORE_KEY_ALIAS = "interweave.store.human-android"

        /**
         * custody.rs `KeyRef::alias`: the seed's, at the default
         * `background-compatible` policy, `interweave.iwk1.<profile>.<policy>`.
         */
        const val SEED_KEY_ALIAS = "interweave.iwk1.human-android.background-compatible"

        init {
            System.loadLibrary("interweave_human_android")
        }

        /** The case's result JSON (src/e2e.rs). BLOCKS for the case's length. */
        @JvmStatic external fun run(case: String, args: String, appDataDir: String): String
    }
}
