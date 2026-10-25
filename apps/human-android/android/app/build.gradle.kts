// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

import com.android.build.api.artifact.SingleArtifact

plugins {
    alias(libs.plugins.android.application)
}

// The repository's root, where the native library's cargo build runs.
val workspace: File = rootDir.resolve("../../..").canonicalFile
// Where that build's libraries land, one directory per variant and in it
// one per ABI: a variant's own, so a release APK can never package the
// debug library, which creates a first run's identity without asking
// (review of #255).
fun rustJniLibs(variant: String): Provider<Directory> =
    layout.buildDirectory.dir("rustJniLibs/${variant.lowercase()}")

// The release version (ADR-0065 decision 5): the root Cargo.toml's
// [workspace.package] version is its one committed place, and the APK's
// versionName is produced from it, never typed here. A pre-release
// (0.4.0-rc.1) is carried whole; build metadata is refused, since a tag
// carries none (decision 6) and release_android_apk.sh compares the
// versionName with the tag's version exactly.
val workspaceVersion: String = run {
    val manifest = rootProject.layout.projectDirectory.file("../../../Cargo.toml")
    var section = ""
    var version: String? = null
    for (line in providers.fileContents(manifest).asText.get().lines()) {
        Regex("""^\s*\[([^\]]+)]\s*(#.*)?$""").find(line)?.let { section = it.groupValues[1].trim() }
        if (section == "workspace.package") {
            Regex("""^\s*version\s*=\s*"([^"]*)"\s*(#.*)?$""").find(line)?.let { version = it.groupValues[1] }
        }
    }
    val found = checkNotNull(version) { "${manifest.asFile} has no [workspace.package] version" }
    // tools/ci/release_tag.sh's grammar, which the release run applies to the tag.
    val num = "(0|[1-9][0-9]*)"
    val ident = "($num|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    check(Regex("""^$num\.$num\.$num(-$ident(\.$ident)*)?$""").matches(found)) {
        "the workspace version '$found' is not MAJOR.MINOR.PATCH[-pre-release] (Semantic Versioning 2.0.0, no build metadata)"
    }
    found
}

android {
    namespace = "org.interweave.human"
    compileSdk = 36
    // The NDK the pins name (tools/host/android/android-toolchain.pins),
    // also the one that strips the native library's debug symbols.
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "org.interweave.human"
        // API 30: the oldest the Stage 17 spikes ran on (SPIKE-008, -009).
        minSdk = 30
        // API 34+ enforces the remoteMessaging foreground-service type
        // (ADR-0041); 36 is Play's floor from 2026-08-31.
        targetSdk = 36
        // Not a version (ADR-0065 decision 5): the counter Android orders
        // installs by, raised by one in a commit for every APK published,
        // pre-releases included, never reused and never derived from
        // versionName. No APK has been published yet.
        versionCode = 1
        versionName = workspaceVersion
        // The android-e2e cases' runner (src/androidTest/, tests/android-e2e's
        // src/adb.rs): one case per `am instrument`, in the app's process.
        testInstrumentationRunner = "org.interweave.human.test.E2eCase"
        ndk {
            // The phones this client is built for (plan section 20).
            abiFilters += "arm64-v8a"
        }
    }

    sourceSets {
        listOf("Debug", "Release").forEach { variant ->
            getByName(variant.lowercase()) {
                jniLibs.directories += rustJniLibs(variant).get().asFile.path
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

// The native library, built by cargo-ndk for every ABI above: the debug
// variant from cargo's dev profile, the release variant from --release.
listOf("Debug", "Release").forEach { variant ->
    val cargo = tasks.register<Exec>("cargoNdk$variant") {
        group = "build"
        description = "Builds the native library for the $variant variant."
        workingDir = workspace
        val sdk = androidComponents.sdkComponents
        doFirst {
            environment("ANDROID_NDK_HOME", sdk.ndkDirectory.get().asFile.path)
            // Slint's Android backend compiles a Java helper against this
            // (it needs API 33's classes; left to itself it takes the
            // lowest platform installed).
            environment(
                "ANDROID_JAR",
                sdk.sdkDirectory.get().asFile.resolve("platforms/android-36/android.jar").path,
            )
        }
        // The release build is optimised; the debug build carries the
        // stand-in profile (crates/human/android-platform's stand_in) and
        // creates a first run's identity without asking (jni.rs's
        // FirstRun::Create), until profile-config's provisioning and step 7's
        // onboarding land, and only it does.
        // The debug build also carries the instrumentation's case entry
        // (e2e-cases, src/e2e.rs) and the views' trace to logcat for the
        // screen-driving tests (ui-trace); the release build carries none
        // of them, which verifyReleaseNative below checks of the library
        // it packages.
        val profile =
            if (variant == "Release") listOf("--release") else listOf("--features", "dev-stand-ins,e2e-cases,ui-trace")
        // --locked: the library is built from the reviewed Cargo.lock, as
        // every other cargo build here is (CI's `cargo check --locked` for
        // this target included); a lockfile the build would rewrite is a
        // failure, not a silent new dependency set in the APK.
        commandLine(
            listOf(
                "cargo", "ndk", "-t", "arm64-v8a", "-P", "30",
                "-o", rustJniLibs(variant).get().asFile.path,
                "build", "--locked", "-p", "interweave-human-android",
            ) + profile,
        )
    }
    tasks.matching { it.name == "merge${variant}JniLibFolders" }.configureEach {
        dependsOn(cargo)
    }
}

// The release library names no android-e2e entry: the instrumentation's
// case entry is the debug build's alone (DECISION 01a12770's invariant
// for the APK; the Cargo graph's own check is devex-tooling's). Nor the
// views' trace: every line it writes starts with "ui-trace", ui-slint's
// trace::PREFIX, so a library that could write one carries the marker.
val verifyReleaseNative = tasks.register("verifyReleaseNative") {
    group = "verification"
    description = "Fails if the release native library carries the android-e2e case entry or the views' trace."
    dependsOn("cargoNdkRelease")
    val library = rustJniLibs("Release").map { it.file("arm64-v8a/libinterweave_human_android.so") }
    inputs.file(library)
    doLast {
        val bytes = library.get().asFile.readBytes().toString(Charsets.ISO_8859_1)
        for (name in listOf("Java_org_interweave_human_test_E2eCase_run", "interweave_android_e2e_cases", "ui-trace")) {
            check(!bytes.contains(name)) { "the release native library carries $name" }
        }
    }
}
tasks.matching { it.name == "mergeReleaseJniLibFolders" }.configureEach {
    dependsOn(verifyReleaseNative)
}

// The APK's own versionName against the workspace version, read from the
// merged manifest the APK is packaged with: a value set anywhere else -- a
// hand edit of defaultConfig above, a variant or a manifest overriding it
// -- fails the assemble (ADR-0065: a test that fails when its value and the
// workspace's differ). release_android_apk.sh checks the signed APK again,
// against a manifest read of its own.
androidComponents {
    onVariants { variant ->
        val name = variant.name.replaceFirstChar { it.uppercase() }
        val verify = tasks.register("verify${name}VersionName") {
            group = "verification"
            description = "Fails if the ${variant.name} APK's versionName is not the workspace version."
            val merged = variant.artifacts.get(SingleArtifact.MERGED_MANIFEST)
            // Plain values for the action: the Variant itself holds the
            // project's services, which the configuration cache cannot
            // store (review of #404).
            val variantName = variant.name
            val expected = workspaceVersion
            inputs.file(merged)
            doLast {
                val found = Regex("""android:versionName="([^"]*)"""")
                    .find(merged.get().asFile.readText())?.groupValues?.get(1)
                check(found == expected) {
                    "the $variantName APK's versionName is '$found', not the workspace version $expected"
                }
            }
        }
        tasks.matching { it.name == "assemble$name" }.configureEach { dependsOn(verify) }
    }
}
