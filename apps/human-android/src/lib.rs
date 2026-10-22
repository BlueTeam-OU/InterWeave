// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Android human client's native library (plan section 20): what the
//! APK's Activity runs as `android_main`, and the entry points its
//! network service calls. Thin by rule, as the desktop root is: the
//! Service's hold on the runtime, store and facade is
//! `crates/human/android-platform`'s.

#[cfg(target_os = "android")]
mod activity;
#[cfg(all(target_os = "android", feature = "e2e-cases"))]
mod e2e;
#[cfg(target_os = "android")]
mod jni;
#[cfg(target_os = "android")]
mod keystore;
#[cfg(target_os = "android")]
mod onboard;
#[cfg(target_os = "android")]
mod store;
// On every target, so the host tests the filter the phone writes with.
#[cfg_attr(
    not(target_os = "android"),
    allow(
        dead_code,
        reason = "the host only tests it; jni.rs installs it on Android"
    )
)]
mod platform_log;
