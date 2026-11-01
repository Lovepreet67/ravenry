# Muninn: file tree and file guide

## Full tree

```text
muninn/
├── Cargo.toml
├── Dockerfile
├── Dockerfile.dockerignore
├── config.toml
├── migrations/
│   ├── 0001_create_inbox.up.sql
│   ├── 0001_create_inbox.down.sql
│   ├── 0002_create_events.up.sql
│   ├── 0002_create_events.down.sql
│   ├── 0003_create_deliveries.up.sql
│   └── 0003_create_deliveries.down.sql
└── src/
    ├── main.rs
    ├── config.rs
    ├── error.rs
    ├── state.rs
    ├── core/
    │   ├── mod.rs
    │   ├── pipeline.rs
    │   ├── source.rs
    │   ├── ingester.rs
    │   ├── processor.rs
    │   ├── retry.rs
    │   ├── supervisor.rs
    │   └── inbox/
    │       ├── mod.rs
    │       ├── model.rs
    │       ├── store.rs
    │       ├── postgres.rs
    │       └── tests.rs
    ├── pipelines/
    │   ├── mod.rs
    │   ├── events/
    │   │   ├── mod.rs
    │   │   ├── source.rs
    │   │   ├── model.rs
    │   │   └── projector.rs
    │   └── deliveries/
    │       ├── mod.rs
    │       ├── source.rs
    │       ├── model.rs
    │       └── projector.rs
    └── observability/
        ├── mod.rs
        ├── metrics.rs
        └── health.rs
```

## Root files

| File | What it does |
|---|---|
| `Cargo.toml` | Dependencies. Uses only runtime `sqlx::query*`, so no `DATABASE_URL` is needed at build time. Path deps on `streamforge` and `iam-contracts` are marked `ADJUST`. |
| `Dockerfile` | Multi-stage build (context = workspace root). Runs as a non-root user, exposes 8080. |
| `Dockerfile.dockerignore` | Excludes `target`, `.git`, `.env*` from the build context. |
| `config.toml` | Non-secret settings: DB and Redis URLs (overridable via `MUNINN_*` env vars), queue, inbox, workers, observability. |

## `migrations/`

| File | What it does |
|---|---|
| `0001_create_inbox` | `inbox_messages`: the durable inbox. Holds the lease (`locked_by`, `locked_until`), `attempts` and `deferrals` counters, `raw_payload` for malformed messages, and the `quarantined` status. |
| `0002_create_events` | `events`: the full event envelope, with tenant-scoped indexes. |
| `0003_create_deliveries` | `deliveries` (logical delivery and current state) and `delivery_attempts` (immutable per-attempt history). |
| `*.down.sql` | Drop the tables, in reverse dependency order. |

## `src/` top level

| File | What it does |
|---|---|
| `main.rs` | Entry point. Loads config, runs the `migrate` subcommand if asked, connects Postgres and Redis (fails fast), builds pipelines, spawns every worker under the supervisor, and starts the HTTP server. |
| `config.rs` | Figment config structs with defaults. Loads TOML, then `MUNINN_` env overrides (`__` separates nesting). |
| `error.rs` | The shared `Error` enum (database, serde, queue, source, `LeaseLost`) and the `Result` alias. |
| `state.rs` | `AppState`: the shared pool, queue, and inbox store, built once at startup. |

## `src/core/`: the generic engine

Nothing here knows about events or deliveries.

| File | What it does |
|---|---|
| `mod.rs` | Module declarations and re-exports. |
| `pipeline.rs` | The central contracts. `Pipeline` (name, group, source, projector, `prepare`), `Projector` (writes business tables inside a transaction it is handed), and `ProjectError` with its three variants: `Transient`, `Defer`, `Invalid`. |
| `source.rs` | `StreamSource` trait ("which streams now?") plus `StaticSource` for fixed lists. |
| `ingester.rs` | Queue to inbox to ACK. Reconciles streams on an interval, runs one reader task per stream, inserts batches into the inbox (retrying in place if the DB is down), and ACKs only after commit. Its bottom two functions are the StreamForge adapter. |
| `processor.rs` | Inbox to business tables. Claims a batch, then per message: BEGIN, `project`, `mark_processed_tx`, COMMIT. Maps failures to retry, defer, fail, or quarantine. Also contains the stale-lease reaper loop. |
| `retry.rs` | `RetryPolicy`: exponential backoff, a separate budget for deferrals, and overflow-safe math, with unit tests. |
| `supervisor.rs` | Spawns tasks, handles SIGINT/SIGTERM, cancels everyone, drains within a grace period. If a task dies unexpectedly, the process exits non-zero so Docker restarts it. |

