# README.md — Payment Service

```markdown
# Payment Service

A production-grade payment microservice written in Rust that exposes **gRPC** for internal callers and **HTTP webhooks** for gateway callbacks. It guarantees **idempotency**, **state-machine safety**, **transactional outbox** for Kafka, and **TTL-based expiry** for reserved payments.

---

## Table of Contents

- [Architecture](#architecture)
- [Domain Model](#domain-model)
- [Payment State Machine](#payment-state-machine)
- [Features](#features)
- [Project Layout](#project-layout)
- [Configuration](#configuration)
- [Running the Service](#running-the-service)
- [Database Schema](#database-schema)
- [gRPC API](#grpc-api)
- [HTTP Webhooks](#http-webhooks)
- [Outbox → Kafka](#outbox--kafka)
- [Testing](#testing)
- [Running Tests](#running-tests)
- [Operational Notes](#operational-notes)

---

## Architecture

```
┌────────────────┐      gRPC       ┌──────────────────┐
│  Order Service │ ───────────────▶│ Payment Service  │
└────────────────┘                 │                  │
                                   │  ┌────────────┐  │        ┌─────────────┐
┌────────────────┐   HTTP webhook   │  │ gRPC Srv   │  │        │  PostgreSQL │
│ Payment Gateway│ ───────────────▶│  ├────────────┤  │ ◀────▶ │  (payments, │
└────────────────┘                 │  │ HTTP Srv   │  │        │   outbox,   │
                                   │  ├────────────┤  │        │   attempts, │
                                   │  │ Expiry     │  │        │  webhooks)  │
                                   │  │  Worker    │  │        └─────────────┘
                                   │  ├────────────┤  │
                                   │  │ Outbox     │  │        ┌─────────────┐
                                   │  │  Publisher │  │ ─────▶ │   Kafka     │
                                   │  └────────────┘  │        └─────────────┘
                                   └──────────────────┘
```

Three background concerns run alongside the servers:

1. **HTTP server** — receives gateway webhooks, verifies HMAC signatures, deduplicates events.
2. **Expiry worker** — scans `payments` where `reservation_expires_at < now()` and marks them `expired`.
3. **Outbox publisher** — polls `outbox_events` where `published_at IS NULL` and forwards to Kafka.

---

## Domain Model

| Field | Type | Notes |
|---|---|---|
| `id` | `TEXT` (UUIDv7) | Primary key, time-ordered |
| `order_id` | `TEXT` | Reference to caller's order |
| `idempotency_key` | `TEXT UNIQUE` | Caller-supplied or defaulted to `order:<order_id>` |
| `amount` | `NUMERIC(14,2)` | Never a float |
| `currency` | `CHAR(3)` | ISO-4217 |
| `status` | `TEXT` (CHECK) | See state machine |
| `gateway` | `TEXT` | e.g. `mock`, `stripe` |
| `gateway_reference` | `TEXT NULL` | External ID from gateway |
| `payment_url` | `TEXT NULL` | Hosted checkout URL |
| `failure_reason` | `TEXT NULL` | Populated on failures |
| `reservation_expires_at` | `TIMESTAMPTZ` | TTL enforcement |

---

## Payment State Machine

```
              ┌─────────┐
   create ───▶│ pending │
              └────┬────┘
                   │ gateway OK
                   ▼
              ┌────────────┐        cancel       ┌───────────┐
              │ processing │ ──────────────────▶ │ cancelled │
              └────┬───────┘                     └───────────┘
                   │
     ┌─────────────┼─────────────┐
     │             │             │
     │  succeeded  │  failed     │ ttl exceeded
     ▼             ▼             ▼
┌───────────┐ ┌────────┐   ┌─────────┐
│ succeeded │ │ failed │   │ expired │
└─────┬─────┘ └────────┘   └─────────┘
      │ refund
      ▼
