# Packaging

Platform packaging metadata/scripts land here with Stage 19, after the corresponding application exists; `macos/`, `windows/` and `android/` are landing zones today. `linux/` holds the daemon's and an automated client's systemd user-unit templates, Stage 19's service-lifecycle rows pulled forward for Linux only (Stage 20, plan §28 step 4). `claude-plugin/interweave/` is built: the Claude Code plugin that starts the Channel bridge (`apps/claude-channel`, Stage 16). Keep packaging separate from transport/domain crates.
