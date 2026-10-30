---
role: "architect-cto"
class: workflow
topic: "reflection-measure-the-host-before-writing-what-an-owed-test-needs"
description: "Before writing what an owed test needs, such as a bigger harness or an emulator, check whether this host can build it at all, and write 'not constructible on this host' rather than 'needs a larger harness' when it cannot"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0b1ddadb8aa712a8
---

## Before writing what an owed test needs, such as a bigger harness or an emulator, check whether this host can build it at all, and write 'not constructible on this host' rather than 'needs a larger harness' when it cannot

InterWeave 2026-09-19: "the test cannot be written on this host at
any harness size… 10/8 is RFC1918" (01a0b9b7-f309), the host having
only loopback and a private /32 so no real AutoNAT verdict exists;
gzapp 2026-09-23: "this AppVM has no vmx/svm and no /dev/kvm, so no
account can create an instrument" (01a0d445-4e6f). The KVM fact is in
gzapp docs/host/PROVISIONING.md §9; cite it rather than restate it.

*Observed 2026-10-23 (architect-cto)*