┌──────────┐
│ refunded │
└──────────┘
```

All transitions go through `PaymentRepo::transition(...)`, which:

1. Opens a **transaction**.
2. `UPDATE payments ... WHERE id=$1 AND status = ANY($2) RETURNING *` — the `status = ANY` clause enforces the "from" set.
3. If no row: `PaymentError::InvalidState`.
4. **Inserts** an `outbox_events` row **atomically** with the transition.
5. Commits.

This guarantees **outbox correctness**: no event is ever published without a state change, and no state change happens without an event.

---

## Features

- ✅ **Idempotent creation** — unique key; identical replay returns the same payment. Mismatched `amount`/`order_id` → `IdempotencyMismatch`.
- ✅ **Transactional outbox** — atomic DB update + event insert; publisher is best-effort.
- ✅ **At-least-once webhooks** — `processed_webhooks.gateway_event_id` PK prevents reprocessing.
- ✅ **HMAC-verified webhooks** — `x-gateway-signature` header, constant-time compare.
- ✅ **TTL expiry** — a background scan cancels gateway reservations and transitions payments to `expired`.
- ✅ **Typed errors** — `PaymentError` maps to both `tonic::Status` and `axum::Response` with correct HTTP/gRPC codes.
- ✅ **Structured logging** — `tracing` with JSON output.
- ✅ **Graceful shutdown** — Ctrl-C / SIGTERM.

---

## Project Layout

```
.
├── Cargo.toml
├── build.rs                     # tonic-build for proto
├── migrations/                  # sqlx migrations
│   └── 0001_init.sql
├── proto/
│   └── payment.proto
├── src/
│   ├── main.rs                  # servers + workers
│   ├── config.rs                # PaymentConfig::from_env
│   ├── error.rs                 # PaymentError + IntoResponse + Into<Status>
│   ├── state.rs                 # AppState { db, gateway, config }
│   ├── gateway/
│   │   ├── mod.rs               # build() factory
│   │   ├── traits.rs            # PaymentGateway trait
│   │   └── mock.rs              # MockGateway (HMAC + hash ref)
│   ├── outbox/
│   │   └── publisher.rs         # outbox → Kafka batch
│   └── payment/
│       ├── repo.rs              # PaymentRow, PaymentRepo
│       ├── service.rs           # gRPC PaymentSvc
│       ├── webhook.rs           # axum handler
│       ├── expiry.rs            # expire_batch
│       └── kafka_publisher.rs   # optional instant publish
└── tests/
    ├── common/mod.rs            # test_state(), test_config()
    ├── config_tests.rs
    ├── gateway_tests.rs
    ├── repo_tests.rs
    ├── service_tests.rs
    ├── webhook_tests.rs
    └── expiry_tests.rs
```

---

## Configuration

All configuration is provided via environment variables. `PaymentConfig::from_env()` reads:

| Variable | Type | Example | Required |
|---|---|---|---|
| `DATABASE_URL` | URL | `postgres://user:pass@host/db` | ✅ |
| `GRPC_ADDR` | `SocketAddr` | `0.0.0.0:50051` | ✅ |
| `HTTP_ADDR` | `SocketAddr` | `0.0.0.0:8080` | ✅ |
| `PAYMENT_GATEWAY` | string | `mock` | ✅ |
| `GATEWAY_PUBLIC_BASE` | URL | `https://pay.example.com` | ✅ |
| `WEBHOOK_SECRET` | string | `hmac-shared-secret` | ✅ |
| `RESERVATION_TTL_SECS` | i64 | `900` | ✅ |
| `EXPIRY_SCAN_SECS` | u64 | `30` | ✅ |
| `KAFKA_BROKERS` | string | `kafka1:9092,kafka2:9092` | ✅ |
| `KAFKA_PAYMENT_TOPIC` | string | `payment.events` | ✅ |

`.env` is loaded automatically at startup via `dotenvy` when present.

### Example `.env`

