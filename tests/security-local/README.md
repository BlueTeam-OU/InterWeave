# security-local

Stage 18's local adversarial suite (plan section 21, steps S9-S11): the
daemon's data and admin sockets, transportctl and the stores, attacked
from the same host. It covers admin/data socket spoofing, a real foreign
uid, storage failure, and identity recovery failure and tamper.

The gate is [`../SECURITY-MATRIX.md`](../SECURITY-MATRIX.md). The
network half is [`../security`](../security).
