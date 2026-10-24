// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
package org.interweave.human

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.ImageFormat
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CaptureRequest
import android.media.ImageReader
import android.os.Handler
import android.os.HandlerThread
import android.view.Surface

/**
 * The camera behind Scan a code (add-a-peer section 5.5): the platform's
 * own Camera2, no library, opened as the viewfinder opens and closed with
 * it by its owner (MainActivity). Each frame's
 * luma plane goes to the window's side (`Native.cameraFrame`), which draws
 * it and reads codes from it; no picture is kept here or there.
 *
 * The caller holds the CAMERA permission before [open]; a camera that
 * fails to open, or is taken away, is reported to its owner ([lost]), who
 * ends the scan as closed when this is still its camera.
 */
class CodeCamera(context: Context, private val lost: (CodeCamera) -> Unit) {
    private val manager = context.getSystemService(CameraManager::class.java)
    private var thread: HandlerThread? = null
    private var device: CameraDevice? = null
    private var session: CameraCaptureSession? = null
    private var reader: ImageReader? = null
    /**
     * Closed by its owner. A camera that opens after it is closed at once
     * and its thread ended (#418 round 2).
     */
    @Volatile private var stopped = false

    /** The back camera if there is one, else the first; null with none. */
    private fun cameraId(): String? {
        val ids = manager?.cameraIdList ?: return null
        return ids.firstOrNull {
            manager.getCameraCharacteristics(it).get(CameraCharacteristics.LENS_FACING) ==
                CameraCharacteristics.LENS_FACING_BACK
        } ?: ids.firstOrNull()
    }

    @SuppressLint("MissingPermission") // the caller asked CAMERA first
    fun open() {
        if (device != null) return
        stopped = false
        val id = cameraId()
        if (id == null) {
            lost(this)
            return
        }
        val worker = HandlerThread("interweave-camera").also { it.start() }
        thread = worker
        val handler = Handler(worker.looper)
        // Small frames: a code needs no more, and each is copied once.
        val frames = ImageReader.newInstance(WIDTH, HEIGHT, ImageFormat.YUV_420_888, 2)
        frames.setOnImageAvailableListener({ r ->
            val image = r.acquireLatestImage() ?: return@setOnImageAvailableListener
            try {
                val luma = image.planes[0]
                Native.cameraFrame(image.width, image.height, luma.rowStride, luma.buffer)
            } finally {
                image.close()
            }
        }, handler)
        reader = frames
        try {
            manager.openCamera(id, object : CameraDevice.StateCallback() {
                override fun onOpened(camera: CameraDevice) {
                    if (stopped) {
                        camera.close()
                        worker.quitSafely()
                        return
                    }
                    device = camera
                    start(camera, frames.surface, handler)
                }

                override fun onDisconnected(camera: CameraDevice) {
                    camera.close()
                    closed()
                }

                override fun onError(camera: CameraDevice, error: Int) {
                    camera.close()
                    closed()
                }
            }, handler)
        } catch (e: Exception) {
            closed()
        }
    }

    private fun start(camera: CameraDevice, surface: Surface, handler: Handler) {
        try {
            @Suppress("DEPRECATION") // the list form serves every API from 30
            camera.createCaptureSession(listOf(surface), object : CameraCaptureSession.StateCallback() {
                override fun onConfigured(s: CameraCaptureSession) {
                    session = s
                    val request = camera.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW).apply {
                        addTarget(surface)
                        set(CaptureRequest.CONTROL_AF_MODE, CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE)
                    }
                    try {
                        s.setRepeatingRequest(request.build(), null, handler)
                    } catch (e: Exception) {
                        closed()
                    }
                }

                override fun onConfigureFailed(s: CameraCaptureSession) {
                    closed()
                }
            }, handler)
        } catch (e: Exception) {
            closed()
        }
    }

    /** The camera went away under the scan: released, and the owner told. */
    private fun closed() {
        release(quit = true)
        lost(this)
    }

    /** Stop and release everything; nothing is told. Safe to repeat. */
    fun close() {
        stopped = true
        // While opening, the thread stays for onOpened, which closes the
        // device it is handed and ends the thread.
        release(quit = device != null)
    }

    @Synchronized
    private fun release(quit: Boolean) {
        try { session?.close() } catch (e: Exception) {}
        try { device?.close() } catch (e: Exception) {}
        try { reader?.close() } catch (e: Exception) {}
        session = null
        device = null
        reader = null
        if (quit) thread?.quitSafely()
        thread = null
    }

    companion object {
        private const val WIDTH = 640
        private const val HEIGHT = 480
    }
}