```dotenv
DATABASE_URL=postgres://postgres:postgres@localhost:5432/payment
GRPC_ADDR=0.0.0.0:50051
HTTP_ADDR=0.0.0.0:8080
PAYMENT_GATEWAY=mock
GATEWAY_PUBLIC_BASE=http://localhost:8080/pay
WEBHOOK_SECRET=dev-secret-change-me
RESERVATION_TTL_SECS=900
EXPIRY_SCAN_SECS=30
KAFKA_BROKERS=localhost:9092
KAFKA_PAYMENT_TOPIC=payment.events
```

---

## Running the Service

### Prerequisites

- Rust 1.75+
- PostgreSQL 14+
- Kafka (or a compatible broker)

### Local

```bash
# 1) start postgres + kafka (docker compose example)
docker run -d --name pg -p 5432:5432 \
  -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=payment postgres:16

docker run -d --name kafka -p 9092:9092 \
  -e KAFKA_CFG_NODE_ID=0 -e KAFKA_CFG_PROCESS_ROLES=controller,broker \
  -e KAFKA_CFG_LISTENERS=PLAINTEXT://:9092,CONTROLLER://:9093 \
  -e KAFKA_CFG_CONTROLLER_QUORUM_VOTERS=0@kafka:9093 \
  -e KAFKA_CFG_LISTENER_SECURITY_PROTOCOL_MAP=CONTROLLER:PLAINTEXT,PLAINTEXT:PLAINTEXT \
  -e KAFKA_CFG_CONTROLLER_LISTENER_NAMES=CONTROLLER \
  bitnami/kafka:3.6

# 2) prepare env
cp .env.example .env

# 3) run
cargo run --release
```

Migrations are applied automatically at startup (`sqlx::migrate!("./migrations")`).

---

## Database Schema

Four tables:

### `payments`

```sql
CREATE TABLE payments (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    amount NUMERIC(14, 2) NOT NULL,
    currency CHAR(3) NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'pending','processing','succeeded','failed','cancelled','expired','refunded')),
    gateway TEXT NOT NULL,
    gateway_reference TEXT,
    payment_url TEXT,
    failure_reason TEXT,
    customer_id TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    reservation_expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### `payment_attempts`

Raw audit log of every gateway call (request/response/error).

### `processed_webhooks`

Idempotency ledger for gateway events — `gateway_event_id` is the PK.

### `outbox_events`

```sql
CREATE TABLE outbox_events (
    id TEXT PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    published_at TIMESTAMPTZ
);
CREATE INDEX idx_outbox_unpublished ON outbox_events(created_at)
    WHERE published_at IS NULL;