### `src/core/inbox/`

| File | What it does |
|---|---|
| `mod.rs` | Re-exports, and wires in the test module. |
| `model.rs` | `InboxStatus`, `NewInboxMessage` (with `valid()` and `malformed()` constructors), and `InboxMessage` (a claimed row). |
| `store.rs` | The `InboxStore` trait: batch insert, claim, retry, defer, fail, quarantine, and reap. |
| `postgres.rs` | The SQLx implementation. Key points: `ON CONFLICT DO NOTHING` for dedup, `FOR UPDATE SKIP LOCKED` claiming, every state update guarded by `status = 'processing'`, and `mark_processed_tx` for the in-transaction completion. |
| `tests.rs` | DB tests: duplicate insert, quarantine storage, concurrent claims never overlap, lease expiry, lost-lease protection. |

## `src/pipelines/`: the plugins

| File | What it does |
|---|---|
| `pipelines/mod.rs` | The registry, `all()`. **The only file you edit to add a new stream sync**, plus the new folder. |

### `events/`

| File | What it does |
|---|---|
| `mod.rs` | `EventsPipeline`: wires source and projector, and implements `prepare` (decode the envelope, else build a malformed row). |
| `model.rs` | `decode()`: JSON parse, `PublishedEvent` parse, schema-version check. Has unit tests. |
| `source.rs` | `RegistrySource`: asks `TopicRegistry` for active streams and maps them through `TopicStream::destination()`. |
| `projector.rs` | Checks that the stream name matches the event's tenant and topic (tenant isolation), then does an idempotent insert into `events`. |

### `deliveries/`

| File | What it does |
|---|---|
| `mod.rs` | `DeliveriesPipeline`: same wiring pattern. |
| `model.rs` | `decode()`, `validate()`, and `status_for()` (success to `success`, failure with `will_retry` to `pending`, otherwise `failed`). Has unit tests. |
| `source.rs` | A static source on `DELIVERY_RESULTS`. |
| `projector.rs` | The most intricate file. Steps: (1) defer if the event isn't stored yet, (2) check tenant and topic against the event, (3) find or create the delivery and verify its identity, (4) insert the attempt idempotently, (5) update the status only when `current_attempt < new`. Contains the DB tests. |

## `src/observability/`

| File | What it does |
|---|---|
| `mod.rs` | Re-exports. |
| `metrics.rs` | Installs the Prometheus recorder and runs the periodic `muninn_inbox_rows{source,status}` gauge query. |
| `health.rs` | Axum server with `/healthz` (liveness), `/readyz` (Postgres reachable and every pipeline reconciled recently), and `/metrics`. |

## Suggested review order

1. **`migrations/0001`**: understand the inbox states and lease.
2. **`core/pipeline.rs`**: the three traits are the whole extension surface.
3. **`core/ingester.rs`**: check the ACK-after-commit ordering.
4. **`core/inbox/postgres.rs`**: the claim query and `status = 'processing'` guards.
5. **`core/processor.rs`**: the transaction boundary and failure mapping.
6. **`pipelines/deliveries/projector.rs`**: idempotency and the monotonic update.
7. **`main.rs`**: wiring and shutdown.
8. Everything else is supporting code.

## What to scrutinize

- **ACK ordering** in `ingester.rs::read_loop`: ACK must come only after `insert_with_retry` returns `true`.
- **Transaction scope** in `processor.rs::run_tx`: `project` and `mark_processed_tx` must share one `tx`.
- **Lease versus batch time:** `lease_secs` (60) must exceed the time to process a full batch of 100 sequentially.
- **A pool connection** is held by each processor during its transaction, so size `max_connections` with processor count in mind.
- **Module name:** `src/core` may clash with Rust's `core` crate. Rename it to `engine` if you hit path errors.
- **Known gap:** StreamForge `reclaim` doesn't exist yet. Messages consumed but not yet inboxed when a process dies stay stranded. This is the production blocker.

Once you've reviewed it, send me the StreamForge and `iam-contracts` definitions and I'll make the compatibility pass.