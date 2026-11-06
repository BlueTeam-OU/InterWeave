# s30-d-daemon-drops

## Run

```json
{
  "run": "s30-d-daemon-drops",
  "tree": "dbe43042c",
  "nonce": "538b9977",
  "count": 200,
  "refill_wait_s": 20,
  "quiet_after_s": 10,
  "started_at": "2026-11-06T00:58:36Z",
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
    "mesh_formed": true,
    "mesh_seen_at_s": 2.4,
    "publish_span_s": 2.184,
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
      "since": "2026-11-06T00:58:37.566Z"
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
      "ingress_refused": 153,
      "queue_overflow": 0,
      "since": "2026-11-06T00:58:37.566Z"
    }
  },
  "notified": 47,
  "notified_distinct": 47,
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
    34,
    35,
    42,
    58,
    66,
    88,
    104,
    112,
    136,
    145,
    162,
    173,
    182,
    183
  ],
  "sums": {
    "daemon_a_events_dropped_delta": 0,
    "events_dropped_total_present": true,
    "bridge_a_ingress_refused_delta": 153,
    "notified_plus_daemon_dropped": 47,
    "notified_plus_daemon_dropped_plus_ingress_refused": 200
  },
  "passed": true
}
```
