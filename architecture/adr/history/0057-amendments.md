# ADR-0057 — amendment history

### Amendment 2026-10-19 — `com.google.gms` is a fifth refused group

**Trigger.** devex-tooling built decision 2's guard (`tools/checks/check_android_no_play_services.sh`, InterWeave #307) as the decision named it and reported one group it therefore does not refuse: `com.google.gms`, the Google services Gradle plugin's group, which is how Firebase and Play services are usually wired into a build. The decision's four groups were the artifacts, not the plugin that pulls them in.

**Ruled.** The plugin's group joins the list; the guard refuses it on every application configuration as it refuses the four, the buildscript classpath included, since the decision exempts lint and test tooling only. No other rule changes; the Gradle lockfiles on `main` carry none of the five today.
