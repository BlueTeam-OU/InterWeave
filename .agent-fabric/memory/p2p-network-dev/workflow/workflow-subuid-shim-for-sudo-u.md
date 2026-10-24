---
role: "p2p-network-dev"
class: workflow
topic: "workflow-subuid-shim-for-sudo-u"
description: "run a `sudo -n -u <user>` foreign-uid test on a host with no sudo — a user-namespace shim gives a real second kernel uid"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - eb78ad960ee233dd
---

## run a `sudo -n -u <user>` foreign-uid test on a host with no sudo — a user-namespace shim gives a real second kernel uid

This host has no passwordless sudo, but the login has subuids (`/etc/subuid`). A `sudo` shim earlier on PATH can drop `-n -u <user> --` and run:

`exec unshare --user --map-root-user --map-auto setpriv --reuid=1000 --regid=1000 --clear-groups -- "$@"`

The process is then a host subuid. SO_PEERCRED and the file mode checks see it as a different uid, so a daemon's peer-credential close is really exercised.

Also run with TMPDIR=/tmp: a scratch dir under the 0700 home cannot be traversed by the foreign uid.

Verified on InterWeave #337's `a_foreign_uid_is_refused_by_the_modes_and_then_by_the_peer_credential_check`:
- it passes with the shim;
- it fails with the peer-credential check removed;
- a shim that does not switch the uid fails the test's uid assertion.

CI uses real `sudo -u` (devex-tooling j189).

*Observed 2026-10-21 (p2p-network-dev)*
