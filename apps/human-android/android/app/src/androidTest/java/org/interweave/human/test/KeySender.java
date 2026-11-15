// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
package org.interweave.human.test;

import android.os.SystemClock;
import android.view.InputEvent;
import android.view.KeyCharacterMap;
import android.view.KeyEvent;
import java.lang.reflect.Method;

/**
 * The android-e2e cases' key sender for the reader's Back runs
 * (design/2026-11-04-long-message.md A5; tests/android-e2e's
 * src/back_run.rs): run by the phone's shell, not the app, as
 * {@code CLASSPATH=<this APK> app_process /system/bin
 * org.interweave.human.test.KeySender KEY:ms...}. Each argument is a key
 * code name and how long after the previous press was sent this one is
 * sent ({@code ENTER:0 BACK:600 BACK:30}).
 *
 * <p>Why one process and not one {@code input keyevent} per key: each
 * {@code input} is a fresh runtime, so when its press leaves varies with
 * its start-up by more than the 30 ms A5 asks between two Backs. Here the
 * schedule runs in one process, and each press's send time is read on
 * the uptime clock (the clock a KeyEvent's event time and a monotonic
 * logcat both use) the moment before it is injected, and is the event's
 * own time.
 *
 * <p>Prints {@code sent <KEY> <uptime ms>} for a press whose down and up
 * were both injected, and {@code unsent <KEY> <why>} for one that was
 * not: A5 judges a round with a missing send time void, so a press not
 * injected must have no send time. Injection is asynchronous, as a
 * person's press does not wait for the app: {@code input} waits for the
 * app to finish each event, which would hold the second Back behind the
 * first while the reader's first turn holds input.
 *
 * <p>Java, not Kotlin: app_process loads this APK alone, and the Kotlin
 * standard library is the app's, not this APK's.
 */
public final class KeySender {
    /** InputManager's INJECT_INPUT_EVENT_MODE_ASYNC. */
    private static final int ASYNC = 0;

    private KeySender() {}

    public static void main(String[] args) {
        Method inject;
        Object manager;
        try {
            manager = inputManager();
            inject = manager.getClass().getMethod("injectInputEvent", InputEvent.class, int.class);
        } catch (ReflectiveOperationException e) {
            System.out.println("unsent ALL no injectInputEvent: " + e);
            System.exit(1);
            return;
        }
        long previous = -1;
        for (String arg : args) {
            int colon = arg.indexOf(':');
            String name = colon < 0 ? arg : arg.substring(0, colon);
            long after = colon < 0 ? 0 : Long.parseLong(arg.substring(colon + 1));
            int code = KeyEvent.keyCodeFromString("KEYCODE_" + name);
            if (code == KeyEvent.KEYCODE_UNKNOWN) {
                System.out.println("unsent " + name + " no such key code");
                System.exit(2);
                return;
            }
            if (previous >= 0) {
                long wait = previous + after - SystemClock.uptimeMillis();
                if (wait > 0) SystemClock.sleep(wait);
            }
            long at = SystemClock.uptimeMillis();
            String failed = press(manager, inject, code, at);
            if (failed == null) {
                System.out.println("sent " + name + " " + at);
            } else {
                System.out.println("unsent " + name + " " + failed);
            }
            System.out.flush();
            // The next press is timed from this one's send, sent or not.
            previous = at;
        }
    }

    /** Down then up at `at`; null when both were injected, else why not. */
    private static String press(Object manager, Method inject, int code, long at) {
        for (int action : new int[] {KeyEvent.ACTION_DOWN, KeyEvent.ACTION_UP}) {
            KeyEvent event =
                    new KeyEvent(at, at, action, code, 0, 0, KeyCharacterMap.VIRTUAL_KEYBOARD, 0, 0,
                            android.view.InputDevice.SOURCE_KEYBOARD);
            try {
                Object took = inject.invoke(manager, event, ASYNC);
                if (!Boolean.TRUE.equals(took)) return "refused";
            } catch (ReflectiveOperationException e) {
                return "threw " + e.getCause();
            }
        }
        return null;
    }

    /**
     * The system's input manager: InputManagerGlobal from Android 14,
     * where InputManager lost its static getInstance, else InputManager.
     */
    private static Object inputManager() throws ReflectiveOperationException {
        Class<?> global;
        try {
            global = Class.forName("android.hardware.input.InputManagerGlobal");
        } catch (ClassNotFoundException e) {
            global = Class.forName("android.hardware.input.InputManager");
        }
        return global.getMethod("getInstance").invoke(null);
    }
}
