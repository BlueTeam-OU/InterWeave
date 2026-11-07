// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
package org.interweave.human

import android.app.Service

/** The release build has no test hooks: the debug source set's are its own. */
internal object DebugHooks {
    @Suppress("UNUSED_PARAMETER")
    fun register(service: Service, restart: () -> Unit): () -> Unit = {}
}