```

---

## gRPC API

Package: `payment.v1`

| RPC | Request | Behavior |
|---|---|---|
| `CreatePayment` | `{order_id, amount, currency, idempotency_key?, customer_id?, metadata?}` | Insert pending, call gateway, transition to `processing` (or `failed`). Idempotent. |
| `GetPayment` | `{payment_id}` | Returns current row. |
| `CancelPayment` | `{payment_id, reason}` | `pending|processing` → `cancelled`, calls gateway cancel. |
| `RefundPayment` | `{payment_id, amount, reason}` | `succeeded` → `refunded`. |

### Example — `grpcurl`

```bash
grpcurl -plaintext -d '{
  "order_id":"order-42",
  "amount":"100.00",
  "currency":"USD",
  "idempotency_key":"order-42-v1",
  "metadata":{"channel":"mobile"}
}' localhost:50051 payment.v1.PaymentService/CreatePayment
```

Response:

```json
{
  "id": "0193c1f0-7a8b-7c4d-9e2f-1a2b3c4d5e6f",
  "orderId": "order-42",
  "status": "processing",
  "amount": "100.00",
  "currency": "USD",
  "gatewayReference": "mock_a1b2c3d4e5",
  "paymentUrl": "http://localhost:8080/pay/mock_a1b2c3d4e5",
  "createdAt": "2025-01-15T10:00:00Z",
  "updatedAt": "2025-01-15T10:00:00.120Z"
}
```

### Error mapping

| `PaymentError` | gRPC `Code` |
|---|---|
| `NotFound` | `NOT_FOUND` |
| `Duplicate` | `ALREADY_EXISTS` |
| `IdempotencyMismatch` | `INVALID_ARGUMENT` |
| `InvalidState` | `FAILED_PRECONDITION` |
| `Gateway` | `UNAVAILABLE` |
| `WebhookVerification` | `UNAUTHENTICATED` |
| `BadRequest`, `InvalidQuantity` | `INVALID_ARGUMENT` |
| `Database`, `VarError`, ... | `INTERNAL` |

---

## HTTP Webhooks

**Endpoint:** `POST /webhooks/payment/:payment_id`
**Header:** `x-gateway-signature: <hex(HMAC-SHA256(secret, body))>`

Body:

```json
{
  "event_id": "evt_abc",
  "event_type": "payment.succeeded",
  "payment_id": "…",
  "data": { }
}
```

### Supported `event_type`

| Event | From → To |
|---|---|
| `payment.succeeded` | `pending|processing` → `succeeded` |
| `payment.failed` | `pending|processing` → `failed` |
| `payment.refunded` | `succeeded` → `refunded` |

### Status codes

| Code | Meaning |
|---|---|
| `200` | Applied, or duplicate ignored, or transition not allowed (safe to ignore) |
| `400` | Bad signature, unknown event type, or malformed JSON |
| `401` | Signature verification failed |

### Verification example

```bash
BODY='{"event_id":"evt_1","event_type":"payment.succeeded"}'
SIG=$(printf '%s' "$BODY" | openssl dgst -sha256 -hmac "$WEBHOOK_SECRET" -hex | awk '{print $2}')
curl -X POST "http://localhost:8080/webhooks/payment/$PAYMENT_ID" \
  -H "x-gateway-signature: $SIG" \
  -d "$BODY"
```

---

## Outbox → Kafka

Every state transition writes a row into `outbox_events`. A background publisher (1s interval):

1. `SELECT … FOR UPDATE SKIP LOCKED … LIMIT 100`.
2. Sends each to `KAFKA_PAYMENT_TOPIC` keyed by `aggregate_id` (payment_id).
3. Marks `published_at = now()`.
4. Commits the transaction.

Kafka payload:

```json
{
  "event": "payment.succeeded",
  "payment_id": "0193c1f0-…",
  "order_id": "order-42",
  "status": "succeeded",
  "reason": null,
  "data": { "gateway_event_id": "evt_abc" }
}
```

Producer is configured with `acks=all`, `enable.idempotence=true`, `compression.type=zstd`.

---

## Testing

The test suite is organized by responsibility and lives entirely under `tests/`.

### Test matrix

| File | Scope | Requires DB? | Coverage |
|---|---|---|---|
| `gateway_tests.rs` | `MockGateway` — HMAC verification, create, cancel | ❌ | signature accept/reject, URL shape |
| `config_tests.rs` | `PaymentConfig::from_env` | ❌ | happy path, missing var |
| `repo_tests.rs` | `PaymentRepo` | ✅ | insert/get, duplicate key, transition + outbox, invalid transition, `find_expired` |
| `webhook_tests.rs` | `webhook::handle` | ✅ | bad signature → 401, happy path → 200, duplicate event_id, unknown event |
| `service_tests.rs` | `PaymentSvc` (gRPC) | ✅ | create + URL, idempotent replay, idempotency mismatch, get-not-found, cancel, **refund (succeeded only)** |
| `expiry_tests.rs` | `expire_batch` | ✅ | marks expired, returns 0 when nothing to do |

### Shared helpers

`tests/common/mod.rs` provides:

```rust
pub fn test_config() -> PaymentConfig;                 // reads TEST_DATABASE_URL
pub async fn test_state() -> Arc<AppState>;            // runs migrations + truncates
```

Every DB-backed test starts from a clean slate (`TRUNCATE … RESTART IDENTITY CASCADE`).

### Env for tests

| Variable | Default | Purpose |
|---|---|---|
| `TEST_DATABASE_URL` | `postgres://postgres:postgres@localhost:5433/payment_test` | Test-only DB |

