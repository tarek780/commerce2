# README.md — Order Service

```markdown
# Order Service

خدمة إدارة الطلبات (Order Service) المكتوبة بلغة Rust، مبنية على **gRPC** و**PostgreSQL**، وتتبع نمط **Orchestration Saga** مع **Transactional Outbox** للتكامل مع خدمتي `inventory` و`payment` عبر gRPC وKafka.

---

## جدول المحتويات

- [نظرة عامة](#نظرة-عامة)
- [المعمارية](#المعمارية)
- [دورة حياة الطلب](#دورة-حياة-الطلب)
- [المتطلبات](#المتطلبات)
- [التشغيل السريع](#التشغيل-السريع)
- [متغيرات البيئة](#متغيرات-البيئة)
- [الـ APIs (gRPC)](#الـ-apis-grpc)
- [قاعدة البيانات](#قاعدة-البيانات)
- [Transactional Outbox](#transactional-outbox)
- [Kafka Consumer](#kafka-consumer)
- [Expiry Worker](#expiry-worker)
- [هيكل المشروع](#هيكل-المشروع)
- [الاختبارات](#الاختبارات)
- [استكشاف الأخطاء](#استكشاف-الأخطاء)

---

## نظرة عامة

خدمة الطلبات مسؤولة عن:

- إنشاء الطلبات (Create Order) مع دعم **Idempotency**.
- تنسيق **Saga** بين خدمة المخزون (`inventory`) وخدمة الدفع (`payment`).
- إدارة دورة حياة الطلب (`pending → reserved → awaiting_payment → confirmed / failed / cancelled`).
- الاستماع إلى أحداث الدفع من Kafka لتأكيد الطلب أو إفشاله.
- إلغاء الطلبات المنتهية تلقائياً (TTL).
- نشر أحداث الطلب عبر **Outbox Pattern** (جاهزة للـ Kafka).

**التقنيات المستخدمة:**

| المكوّن | التقنية |
|---|---|
| اللغة | Rust (edition 2021) |
| الـ API | gRPC (tonic + prost) |
| قاعدة البيانات | PostgreSQL (sqlx) |
| الأحداث | Kafka (rdkafka) + Transactional Outbox |
| الاتصال بين الخدمات | gRPC |
| Observability | tracing + tracing-subscriber (JSON) |

---

## المعمارية

```
┌─────────────┐   gRPC    ┌─────────────────┐   gRPC    ┌──────────────┐
│   Client    │──────────▶│  Order Service  │──────────▶│  Inventory   │
└─────────────┘           │                 │           └──────────────┘
                          │                 │   gRPC    ┌──────────────┐
                          │                 │──────────▶│   Payment    │
                          └────────┬────────┘           └──────┬───────┘
                                   │                           │
                                   ▼                           ▼
                          ┌─────────────────┐         ┌─────────────────┐
                          │   PostgreSQL    │         │      Kafka      │
                          │  (orders,       │         │  payment.events │
                          │   order_items,  │◀────────│  order.events   │
                          │   outbox_events)│         │                 │
                          └─────────────────┘         └─────────────────┘
```

**نمط Saga (Orchestration):**
- Order Service هو الـ orchestrator.
- كل خطوة إما تنجح أو تُطلق تعويضاً (compensation) للخطوات السابقة.

---

## دورة حياة الطلب

```
                    ┌──────────────────────────────────────────┐
                    │                                          │
  create_order      ▼                                          │
  ──────────────▶ pending ──(reserve ok)──▶ reserved ──(payment ok)──▶ awaiting_payment
                    │                        │                           │
                    │ (reserve fail)         │ (payment fail)            │
                    ▼                        ▼                           ▼
                  failed                   failed         ┌───── payment.succeeded ─────▶ confirmed
                                                          │
                                                          └───── payment.failed ───────▶ failed
                                                          │
                                                          └───── payment.expired ──────▶ failed
                                                          │
                                                          └───── payment.cancelled ────▶ failed

  cancel_order ──▶ cancelled  (من pending | reserved | awaiting_payment فقط)
  TTL expiry   ──▶ cancelled  (نفس الشرط أعلاه)
```

**الحالات النهائية (terminal):** `confirmed`، `failed`، `cancelled`، `expired`.

---

## المتطلبات

- **Rust** ≥ 1.75
- **PostgreSQL** ≥ 14
- **Kafka** ≥ 3.x
- **protoc** (Protocol Buffers compiler)
- خدمتا `inventory` و`payment` شغّالتان (أو على الأقل متاحتان على gRPC)

### تثبيت protoc

```bash
# Ubuntu/Debian
sudo apt install -y protobuf-compiler

