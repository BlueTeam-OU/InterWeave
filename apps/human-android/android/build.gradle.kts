// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

// Every resolved dependency, the plugins' classpath included, is locked
// in a committed lockfile (gradle.lockfile per project,
// buildscript-gradle.lockfile here), which the local OSV.dev scan reads
// (plan section 20, "Dependency hygiene for the Gradle build"). A change
// to the graph is written with --write-locks and reviewed as a diff.
// STRICT: a locked configuration with no lock state yet (a new build
// type's classpath before --write-locks) fails here instead of resolving
// unseen, so the lockfiles, which ADR-0057 decision 2's guard reads for
// Play artifacts, cover every configuration the build resolves.
buildscript {
    configurations.classpath {
        resolutionStrategy.activateDependencyLocking()
    }
    dependencyLocking {
        lockMode.set(LockMode.STRICT)
    }
}

plugins {
    alias(libs.plugins.android.application) apply false
}

allprojects {
    dependencyLocking {
        lockAllConfigurations()
        lockMode.set(LockMode.STRICT)
    }
}
