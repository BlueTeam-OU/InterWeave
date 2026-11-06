# s30-d-daemon-drops

## Run

```json
{
  "run": "s30-d-daemon-drops",
  "tree": "9995f18d0",
  "nonce": "6693726f",
  "count": 200,
  "refill_wait_s": 20,
  "quiet_after_s": 10,
  "started_at": "2026-11-06T00:17:16Z",
  "steps": {
    "join_a": {
      "answer": "joined general",
      "error": false
    },
    "join_b": {
      "answer": "joined general",
      "error": false
    },
    "mesh_publishes": 1,
    "mesh_seen_at_s": 2.4,
    "publish_span_s": 0.637,
    "publish_errors": []
  },
  "before": {
    "a_admin": {
      "ipc": {
        "data_connections": 1,
        "admin_connections": 1,
        "active_leases": 1,
        "cross_domain_capability_denied_total": 0,
        "peer_credential_refused_total": 0,
        "events_dropped_total": 0
      },
      "health": "healthy"
    },
    "b_admin": {
      "ipc": {
        "data_connections": 1,
        "admin_connections": 1,
        "active_leases": 1,
        "cross_domain_capability_denied_total": 0,
        "peer_credential_refused_total": 0,
        "events_dropped_total": 0
      },
      "health": "healthy"
    },
    "a_bridge_loss": {
      "ingress_refused": 0,
      "queue_overflow": 0,
      "since": "2026-11-06T00:17:17.074Z"
    }
  },
  "after": {
    "a_admin": {
      "ipc": {
        "data_connections": 1,
        "admin_connections": 1,
        "active_leases": 1,
        "cross_domain_capability_denied_total": 0,
        "peer_credential_refused_total": 0,
        "events_dropped_total": 0
      },
      "health": "healthy"
    },
    "b_admin": {
      "ipc": {
        "data_connections": 1,
        "admin_connections": 1,
        "active_leases": 1,
        "cross_domain_capability_denied_total": 0,
        "peer_credential_refused_total": 0,
        "events_dropped_total": 0
      },
      "health": "healthy"
    },
    "a_bridge_loss": {
      "ingress_refused": 163,
      "queue_overflow": 0,
      "since": "2026-11-06T00:17:17.074Z"
    }
  },
  "notified": 37,
  "notified_distinct": 37,
  "notified_order_held": true,
  "notified_numbers": [
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10,
    11,
    12,
    13,
    14,
    15,
    16,
    17,
    18,
    19,
    20,
    21,
    22,
    23,
    24,
    25,
    26,
    27,
    28,
    29,
    30,
    31,
    32,
    33,
    82,
    122,
    161,
    193
  ],
  "sums": {
    "daemon_a_events_dropped_delta": 0,
    "events_dropped_total_present": true,
    "bridge_a_ingress_refused_delta": 163,
    "notified_plus_daemon_dropped": 37,
    "notified_plus_daemon_dropped_plus_ingress_refused": 200
  },
  "passed": true
}
```
