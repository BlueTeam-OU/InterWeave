---
role: "p2p-network-dev"
class: threads
topic: "j125-deploy-account-needs-authorized-keys"
description: "j125 second-uid run — interweave-deploy (uid 1032) exists with linger but no authorized_keys; root installs it via ~/j125/authorize-key.sh"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - fed4267e605a97e0
---

## j125 second-uid run — interweave-deploy (uid 1032) exists with linger but no authorized_keys; root installs it via ~/j125/authorize-key.sh

State on 2026-10-26: v0.1.0 is released (tag 71961578; Linux archive sha256
77fcc622473016a5232bf10783c67581b8404cdeb910a9cd29855412c67e5115, verified
against its .sha256, downloaded to ~/j125/dl). The owner made the account
`interweave-deploy` (uid 1032, gid 1035 own group, Linger=yes) but put no
`~/.ssh/authorized_keys` in it, so SSH from p2p-network-dev-01 with
~/.ssh/interweave-deploy_ed25519 is refused (publickey). The root step is
`sudo sh ~/j125/authorize-key.sh`; asked of the owner in
session and via architect-cto (REPLY seq 572878). Job j125 is blocked on it.

**Why:** DEPLOYING.md needs a real login session (user manager +
XDG_RUNTIME_DIR); sudo -u / su -c give neither, so SSH is the only route.
**How to apply:** when resumed, first `ssh -i ~/.ssh/interweave-deploy_ed25519
interweave-deploy@localhost id`; my home is 0700, so copy the archive over scp,
never point the deploy account at ~/j125. Work branch:
develop-qzapp/p2p-network-dev-01/feat/j125-v0-1-0-on-this-host. See [[j110-withheld-lease-race]].

**2026-10-27: the key install was not enough.** The owner ran authorize-key.sh, and SSH is still refused. This host's sshd is agent-fabric's ADR-048 operator door (ssh-operator.sh). Its `AllowUsers` is the registry-placed logins only (journal: "not listed in AllowUsers"). `AuthorizedKeysFile` is `/etc/ssh/authorized_keys/%u`, root-owned and forcing enter-ssh, so `~/.ssh/authorized_keys` is never read. On Qubes `/etc/ssh` is restored from staging at boot. Lesson: read a host's `sshd -T` before writing a key-install script. Now blocked on fabric-coordinator's REQUEST 01a17c3c (sent at the owner's word); the diagnosis script is ~/j125/diagnose-ssh.sh.

**2026-10-27, later: route changed.** fabric-coordinator refused any shell into another uid (REPLY 01a17c3d). ADR-038 says the login is the agent, and ADR-048's door is closed. The owner chose route 1: p2p-network-dev-02 deploys v0.1.0 in its own account and tries the port (REQUEST 01a17c3f). My side is the throwaway profile `j125-hold`, unit `interweave-daemon@j125-hold` (started, not enabled), on 127.0.0.1:47990; it must be stopped for the control step and removed afterwards. Never use the fleet's `gzcoord` port for this: if the rule failed, fleet traffic would be split. Reading another account's processes is refused by the classifier; ask the account instead.

**MEASURED 2026-10-27:** uid 1022's daemon exited 1 on EADDRINUSE and its unit retried every 5 s while uid 1016 held 127.0.0.1:47990. One retry after the holder stopped, it listened. Both test profiles and their directories were removed. The record is packaging/linux/evidence/2026-10-27-second-uid-port.md (6cdeecf6, on #475). Reported to architect-cto (REPLY 01a17c4a); j125 is blocked on their word that this answers it. The fresh-install-from-archive half was not run.

**DELIVERED 2026-10-27** on #475 and commit 6cdeecf6. architect-cto accepted it (REPLY 01a17c4d), amended decision 5 (5294afc9, folded), and asked for the DEPLOYING.md citation (38db9e38). The fresh-install run is carried to the owner as a separate ask.

*References: j110-withheld-lease-race*

*Observed 2026-10-27 (p2p-network-dev)*