# macOS
brew install protobuf
```

---

## التشغيل السريع

### 1) إعداد قاعدة البيانات

```bash
createdb order_service
# أو
psql -c "CREATE DATABASE order_service;"
```

### 2) الإعدادات

انسخ ملف البيئة وعدّله:

```bash
cp .env.example .env
```

### 3) تشغيل الخدمة

```bash
cargo run --release
```

الخدمة ستستمع افتراضياً على `0.0.0.0:50053`.

### 4) اختبار سريع

```bash
grpcurl -plaintext -d '{
  "customer_id": "cust-1",
  "currency": "USD",
  "items": [{"variant_id": 1, "quantity": 2, "unit_price": "19.99"}],
  "idempotency_key": "my-key-1"
}' localhost:50053 order.v1.OrderService/CreateOrder
```

---

## متغيرات البيئة

| المتغير | إلزامي | الافتراضي | الوصف |
|---|---|---|---|
| `DATABASE_URL` | ✅ | — | `postgres://user:pass@host:5432/db` |
| `BIND_ADDRESS` | ❌ | `0.0.0.0:50053` | عنوان استماع gRPC |
| `INVENTORY_GRPC_URL` | ❌ | `http://127.0.0.1:50051` | عنوان خدمة المخزون |
| `PAYMENT_GRPC_URL` | ❌ | `http://127.0.0.1:50052` | عنوان خدمة الدفع |
| `KAFKA_BROKERS` | ❌ | `localhost:9092` | قائمة brokers مفصولة بفواصل |
| `KAFKA_PAYMENT_TOPIC` | ❌ | `payment.events` | topic أحداث الدفع |
| `KAFKA_ORDER_PAYMENT_GROUP` | ❌ | `order-service` | consumer group |
| `ORDER_TTL_SECS` | ❌ | `900` | مدة صلاحية الطلب (ثانية) |
| `ORDER_EXPIRY_SCAN_SECS` | ❌ | `30` | فترة فحص الطلبات المنتهية |
| `RUST_LOG` | ❌ | `order_service=debug,sqlx=warn` | مستوى التسجيل |

مثال `.env`:

```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/order_service
BIND_ADDRESS=0.0.0.0:50053
INVENTORY_GRPC_URL=http://127.0.0.1:50051
PAYMENT_GRPC_URL=http://127.0.0.1:50052
KAFKA_BROKERS=localhost:9092
KAFKA_PAYMENT_TOPIC=payment.events
KAFKA_ORDER_PAYMENT_GROUP=order-service
ORDER_TTL_SECS=900
ORDER_EXPIRY_SCAN_SECS=30
RUST_LOG=order_service=debug,sqlx=warn
```

---

## الـ APIs (gRPC)

الـ proto: [`proto/order/v1/order.proto`](proto/order/v1/order.proto)

### `CreateOrder`

```protobuf
rpc CreateOrder (CreateOrderRequest) returns (OrderResponse);
```

**السلوك:**
1. إنشاء الطلب بحالة `pending`.
2. حجز المخزون (inventory.reserve) لكل عنصر.
3. تغيير الحالة إلى `reserved`.
4. إنشاء دفعة (payment.create).
5. تغيير الحالة إلى `awaiting_payment`.

**Idempotency:** إذا أُرسل `idempotency_key` مكرر يُرجع الخطأ `ALREADY_EXISTS`. إن كان فارغاً يُولَّد واحد تلقائياً.

**أخطاء محتملة:**

| Code | السبب |
|---|---|
| `INVALID_ARGUMENT` | قائمة عناصر فارغة أو `unit_price` غير صالح |
| `ALREADY_EXISTS` | `idempotency_key` مكرر |
| `FAILED_PRECONDITION` | فشل الحجز أو إنشاء الدفعة |
| `INTERNAL` | خطأ قاعدة بيانات |

### `GetOrder`

```protobuf
rpc GetOrder (GetOrderRequest) returns (OrderResponse);
```

يُرجع تفاصيل الطلب. `NOT_FOUND` إن لم يوجد.

### `CancelOrder`

```protobuf
rpc CancelOrder (CancelOrderRequest) returns (OrderResponse);
```

**السلوك:**
1. التحقق من أن الحالة قابلة للإلغاء (`pending | reserved | awaiting_payment`).
2. إلغاء الدفعة (إن وُجدت).
3. تحرير الحجز (إن وُجد).
4. نقل الطلب إلى `cancelled`.

**أخطاء:** `FAILED_PRECONDITION` إن كانت الحالة نهائية.

### `PayOrder`

```protobuf
rpc PayOrder (PayOrderRequest) returns (PayOrderResponse);
```

يُعيد رابط الدفع. مفيد لإعادة توليد الدفعة عند الحاجة.

### استجابة الطلب

```json
{
  "id": "018f...",
  "customer_id": "cust-1",
  "status": "awaiting_payment",
  "total_amount": "39.98",
  "currency": "USD",
  "reservation_id": "res-abc",
  "payment_id": "pay-xyz",
  "created_at": "2024-11-01T12:00:00Z",
  "updated_at": "2024-11-01T12:00:01Z",
  "failure_reason": null
}
```

