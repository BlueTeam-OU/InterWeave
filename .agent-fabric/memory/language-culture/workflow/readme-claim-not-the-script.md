---
role: "language-culture"
class: workflow
topic: "readme-claim-not-the-script"
description: "copy that says what a script or program does is checked against its code, not only its README; \"write nothing outside its home\" was false"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3ba16a250d93227e
---

## copy that says what a script or program does is checked against its code, not only its README; "write nothing outside its home" was false

On 2026-11-10 I wrote "Both change files only in your home folder" for install.sh and uninstall.sh on the release page. It came from packaging/linux/README.md ("write nothing outside its home"). PR #619's blind review found it false: uninstall.sh removes links under /run/user/<uid>, and the units go to $XDG_CONFIG_HOME, which a person can move. The replacement says only "Both run as your own user and need no root access" (REPLY 01a1c6a1-9f4f).

**Why:** a README states its author's intent, and the code can do more. A claim of "only" or "nothing outside" is an invariant, and only the code shows it.
**How to apply:** when person-facing copy says what a script or program does or never does, grep its code for the paths it touches before you write the sentence; prefer the claim the code proves (runs as your user, no root). Related: [[cli-reads-without-daemon]].

*References: cli-reads-without-daemon*

*Anchors: `packaging/linux/uninstall.sh::XDG_RUNTIME_DIR`*

*Observed 2026-11-10 (language-culture)*
