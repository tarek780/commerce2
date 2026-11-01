# 📦 Inventory Service

خدمة إدارة مخزون مكتوبة بـ **Rust** تعتمد على **Tonic (gRPC)** و**PostgreSQL**،
مع نمط **Outbox** لنشر الأحداث، وعامل خلفي لتحرير الحجوزات المنتهية، وبثّ لحظي لأسهم المخزون.

---

## 📑 المحتويات

1. [الميزات](#-الميزات)
2. [البنية المعمارية](#-البنية-المعمارية)
3. [دورة حياة الحجز](#-دورة-حياة-الحجز)
4. [هيكل المشروع](#-هيكل-المشروع)
5. [المتطلبات](#-المتطلبات)
6. [الإعداد والتشغيل](#-الإعداد-والتشغيل)
7. [متغيرات البيئة](#-متغيرات-البيئة)
8. [gRPC API](#-grpc-api)
9. [دليل قاعدة البيانات](#-دليل-قاعدة-البيانات)
10. [اختبارات الخدمة](#-اختبارات-الخدمة)
11. [استكشاف الأخطاء](#-استكشاف-الأخطاء)

---

## ✨ الميزات

- **حجز مخزون idempotent**: مفتاح `reservation_key` فريد؛ تكرار الطلب بنفس المعاملات يُعيد نفس `reservation_id`.
- **Reserve / Release / Commit**: دورة حياة كاملة مع حالات (`pending`, `released`, `committed`).
- **التحرير التلقائي** للحجوزات المنتهية عبر عامل خلفي كل 30 ثانية.
- **بثّ لحظي** لتحديثات المخزون عبر `watch_stock` (server-streaming gRPC).
- **Outbox Pattern**: كل تغيير يُسجَّل في `outbox_events` ثم يُنشر من عامل منفصل كل ثانية.
- **Inventory Logs**: أثر كامل لكل عملية (`reserve`, `release`, `commit`, `adjust`, `expire`).
- **قفل صفّي آمن**: `SELECT ... FOR UPDATE` + `SKIP LOCKED` لمنع التسابق.
- **قفل ترتيب موحّد**: reservations → inventory لتجنّب deadlocks.

---

## 🏗 البنية المعمارية

```
┌──────────────────┐    gRPC/Protobuf    ┌──────────────────┐
│  gRPC Client     │ ──────────────────► │  Tonic Server    │
└──────────────────┘                     └────────┬─────────┘
                                                  │
                                    ┌─────────────┼─────────────┐
                                    ▼             ▼             ▼
                             ┌───────────┐ ┌───────────┐ ┌───────────┐
                             │ Inventory │ │  Stock    │ │  Outbox   │
                             │ Service   │ │Broadcast  │ │  Worker   │
                             └─────┬─────┘ └─────┬─────┘ └─────┬─────┘
                                   │             │             │
                                   ▼             │             │
                             ┌───────────┐       │             │
                             │  Repo     │       │             │
                             │ (SQLx Tx) │       │             │
                             └─────┬─────┘       │             │
                                   │             │             │
                                   ▼             ▼             ▼
                             ┌──────────────────────────────────┐
                             │          PostgreSQL              │
                             │  inventory | reservations |       │
                             │  inventory_logs | outbox_events  │
                             └──────────────────────────────────┘
                                                ▲
                                                │
                                     ┌──────────────────┐
                                     │  Expiry Worker   │
                                     │  (كل 30 ثانية)   │
                                     └──────────────────┘
```

**الأنماط المستخدمة:**
- **Transactional Outbox** — ضمان نشر الأحداث مرة واحدة على الأقل.
- **Optimistic vs Pessimistic** — نستخدم pessimistic (`FOR UPDATE`) لتجنّب تعارضات الكتابة.
- **Background Workers** — Expiry وOutbox كـ `tokio::spawn` مستقلّين.
- **Broadcast Channel** — بث داخلي للمشتركين (`watch_stock`).
- **Saga-friendly** — `reservation_key` + `reference_id` يسمحان بالتنسيق مع خدمات أخرى.

---

## 🔄 دورة حياة الحجز

```
                 reserve()
                    │
                    ▼
              ┌──────────┐
              │ pending  │──── commit() ──► ┌───────────┐
              └────┬─────┘                   │ committed │
                   │                         └───────────┘
                   │
        ┌──────────┼──────────┐
        │                     │
  release()             expires_at < now
        │                     │
        ▼                     ▼
   ┌──────────┐          ┌──────────┐
   │ released │          │ released │
   └──────────┘          └──────────┘
```

| العملية | stock | reserved | سجل |
|---|---|---|---|
| `reserve(q)` | — | **+q** | `reserve -q` |
| `commit(q)` | **-q** | **-q** | `commit -q` |
| `release(q)` | — | **-q** | `release +q` |
| `expire(q)` | — | **-q** | `expire +q` |
| `adjust(d)` | **+d** | — | `adjust d` |

> `available = stock - reserved` دائماً.

---

## 📂 هيكل المشروع

```
inventory-service/
├── Cargo.toml
├── build.rs                  # tonic-build لبروتوكول gRPC
├── proto/
│   └── inventory.proto       # تعريف gRPC service
├── migrations/
│   └── 0001_inventory.sql
├── .env.example
├── src/
│   ├── main.rs               # نقطة الانطلاق + workers
│   ├── lib.rs                # تصدير الوحدات للاختبارات
│   ├── error.rs              # InventoryError + Into<tonic::Status>
│   ├── inventory/
│   │   ├── repo.rs           # InventoryRepo (كل استعلامات SQL)
│   │   ├── service.rs        # InventorySvc (تنفيذ الـ trait المولّد)
│   │   └── expiry.rs         # expire_batch worker
│   └── outbox/
│       └── publisher.rs      # publish_batch worker
└── tests/
    ├── common/mod.rs         # build_env, seed, cleanup
    ├── repo_reserve.rs       # reserve/release/idempotency
    ├── repo_commit.rs        # commit/adjust/expired
    ├── expiry.rs             # expire_batch
    ├── outbox.rs             # publish_batch
    ├── grpc_service.rs       # reserve/commit/adjust/getStock عبر gRPC
    ├── grpc_watch.rs         # watch_stock streaming
    └── error_mapping.rs      # InventoryError → tonic::Status
```

---

## 🧰 المتطلبات

- Rust **1.75+**
- PostgreSQL **14+**
- `protoc` (Protocol Buffers compiler) — يتطلبه `tonic-build`
- `sqlx-cli` (اختياري)

### تثبيت `protoc`

```bash
# Ubuntu/Debian
sudo apt-get install -y protobuf-compiler

# macOS
brew install protobuf

# أو عبر cargo (لا يُنصح في الإنتاج)
# لا يوجد بديل رسمي؛ استخدم مدير الحزم
```

---

## 🚀 الإعداد والتشغيل

### 1. تهيئة البيئة

```bash
git clone <repo-url> inventory-service
cd inventory-service
cp .env.example .env
```

`.env`:

```env
DATABASE_URL=postgres://postgres:pass@localhost:5432/inventory
BIND_ADDR=0.0.0.0:50051
RUST_LOG=inventory_service=debug,sqlx=warn
```

### 2. تشغيل Postgres

```bash
docker run -d --name inv-pg \
  -e POSTGRES_PASSWORD=pass \
  -e POSTGRES_DB=inventory \
  -p 5432:5432 postgres:16
```

### 3. بناء وتشغيل

```bash
cargo build --release
cargo run --release
```

السجل المتوقع:

```
INFO migrations applied
INFO addr=0.0.0.0:50051 starting gRPC
```

ستُشغَّل خلف الكواليس:
- **Expiry worker**: كل 30 ثانية يحرّر الحجوزات المنتهية.
- **Outbox worker**: كل ثانية ينشر الأحداث المعلّقة.

### 4. اختبار سريع مع `grpcurl`

```bash
brew install grpcurl   # أو من صفحة المشروع

grpcurl -plaintext \
  -d '{"variant_id": 42}' \
  localhost:50051 inventory.v1.InventoryService/GetStock
```

---

## 🔧 متغيرات البيئة

| المتغير | إلزامي | الافتراضي | الوصف |
|---|---|---|---|
| `DATABASE_URL` | ✅ | — | رابط Postgres |
| `BIND_ADDR` | ❌ | `0.0.0.0:50051` | عنوان gRPC |
| `RUST_LOG` | ❌ | `inventory_service=debug,sqlx=warn` | مستوى التسجيل |

---

## 📡 gRPC API

**الحزمة:** `inventory.v1`
**الخدمة:** `InventoryService`

### `Reserve`

```protobuf
rpc Reserve(ReserveRequest) returns (ReserveResponse);

message ReserveRequest {
  string reservation_key = 1; // مفتاح فريد للـ idempotency
  int32  variant_id       = 2;
  int32  quantity         = 3; // > 0
  int64  ttl_seconds      = 4; // مدة الحجز
}

message ReserveResponse {
  string reservation_id    = 1;
  int32  variant_id        = 2;
  int32  quantity          = 3;
  int64  expires_at_unix   = 4;
}
```

**السلوك:**
- `reservation_key` جديد + مخزون كافٍ → حجز جديد.
- `reservation_key` موجود بنفس `(variant_id, quantity)` → يُعيد نفس `reservation_id` (idempotent).
- `reservation_key` موجود بمعاملات مختلفة → `INVALID_ARGUMENT`.
- مخزون غير كافٍ → `FAILED_PRECONDITION` مع `available` في الرسالة.

---

### `Release`

```protobuf
rpc Release(ReleaseRequest) returns (ReleaseResponse);
```

يحرّر الحجز (`pending` → `released`). إن لم يكن `pending` → `FAILED_PRECONDITION invalid state`.

---

### `Commit`

```protobuf
rpc Commit(CommitRequest) returns (CommitResponse);
```

يحوّل `pending` → `committed`، مع:
- `stock -= quantity` و`reserved -= quantity`.
- إن انتهت المدة → `FAILED_PRECONDITION reservation expired`.

---

### `GetStock`

```protobuf
rpc GetStock(GetStockRequest) returns (GetStockResponse);

message GetStockResponse {
  int32 stock_quantity     = 1;
  int32 reserved_quantity  = 2;
  int32 available_quantity = 3; // stock - reserved
}
```

---

### `AdjustStock`

```protobuf
rpc AdjustStock(AdjustStockRequest) returns (AdjustStockResponse);
```

`delta` موجب أو سالب؛ يُرفض إن جعل `stock < 0`.

---

### `WatchStock` (server streaming)

```protobuf
rpc WatchStock(WatchStockRequest) returns (stream StockUpdate);

message WatchStockRequest {
  repeated int32 variant_ids        = 1; // فارغة = الكل
  int32          low_stock_threshold = 2; // افتراضي 1
}

message StockUpdate {
  int32 variant_id         = 1;
  int32 available_quantity = 2;
  int32 reserved_quantity  = 3;
  string event_type        = 4; // reserved | released | committed | adjusted | expired
  int64 updated_at_unix    = 5;
}
```

**منطق التصفية:**
- إن حُدّدت `variant_ids` → الأحداث الخاصة بها فقط.
- يُبثّ الحدث إن كانت `available <= threshold` **أو** كانت `event_type` من الأحداث المهمّة.

---

### جدول أكواد الأخطاء

| `InventoryError` | `tonic::Code` | الرسالة |
|---|---|---|
| `VariantNotFound` | `NOT_FOUND` | `variant not found` |
| `ReservationNotFound` | `NOT_FOUND` | `reservation not found` |
| `DuplicateReservation` | `ALREADY_EXISTS` | `duplicate reservation_key` |
| `IdempotencyMismatch` | `INVALID_ARGUMENT` | `reservation_key exists with different params` |
| `InsufficientStock` | `FAILED_PRECONDITION` | `insufficient stock, available: N` |
| `ReservationExpired` | `FAILED_PRECONDITION` | `reservation expired` |
| `InvalidState` | `FAILED_PRECONDITION` | `invalid state: <state>` |
| `Database` | `INTERNAL` | `database error` |

---

## 🗄 دليل قاعدة البيانات

### `inventory`

| العمود | النوع | ملاحظات |
|---|---|---|
| `variant_id` | `INT PK` | مرجع المتغيّر |
| `stock_quantity` | `INT` | ≥ 0 |
| `reserved_quantity` | `INT` | ≥ 0 |
| `updated_at` | `TIMESTAMPTZ` | |

### `reservations`

| العمود | النوع | ملاحظات |
|---|---|---|
| `id` | `UUID PK` | v7 |
| `reservation_key` | `TEXT UNIQUE` | idempotency |
| `variant_id` | `INT` | |
| `quantity` | `INT` | > 0 |
| `status` | `TEXT` | `pending` \| `released` \| `committed` |
| `expires_at` | `TIMESTAMPTZ` | |
| `created_at` | `TIMESTAMPTZ` | |

### `inventory_logs`

| العمود | النوع | ملاحظات |
|---|---|---|
| `id` | `BIGSERIAL PK` | |
| `sku` | `TEXT` | `variant_id` كنص |
| `change_amount` | `INT` | موجب/سالب |
| `type` | `TEXT` | `reserve` \| `release` \| `commit` \| `adjust` \| `expire` |
| `reference_id` | `TEXT` | `reservation_key`/`reservation_id`/`reference_id` |

### `outbox_events`

| العمود | النوع | ملاحظات |
|---|---|---|
| `id` | `UUID PK` | |
| `aggregate_type` | `TEXT` | `inventory` |
| `aggregate_id` | `BIGINT` | `variant_id` |
| `event_type` | `TEXT` | `reserved`, ... |
| `payload` | `JSONB` | الحمولة |
| `published_at` | `TIMESTAMPTZ` | `NULL` = غير منشور |
| `created_at` | `TIMESTAMPTZ` | |

### استعلامات تشخيص

```sql
-- مخزون كل variant
SELECT variant_id, stock_quantity, reserved_quantity,
       stock_quantity - reserved_quantity AS available
FROM inventory ORDER BY variant_id;

-- حجوزات معلّقة منتهية
SELECT id, variant_id, quantity, expires_at
FROM reservations
WHERE status = 'pending' AND expires_at < now();

-- أحداث غير منشورة
SELECT id, event_type, payload, created_at
FROM outbox_events WHERE published_at IS NULL ORDER BY created_at;
```

---

## 🧪 اختبارات الخدمة

### ⚙️ تحضير البيئة

الاختبارات تتطلب **PostgreSQL فقط** (لا Redis). تُتخطّى تلقائياً إن غاب `DATABASE_URL`.

```bash
docker run -d --name inv-pg-test \
  -e POSTGRES_PASSWORD=pass \
  -e POSTGRES_DB=inventory_test \
  -p 5433:5432 postgres:16

export DATABASE_URL=postgres://postgres:pass@localhost:5433/inventory_test
```

> ⚠️ **مهم**: المشروع يعرّض الوحدات من `src/lib.rs`. تأكد من أن `main.rs` يستورد من اسم الحزمة (`