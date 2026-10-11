// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
package org.interweave.human

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import android.security.keystore.UserNotAuthenticatedException
import java.security.KeyStore
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * The platform cipher the embedded custody path seals the profile's seed
 * with (`crates/transport/embedded/src/custody.rs`, `SeedCipher`;
 * android-key-custody.md): an AES-256-GCM key in `AndroidKeyStore`, one
 * per alias, which custody computes and this never derives. The native
 * library calls these three functions (`src/keystore.rs`).
 *
 * Every answer is framed, so no exception crosses into native code: the
 * first byte is [OK] or one of the failure codes, which map one to one
 * onto custody's closed `CipherFailure` set, and the rest is the payload
 * -- the IV and the sealed bytes, the seed, or a failure's message for
 * the log. A message never holds key or seed material.
 */
internal object KeystoreCipher {
    const val OK: Byte = 0
    const val KEY_INVALIDATED: Byte = 1
    const val KEY_MISSING: Byte = 2
    const val USER_NOT_AUTHENTICATED: Byte = 3
    const val AUTHENTICATION: Byte = 4
    const val UNAVAILABLE: Byte = 5

    private const val STORE = "AndroidKeyStore"
    private const val TRANSFORMATION = "AES/GCM/NoPadding"
    private const val TAG_BITS = 128

    /**
     * How long, in seconds, a user-presence key stays usable after the
     * person authenticates: the window for the key operation that follows
     * an authentication and nothing else (ADR-0042 A 2026-10-11, the value
     * SPIKE-009 exercised). A constant, never a setting: the profile's
     * key_unlock_policy picks the mode, the record the time. The default
     * policy, background-compatible, has none.
     */
    private const val USER_PRESENCE_WINDOW_S = 60

    /**
     * Generate a fresh key at [alias], deleting what was there first, and
     * encrypt [seed] with [aad] under it. Answers [OK], the 12-byte IV,
     * then the ciphertext with its tag. [seed] is cleared before this
     * returns, whatever it answers: the plaintext's one call in the Java
     * heap, as for [open].
     */
    @JvmStatic
    fun seal(alias: String, userPresence: Boolean, aad: ByteArray, seed: ByteArray): ByteArray =
        try {
            framed {
                val store = store()
                if (store.containsAlias(alias)) store.deleteEntry(alias)
                val key = generate(alias, userPresence)
                val cipher = Cipher.getInstance(TRANSFORMATION)
                cipher.init(Cipher.ENCRYPT_MODE, key)
                cipher.updateAAD(aad)
                val sealed = cipher.doFinal(seed)
                byteArrayOf(OK) + cipher.iv + sealed
            }
        } finally {
            seed.fill(0)
        }

    /**
     * Decrypt [sealed] (ciphertext and tag) with [iv] and [aad] under the
     * key at [alias]. Answers [OK] then the seed; the caller copies it and
     * clears this array, the only wipe the platform allows.
     */
    @JvmStatic
    fun open(alias: String, iv: ByteArray, sealed: ByteArray, aad: ByteArray): ByteArray = framed {
        val key = store().getKey(alias, null) as SecretKey? ?: return@framed missing(alias)
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(TAG_BITS, iv))
        cipher.updateAAD(aad)
        val seed = cipher.doFinal(sealed)
        val out = byteArrayOf(OK) + seed
        seed.fill(0)
        out
    }

    /** Delete the key at [alias]; no key there is [OK]. */
    @JvmStatic
    fun retire(alias: String): ByteArray = framed {
        val store = store()
        if (store.containsAlias(alias)) store.deleteEntry(alias)
        byteArrayOf(OK)
    }

    private fun store(): KeyStore = KeyStore.getInstance(STORE).apply { load(null) }

    /** StrongBox when the device has one, the TEE otherwise: no change of `PeerId` either way. */
    private fun generate(alias: String, userPresence: Boolean): SecretKey {
        fun spec(strongBox: Boolean): KeyGenParameterSpec {
            val builder =
                KeyGenParameterSpec.Builder(
                        alias,
                        KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
                    )
                    .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                    .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                    .setKeySize(256)
                    .setIsStrongBoxBacked(strongBox)
            if (userPresence) {
                builder
                    .setUserAuthenticationRequired(true)
                    .setUserAuthenticationParameters(
                        USER_PRESENCE_WINDOW_S,
                        KeyProperties.AUTH_DEVICE_CREDENTIAL or
                            KeyProperties.AUTH_BIOMETRIC_STRONG,
                    )
            }
            return builder.build()
        }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, STORE)
        try {
            generator.init(spec(strongBox = true))
            return generator.generateKey()
        } catch (_: StrongBoxUnavailableException) {
            generator.init(spec(strongBox = false))
            return generator.generateKey()
        }
    }

    private fun missing(alias: String): ByteArray = failure(KEY_MISSING, "no key at $alias")

    private fun failure(code: Byte, message: String): ByteArray =
        byteArrayOf(code) + message.toByteArray(Charsets.UTF_8)

    /** [body]'s answer, or the failure code of what it threw. */
    private inline fun framed(body: () -> ByteArray): ByteArray =
        try {
            body()
        } catch (e: KeyPermanentlyInvalidatedException) {
            failure(KEY_INVALIDATED, e.javaClass.simpleName)
        } catch (e: UserNotAuthenticatedException) {
            failure(USER_NOT_AUTHENTICATED, e.javaClass.simpleName)
        } catch (e: AEADBadTagException) {
            failure(AUTHENTICATION, e.javaClass.simpleName)
        } catch (e: Exception) {
            failure(UNAVAILABLE, "${e.javaClass.name}: ${e.message}")
        }
}
