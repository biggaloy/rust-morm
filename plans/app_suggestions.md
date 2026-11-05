# App and operations suggestions

These belong to the app or to operations, not to rust-morm. Anything morm handles itself is in `rust-morm-plan.md`.

## Database setup

| Area | What to do |
|---|---|
| **Roles** | Use a migration role that can change the schema, and an app role that can only read and write data. A bug or injection in the app then can't drop tables. morm's `MORM_MIGRATION_URL` supports this |
| **Timeouts** | Set `statement_timeout` for the app role, for example 30s. morm also has `statement_timeout` and `idle_in_transaction_session_timeout` settings, and a role-level value covers clients that aren't morm |
| **Collations** | A glibc upgrade can change how text sorts, which silently corrupts text indexes until they're rebuilt with `REINDEX`. ICU or `C` collations avoid this. After an OS upgrade, check collation version warnings |

## Keeping tables healthy

| Area | What to do |
|---|---|
| **Vacuum and bloat** | Frequently updated tables collect dead rows, so tune autovacuum for hot tables. Long transactions block vacuum across the database. Monitor transaction ID wraparound |
| **Fast updates (HOT)** | Updates are much cheaper when no indexed column changes. Don't index columns that change constantly, such as counters or `last_seen_at`. Consider `fillfactor` 80–90 on update-heavy tables, with morm's `storage` option |
| **Large tables** | Partition time-series data, for example by month. Dropping an old partition is instant, while deleting the same rows takes hours |
| **Sequences** | `bigserial` values have gaps, because rolled-back inserts still use numbers. Never assume ids are contiguous |

## Availability

| Area | What to do |
|---|---|
| **Backups** | Use point-in-time recovery, with pgBackRest or WAL-G, and regularly test restores. A backup that has never been restored isn't proven |
| **Replicas** | Replication lag means users might not see their own writes when the next read goes to a replica. Send reads that follow a write to the primary |
| **Failover** | Run it through a tested tool, and rehearse it. morm discards connections to the old primary, and `target_session_attrs = "read-write"` finds the new one |
| **Major upgrades** | Plan for `pg_upgrade`, or logical replication for near-zero downtime |

## Monitoring

Enable `pg_stat_statements` for slow and frequent queries. Watch `pg_stat_activity` for long transactions and lock waits. Track bloat and disk use. morm's `pool_stats()` adds pool waits, deadlocks, retries and lock timeouts from the app's side.
