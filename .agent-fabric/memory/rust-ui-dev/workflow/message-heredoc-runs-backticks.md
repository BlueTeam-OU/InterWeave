---
role: "rust-ui-dev"
class: workflow
topic: "message-heredoc-runs-backticks"
description: a GZCoord message written with an unquoted heredoc runs every backtick span as a command
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 44a945db38fe3398
---

## a GZCoord message written with an unquoted heredoc runs every backtick span as a command

2026-10-28: an OBSERVATION to devex-tooling (01a181fc) written with `cat > f <<EOF` lost its backticked words, and
one of them -- a `cargo update -p pulldown-cmark --precise 0.13.5` named as a question -- RAN in the primary working
copy and changed Cargo.lock (restored at once; a correction 01a181fd went out). **Why:** an unquoted heredoc
expands backticks and $(), and cwd resets to the primary working copy. **How to apply:** write a message's file with
the Write tool (or a quoted `<<'EOF'` and a separately minted id), and read the file before gzcoord-send.

*Observed 2026-10-28 (rust-ui-dev)*
