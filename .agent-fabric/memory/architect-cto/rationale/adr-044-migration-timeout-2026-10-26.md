---
role: "architect-cto"
class: rationale
topic: "adr-044-migration-timeout-2026-10-26"
description: "j385 2026-10-26: gzapp release migrations run under NO statement bound (Command Timeout 0), each migration tx opens with SET LOCAL lock_timeout='30s', the operator variable unset=none; DECISION 01a175d1-0809 to backend-dev-01; owed…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 17387282949f5d0b
---

## j385 2026-10-26: gzapp release migrations run under NO statement bound (Command Timeout 0), each migration tx opens with SET LOCAL lock_timeout='30s', the operator variable unset=none; DECISION 01a175d1-0809 to backend-dev-01; owed ADR-044 s2.3 A 2026-10-26 triple + devex RELEASE.md REQUEST

backend-dev-01 asked (QUESTION 01a175ce-36dd, 2026-10-26) what statement
timeout a release's migration (ADR-044 §2.3 stop, migrate, start) runs
under: `NpgsqlConnectionFactory` bakes `Command Timeout` 30 s into the
connection string and `Gzapp.Migrate/Program.cs` constructs it with the
default, so a >30 s index build or backfill rolls back and the release
exits 5 with the APIs down. Their local branch adds
`GZAPP_MIGRATE_COMMAND_TIMEOUT_SECONDS` (waits on gzapp #1178 arming).

Ruled (DECISION 01a175d1-0809-7353-ad74-876174dbc973): (1) default
none: a statement running long with the APIs stopped is the release's
work, a cap only adds downtime; (2) stuck = lock wait, bounded by
`SET LOCAL lock_timeout = '30s'` as the FIRST statement of each
migration transaction (set after the blocked statement it is inert, a
field slice measured it); #1178's advisory lock stays unbounded on its
own connection; (3) the variable stays, unset = none; (4) the runner
prints each file's name at start and elapsed time at commit; (5)
RELEASE.md step 3 row + §4 lock-timeout case are devex-tooling's, I
send that REQUEST with their PR number; ADR-044 §2.3 gains one sentence
(A 2026-10-26) supplied onto their branch; (6) not verified, theirs:
an aborted runner (SIGTERM/Ctrl-C) aborts the file's transaction; SET
LOCAL via Dapper Execute on the tx is one round trip.

**Why:** the 30 s figure is sized for a request path under pool
pressure (the factory's own comment), not a one-shot runner; no fixed
figure fits every table size, and the wrong one costs a release.

**How to apply:** when backend-dev-01 replies with the branch, supply
the ADR-044 triple (body sentence in §2.3 after the exit-5 clause,
history note, log row) and send devex-tooling the RELEASE.md REQUEST
naming the PR; the slice promotes into ADR-044's amendment and is then
spent.

**2026-10-26 01:43Z backend-dev-01 REPLY 01a175e0-7f85:** built on local
branch `develop-qzapp/backend-dev-01/feat/j56-migrate-command-timeout`
(e9f25794c + 52e4f6cdd over #1178's 74891e9f, unpushed until #1178 arms
or the owner folds it in). 6b holds (SET LOCAL inside the tx, pooled
connection after commit sees 0). 6a holds for the OUTCOME only: SIGTERM
to the runner exits 143 and frees the advisory lock, but the orphaned
backend stays ACTIVE until its statement ends, keeping its locks; an
immediate re-run then fails the 30 s lock wait. My REPLY (01a175e1-d17f,
sent): the runner also opens each migration tx with
`SET LOCAL client_connection_check_interval = '1s'` (every cluster is
Postgres 18: CI, infra/local, Testcontainers; ADR-022 history), so a
closed socket cancels the orphan within a second; limit: a half-open
socket (host vanished) waits for TCP keepalive, the runbook keeps "end
the holder". Measurement asked: orphan gone within ~1 s, immediate
re-run applies. The ADR-044 sentence names all three settings.

**01:54Z REPLY 01a175ea-4df3:** connection check built as 7df193986 (3
work on the local branch, still unpushed); measured: orphan backend gone
0.31 s after SIGTERM with ~5 s of pg_sleep left, no table, no row;
immediate re-run applied the file, exit 0, 2.1 s wall. Figures go in the
PR body. No reply owed; the ADR-044 triple goes out as a for/ supply.

**02:10Z supplied:** ADR-044 §2.3 A 2026-10-26 ("Step 3 runs under no
statement bound; its waits are bounded") as 534a6644b on
`develop-qzapp/architect-cto-01/for/backend-dev-01/adr-044-migration-bounds`
over main b99972a38: body sentence after the exit-5 clause, history note
(figures attributed to backend-dev-01's rig, none in the body), log row,
digest entry + keywords, index 9→10 AND the end-matter "Amended 10×"
(the validator checks both). REPLY 01a175ea answered naming it. Owed:
devex-tooling REQUEST for RELEASE.md (step 3 row, §4 lock-timeout case,
the variable) once the PR number exists; j385 closes when 534a6644b is
an ancestor of origin/main.

**02:12Z REPLY 01a175fb-1050:** 534a6644b folded --no-ff as 02bd1aeeb on
their local branch; validators and collision scan pass; sentence read
against the three commits and matches (runner exits 1 on a lock timeout,
release.sh step 3 maps it to 5; the ADR speaks of the release, so no
change). Still unpushed until the owner says ride #1178 or own PR after
it; PR number comes then, and with it my devex-tooling RELEASE.md
REQUEST.

**02:50Z REPLY 01a1761e-2576:** #1178 merged 98bb714b; the change is its
own PR gzapp #1182 (4 work, 0 fix, head 5bf7c39c) carrying 534a6644b
(verified ancestor), figures in the PR body; arms on the owner's word.
devex-tooling (one login, running) sent the RELEASE.md REQUEST as a
supply onto #1182's branch: step 3 row, §4 lock-timeout case + aborted
runner, the variable and the compose pass-through as their call. j385
blocked on pr:gzapi-org/gzapp#1182.

**02:54Z devex-tooling REPLY 01a17621-60ad:** taken as their j240; the
three RELEASE.md items as a supply onto #1182's branch, no PR of theirs;
sha follows in a REPLY to 01a1761f. Still j385 blocked on #1182.

**03:04Z devex-tooling REPLY 01a1762a-bb6b:** supply ecc946d16 on
for/backend-dev-01/release-md-migrate-bounds over #1182's 5bf7c39c3
(verified on origin): RELEASE.md step-3 row, §4 lock-wait case
(pg_locks × pg_stat_activity, pg_terminate_backend the holder only), §3
variable; FINDING: `compose run` passes no shell variable into the
container, so the variable needed a `migrate` environment line
(`${VAR:-}`), pinned by test_release.sh. Tree facts: the lock-timeout
text is the inner exception, in the server log, not step 3's output; a
bad value today exits 2 at step 3 = release exit 5 with APIs down. I
said yes to a step-1 refusal (exit 3, nothing stopped) using the
runner's own rule, one more commit on the same for/ branch; ADR text
unchanged ("refuses to start" stays true).

**03:15Z devex-tooling REPLY 01a17634-7761:** step-1 check delivered as
4486f8a3f on the same for/ branch (over ecc946d16): release.sh validates
GZAPP_MIGRATE_COMMAND_TIMEOUT_SECONDS where compose interpolates it
(env, --env-file, .env), blank or digits ≤ 2147483647 pass, else exit 3
nothing stopped; the runner's C# rule and release.sh are pinned by
test_release.sh reading MigrateCommandTimeoutTests.cs InlineData (≥4 of
each). backend-dev-01 folded ecc946d16 as 860d78201; 4486f8a3f needs a
second fold (devex told them). ADR-044 untouched. Nothing owed by me.

**04:11Z backend-dev-01 REQUEST 01a17667-c38c (j388):** #1182 round-3 F5
P3: my note still said release.sh names nothing, false since 4486f8a3f.
Fixed b87f49669 on for/backend-dev-01/adr-044-migration-bounds-2 over
#1182 head 5a51f2341 (Answers: F5): note sentence (release.sh refuses
before step 1, exit 3; env then env file; digits ≤ 2147483647, no hidden
upper limit; prints the bound; compose carries the variable; the ledger
statements under the same lock wait, 038206910), §2.3 clause, digest
entry. Lesson: an "X names nothing" sentence about another lane's file
is false the moment that lane's next commit lands; prefer "the runbook
says how the release treats it" or re-read before each supply.

**CLOSED 2026-10-26 20:49Z:** gzapp #1182 merged 298717e9d with 534a6644b unchanged (backend-dev-01 INFO 01a179f9); measured on main: a bound up to int.MaxValue s runs a 3 s statement to completion, so "no hidden upper limit" holds. Nothing owed either side.

*Observed 2026-10-26 (architect-cto)*