---

## قاعدة البيانات

### الجداول

- **`orders`**: جدول الطلبات الرئيسي.
- **`order_items`**: عناصر كل طلب.
- **`outbox_events`**: أحداث للـ Kafka (Transactional Outbox).

### الـ migrations

تُطبَّق تلقائياً عند بدء التشغيل:

```rust
sqlx::migrate!("./migrations").run(&state.db).await?;
```

لإضافة migration جديد:

```bash
sqlx migrate add <name>
```

### الفهارس المهمّة

- `idx_orders_status` — للاستعلام السريع بالحالة.
- `idx_orders_expiry` — جزئي على الحالات النشطة + `expires_at` (لعامل الانتهاء).
- `idx_outbox_unpublished` — جزئي على الأحداث غير المنشورة.
- `idx_order_items_order` — لجلب عناصر الطلب.

---

## Transactional Outbox

كل انتقال حالة يكتب حدثاً داخل **نفس المعاملة** إلى `outbox_events`:

```sql
INSERT INTO outbox_events (id, aggregate_type, aggregate_id, event_type, payload)
VALUES ($1, 'order', $2, $3, $4);
```

**أنواع الأحداث:**
- `order.reserved`
- `order.awaiting_payment`
- `order.confirmed`
- `order.failed`
- `order.cancelled`

**Worker النشر** (يعمل كل ثانية):

```rust
outbox::publisher::publish_batch(&state.db).await
```

- يأخذ حتى 100 حدث غير منشور.
- يستخدم `FOR UPDATE SKIP LOCKED` (آمن مع نسخ متعددة).
- يحدّث `published_at`.

> **حالياً** النشر يتم فقط لـ tracing. لربطه بـ Kafka أضف منتج rdkafka داخل `publish_batch` قبل `UPDATE`.

---

## Kafka Consumer

يستهلك `payment.events` ويحدّث الطلب:

| الحدث | التأثير |
|---|---|
| `payment.succeeded` | `awaiting_payment → confirmed` (بعد commit الحجز) |
| `payment.failed` | `awaiting_payment → failed` (بعد release الحجز) |
| `payment.expired` | نفس `failed` |
| `payment.cancelled` | نفس `failed` |

**شكل الحدث:**

```json
{
  "event": "payment.succeeded",
  "payment_id": "pay-xyz",
  "order_id": "018f...",
  "status": "succeeded",
  "reason": null
}
```

**الإعدادات:**
- `enable.auto.commit=false` (commit يدوي بعد المعالجة).
- `auto.offset.reset=earliest`.

---

## Expiry Worker

يعمل كل `ORDER_EXPIRY_SCAN_SECS` ثانية:

1. يجلب الطلبات المنتهية (`pending | reserved | awaiting_payment` و`expires_at < now()`).
2. يستخدم `FOR UPDATE SKIP LOCKED` (آمن مع نسخ متعددة).
3. يستدعي `orchestrator::cancel(..., "expired_ttl")`.

النتيجة: `status = cancelled`, `failure_reason = expired_ttl`.

---

## هيكل المشروع

```
.
├── Cargo.toml
├── build.rs                    # يبني .proto
├── proto/
│   ├── order/v1/order.proto
│   ├── inventory/v1/inventory.proto
│   └── payment/v1/payment.proto
├── migrations/
│   └── 0001_init.sql
├── src/
│   ├── main.rs                 # bootstrap
│   ├── lib.rs                  # exports للاختبارات
│   ├── config.rs               # OrderConfig::from_env
│   ├── error.rs                # OrderError + From<OrderError> for Status
│   ├── state.rs                # AppState (db + clients + cfg)
│   ├── clients/
│   │   ├── mod.rs
│   │   ├── inventory.rs        # InventoryClient (gRPC)
│   │   └── payment.rs          # PaymentClient (gRPC)
│   ├── kafka/
│   │   ├── mod.rs
│   │   └── consumer.rs         # payment.events consumer
│   ├── order/
│   │   ├── mod.rs
│   │   ├── orchestrator.rs     # saga steps
│   │   ├── repo.rs             # OrderRepo (SQL)
│   │   ├── service.rs          # gRPC OrderService impl
│   │   └── expiry.rs           # expiry worker
│   └── outbox/
│       ├── mod.rs
│       └── publisher.rs        # publish_batch
└── tests/
    ├── common/mod.rs           # helpers (setup_pool, cleanup, setup_state)
    ├── repo_tests.rs
    ├── orchestrator_tests.rs
    ├── expiry_tests.rs
    ├── outbox_tests.rs
    └── service_tests.rs
```

---

## الاختبارات

### الاختبارات الحالية