---

## Running Tests

```bash
# 1) bring up a throwaway postgres
docker run -d --name pg-test -p 5433:5432 \
  -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=payment_test postgres:16

# 2) run unit tests only (no DB required)
cargo test --test gateway_tests --test config_tests

# 3) run integration tests (require DB)
export TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5433/payment_test"
cargo test --test repo_tests    -- --ignored
cargo test --test webhook_tests -- --ignored
cargo test --test service_tests -- --ignored
cargo test --test expiry_tests  -- --ignored

# 4) run everything
cargo test -- --include-ignored
```

> Tests that touch the DB are marked `#[ignore = "requires DB"]` so `cargo test` in a CI container without a database stays green. Pass `--include-ignored` (or `--ignored`) to run them explicitly.

### Refund coverage

`service_tests.rs` covers the refund path explicitly:

- `refund_requires_succeeded_state` — a `processing` payment must return `FAILED_PRECONDITION`.
- `refund_succeeded_payment_marks_refunded` — after transitioning to `succeeded`, refund returns status `refunded` and writes an outbox event.

### Adding tests

1. Drop a new file `tests/<area>_tests.rs`.
2. `mod common;` at the top.
3. Call `common::test_state().await` — it handles migrations and cleanup.
4. Mark DB-dependent tests with `#[ignore = "requires DB"]`.

---

## Operational Notes

### Idempotency semantics

- If `idempotency_key` is empty, it defaults to `order:<order_id>`.
- Replay with the same key **and same** `order_id` + `amount` returns the stored payment (status included).
- Replay with the same key but different fields → `IdempotencyMismatch` (`INVALID_ARGUMENT`).

### Failure handling

- Gateway create failure → payment transitions to `failed`, outbox event `payment.failed` is emitted, `payment_attempts` records the error.
- Kafka publish failure → the row stays `published_at IS NULL`; the outbox worker retries each second.
- Webhook for an unknown event type → `400`, no state change, no `processed_webhooks` row.

### Observability

- Logs are JSON via `tracing_subscriber::fmt::layer().json()`.
- Sensible defaults: `payment_service=debug,sqlx=warn`.
- Override with `RUST_LOG=...`.

### Shutdown

Both servers are wired to `shutdown_signal()` (Ctrl-C / SIGTERM). Workers are detached tasks and stop when the tokio runtime shuts down.

### Security

- Webhook signatures are verified with **HMAC-SHA256** in **constant time**.
- Amounts use `NUMERIC(14,2)` end-to-end (never floats).
- `WEBHOOK_SECRET` must be rotated by redeploy — there is no hot-reload.

### Scaling

- `payments` and `outbox_events` scale horizontally; the outbox uses `FOR UPDATE SKIP LOCKED` so multiple instances can drain concurrently.
- `expire_batch` likewise uses `FOR UPDATE SKIP LOCKED`.
- `processed_webhooks` PK makes webhook consumption safe across replicas.

---

## License

Internal — do not redistribute.
```

---

## ملاحظات على هذا README

1. **يغطي كل ما في المشروع**: proto, schema, state machine, outbox, workers, error mapping, testing.
2. **قسم Testing مفصّل بجدول test matrix** يوضّح كل ملف وما يغطيه وهل يحتاج DB.
3. **أوامر التشغيل جاهزة للنسخ** (Docker + env + cargo test --ignored).
4. **يوثّق refund tests** التي أضفناها — حتى لا تُنسى مستقبلًا.
5. **يشرح لماذا `#[ignore]`** — لفهم سبب عدم فشل CI بدون DB.
6. **يوثّق أمان التوقيع** (constant-time HMAC) و**idempotency semantics** بدقة.

هل تريد إضافة:

- **OpenAPI/AsyncAPI snippet** لقسم الـ webhook؟
- **mermaid diagrams** بدل ASCII في GitHub؟
- **CHANGELOG.md** و **CONTRIBUTING.md** منفصلين؟