| ملف | يغطي |
|---|---|
| `tests/repo_tests.rs` | `OrderRepo`: insert, get, transition, duplicate key, list_expired |
| `tests/orchestrator_tests.rs` | create/cancel flow (بدون خدمات حقيقية) + happy path (ignored) |
| `tests/expiry_tests.rs` | `expiry::expire_batch` |
| `tests/outbox_tests.rs` | `outbox::publisher::publish_batch` |
| `tests/service_tests.rs` | gRPC service: GetOrder, CreateOrder, CancelOrder (status codes) |

### المتطلبات

- **PostgreSQL** اختبارية.
- `TEST_DATABASE_URL` مضبوط.

### الإعداد

```bash
createdb order_test
export TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5432/order_test"
```

### التحضير للاختبارات (مهم)

لتتمكن الاختبارات من استيراد الوحدات، أنشئ `src/lib.rs`:

```rust
pub mod clients;
pub mod config;
pub mod error;
pub mod kafka;
pub mod order;
pub mod outbox;
pub mod state;

pub mod pb_order { tonic::include_proto!("order.v1"); }
pub mod pb_inv   { tonic::include_proto!("inventory.v1"); }
pub mod pb_pay   { tonic::include_proto!("payment.v1"); }
```

وفي `src/main.rs` استبدل تعريفات `mod` و`pb_*` بـ:

```rust
use order_service::{
    config::OrderConfig,
    order, kafka, outbox,
    pb_order::order_service_server::OrderServiceServer,
    state::AppState,
};
```

> **ملاحظة:** إذا كان اسم الحزمة في `Cargo.toml` هو `order-service` فإن المكتبة ستُستورد بـ `order_service`.

### تشغيل الاختبارات

```bash
# كل الاختبارات القابلة للتشغيل محلياً (بدون خدمات gRPC)
cargo test --tests -- --test-threads=1

# اختبار ملف محدد
cargo test --test repo_tests -- --test-threads=1

# اختبارات تحتاج inventory + payment شغّالتين
cargo test --tests -- --ignored --test-threads=1
```

> **`--test-threads=1`** موصى به لأن الاختبارات تستخدم `TRUNCATE` على نفس قاعدة البيانات. للعزل الكامل استخدم `#[sqlx::test]` (يُنشئ DB جديدة لكل اختبار).

### تشغيل خدمات التكامل (اختياري)

لتشغيل اختبار `happy_path_full_orchestration` (والمعلَّم `#[ignore]`):

```bash
# تأكد أن inventory على :50051 وpayment على :50052
export TEST_INVENTORY_URL=http://127.0.0.1:50051
export TEST_PAYMENT_URL=http://127.0.0.1:50052
cargo test --test orchestrator_tests -- --ignored --test-threads=1
```

### معاينة الاختبارات

```
repo_tests
  ✓ insert_and_get_order
  ✓ duplicate_idempotency_key_returns_duplicate
  ✓ get_missing_returns_not_found
  ✓ transition_pending_to_reserved
  ✓ transition_from_wrong_state_fails
  ✓ list_expired_returns_only_due_orders

orchestrator_tests
  ✓ empty_items_rejected
  ✓ cancel_final_state_rejected
  ✓ cancel_pending_succeeds_without_clients
  ⊘ happy_path_full_orchestration  (ignored)

expiry_tests
  ✓ expire_batch_marks_due_orders_cancelled
  ✓ expire_batch_skips_not_due

outbox_tests
  ✓ publish_batch_marks_events_published
  ✓ publish_batch_empty_returns_zero

service_tests
  ✓ get_order_not_found_returns_status
  ✓ get_order_returns_fields
  ✓ create_order_empty_items_invalid_argument
  ✓ create_order_bad_price_invalid_argument
  ✓ cancel_order_final_state_returns_failed_precondition
  ✓ cancel_order_pending_success
```

---

## استكشاف الأخطاء

### `TEST_DATABASE_URL must be set`

```bash
export TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5432/order_test"
```

### `error: failed to run custom build command` (build.rs)

`protoc` غير مثبت — راجع [المتطلبات](#المتطلبات).

### `unable to connect to Kafka`

بعد بدء التشغيل سيحاول الـ consumer الاتصال. إذا كانت Kafka غير متاحة، سترى أخطاء `kafka recv` متكررة لكن gRPC يستمر بالعمل.

### اختبارات `service_tests` تفشل بسبب `FailedPrecondition`

هذا متوقع في اختبار `create_order_empty_items_invalid_argument` — الـ status الناتج هو `FAILED_PRECONDITION` (من `OrderError::InvalidState`). إذا أردت `INVALID_ARGUMENT` أضف variant مستقل للخطأ:

```rust
#[error("invalid argument: {0}")]
InvalidArgument(String),
```

---

## الترخيص

[MIT](LICENSE) — عدّله حسب سياسة فريقك.
```