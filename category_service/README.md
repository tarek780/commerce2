# README.md

```markdown
# Category Service

خدمة **كتالوج المنتجات والفئات** مبنية بـ Rust، تقدّم:
- **GraphQL API** (Query + Mutation) عبر `async-graphql` و `axum`
- **PostgreSQL** كمصدر حقيقة (source of truth) للبيانات
- **Redis Stack** (RedisJSON + RediSearch) كطبقة قراءة سريعة وفهارس بحث
- **Redis Streams** كناقل أحداث (event bus) بين طلبات الكتابة والتنفيذ الفعلي
- **gRPC Client** لخدمة `inventory.v1` (حجز/تحرير مخزون)

الهدف المعماري: **فصل الكتابة عن القراءة (CQRS خفيف)** — طلبات الـ Mutation تُنشر كأحداث إلى Redis Stream، ثم مستهلك (consumer) يعالجها بشكل غير متزامن: يكتب إلى Postgres ويحدّث نسخة RedisJSON.

---

## جدول المحتويات

- [المتطلبات](#المتطلبات)
- [البنية العامة](#البنية-العامة)
- [هيكل المشروع](#هيكل-المشروع)
- [الإعداد والتشغيل](#الإعداد-والتشغيل)
- [متغيرات البيئة](#متغيرات-البيئة)
- [المخطط البياني GraphQL](#المخطط-البياني-graphql)
- [تدفّق الكتابة (Write Flow)](#تدفّق-الكتابة-write-flow)
- [تدفّق القراءة (Read Flow)](#تدفّق-القراءة-read-flow)
- [نموذج البيانات](#نموذج-البيانات)
- [فهارس Redis Search](#فهارس-redis-search)
- [Redis Streams](#redis-streams)
- [gRPC Inventory](#grpc-inventory)
- [الاختبارات](#الاختبارات)
- [تشغيل الاختبارات](#تشغيل-الاختبارات)
- [CI المقترح](#ci-المقترح)
- [قائمة TODO / تحسينات مستقبلية](#قائمة-todo--تحسينات-مستقبلية)

---

## المتطلبات

| المكوّن | الإصدار | ملاحظات |
|---|---|---|
| Rust | 1.82+ (edition 2024) | `rustup toolchain install stable` |
| PostgreSQL | 16+ | لا بد من تطبيق الـ migrations |
| Redis Stack | latest | يحوي RedisJSON + RediSearch + Streams |
| Inventory gRPC Service | — | يجب أن يكون متاحاً على `INVENTORY_GRPC_URL` |
| protoc | 3.20+ | لبناء gRPC stubs عبر `build.rs` |

> **مهم**: Redis العادي (بدون Stack) **لا يكفي**؛ ستفشل الأوامر `JSON.SET` و `FT.CREATE`.

---

## البنية العامة

```
                        ┌─────────────────────────┐
                        │      Client (HTTP)      │
                        └───────────┬─────────────┘
                                    │ POST /graphql
                                    ▼
                        ┌─────────────────────────┐
                        │   axum + async-graphql  │
                        └───────┬─────────┬───────┘
                                │         │
                     Mutation   │         │   Query
                                ▼         ▼
                 ┌───────────────────┐  ┌────────────────────┐
                 │  XADD (Streams)   │  │  FT.SEARCH (Redis) │
                 └─────────┬─────────┘  │  JSON.GET (Redis)  │
                           │            └─────────┬──────────┘
                           │                      │ miss
                           ▼                      ▼
                 ┌───────────────────┐  ┌────────────────────┐
                 │  Consumer worker  │  │   PostgreSQL       │
                 │  (XREADGROUP)     │◄─┤  (source of truth) │
                 └─────────┬─────────┘  └────────────────────┘
                           │
                           ├──► JSON.SET (update read cache)
                           └──► gRPC inventory.EnsureSku
```

---

## هيكل المشروع

```
.
├── Cargo.toml
├── build.rs                    # يبني gRPC client من proto/inventory.proto
├── proto/
│   ├── inventory.proto
│   └── order.proto
├── migrations/
│   └── 20261024090541_category.sql
├── src/
│   ├── lib.rs                  # يعيد تصدير كل الوحدات (لتمكين الاختبارات)
│   ├── main.rs                 # تهيئة الخدمة + تشغيل HTTP + worker
│   ├── config.rs               # AppConfig::from_env
│   ├── error.rs                # AppError + تحويلات tonic/async-graphql
│   ├── state.rs                # AppState (db, redis, inventory, cfg)
│   ├── db/
│   │   ├── mod.rs
│   │   ├── pool.rs             # PgPool
│   │   └── impls/
│   │       ├── category.rs     # CategoryRepo
│   │       ├── product.rs      # ProductRepo
│   │       └── variant.rs      # VariantRepo
│   ├── graphql/
│   │   ├── mod.rs              # router (axum)
│   │   ├── schema.rs           # Schema<Query, Mutation, Empty>
│   │   ├── query.rs            # QueryRoot
│   │   ├── mutation.rs         # MutationRoot
│   │   ├── context.rs          # GqlContext
│   │   ├── pagination.rs       # PageInput, PageInfo, ProductFilterInput
│   │   ├── types/
│   │   │   ├── category.rs
│   │   │   ├── product.rs
│   │   │   ├── variant.rs
│   │   │   └── connection.rs   # ProductConnection
│   │   └── scalars.rs
│   ├── redis/
│   │   ├── mod.rs
│   │   ├── client.rs           # ConnectionManager
│   │   ├── json.rs             # JSON.SET/GET/DEL
│   │   ├── search.rs           # FT.CREATE + FT.SEARCH
│   │   └── stream.rs           # CatalogEvent + publish
│   ├── stream_worker/
│   │   ├── mod.rs
│   │   └── consumer.rs         # XREADGROUP → handle_event
│   ├── grpc/
│   │   ├── mod.rs
│   │   └── inventory.rs        # InventoryClient
│   ├── helpers/
│   │   ├── mod.rs
│   │   └── cursor.rs           # Cursor { id } encode/decode
│   ├── middleware/
│   │   └── auth.rs             # Role + AccessToken
│   └── outbox/
│       └── publisher.rs        # placeholder (نشر الأحداث لاحقاً)
└── tests/
    ├── common/
    │   ├── mod.rs
    │   ├── db.rs               # setup pool + migrate + truncate
    │   └── redis.rs            # setup ConnectionManager + uniq keys
    ├── unit_cursor.rs
    ├── unit_pagination.rs
    ├── unit_config.rs
    ├── unit_error.rs
    ├── unit_stream_events.rs
    ├── unit_query_helpers.rs
    ├── unit_variant_type.rs
    ├── integration_category_repo.rs
    ├── integration_product_repo.rs
    ├── integration_variant_repo.rs
    ├── integration_redis_json.rs
    ├── integration_redis_search.rs
    ├── integration_redis_stream.rs
    └── integration_http.rs
```

> **لتصدير `pub mod` لكل الوحدات من `lib.rs`** — هذا ضروري لأن كل ملف داخل `tests/` هو **crate مستقل** يجب أن يستورد من `category_service::...`.

---

## الإعداد والتشغيل

### 1. تشغيل الخدمات التابعة

```bash
# PostgreSQL
docker run -d --name pg \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=catalog \
  -p 5432:5432 \
  postgres:16

# Redis Stack
docker run -d --name redis \
  -p 6379:6379 \
  redis/redis-stack:latest
```

### 2. متغيرات البيئة

أنشئ `.env`:

```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/catalog
REDIS_URL=redis://127.0.0.1:6379
REDIS_STREAM_KEY=catalog:events
REDIS_CONSUMER_GROUP=catalog-workers
REDIS_CONSUMER_NAME=worker-1
INVENTORY_GRPC_URL=http://127.0.0.1:50051
BIND_ADDRESS=0.0.0.0:8084
REDIS_INDEX_PRODUCTS=catalog:products:idx
REDIS_INDEX_CATEGORIES=catalog:categories:idx
RUST_LOG=category_service=debug,sqlx=warn
```

### 3. تشغيل الخدمة

```bash
cargo run
# يستمع على http://0.0.0.0:8084/graphql
```

افتح `http://localhost:8084/graphql` → Playground جاهز.

### 4. Migrations

تُطبّق تلقائياً عند بدء التشغيل (`sqlx::migrate!`).

يدوياً:

```bash
sqlx migrate run --database-url "$DATABASE_URL"
```

---

## متغيرات البيئة

| المتغير | إلزامي | افتراضي | الوصف |
|---|---|---|---|
| `DATABASE_URL` | ✅ | — | اتصال Postgres |
| `REDIS_URL` | ✅ | — | اتصال Redis |
| `REDIS_STREAM_KEY` | ✅ | — | اسم الـ stream |
| `REDIS_CONSUMER_GROUP` | ✅ | — | مجموعة المستهلك |
| `REDIS_CONSUMER_NAME` | ❌ | `worker-1` | اسم المستهلك الفريد |
| `INVENTORY_GRPC_URL` | ✅ | — | عنوان خدمة المخزون |
| `BIND_ADDRESS` | ❌ | `0.0.0.0:8084` | عنوان HTTP |
| `REDIS_INDEX_PRODUCTS` | ❌ | `catalog:products:idx` | اسم فهرس المنتجات |
| `REDIS_INDEX_CATEGORIES` | ❌ | `catalog:categories:idx` | اسم فهرس الفئات |

---

## المخطط البياني GraphQL

### Query

```graphql
type QueryRoot {
  products(filter: ProductFilterInput, page: PageInput): ProductConnection!
  product(id: ID!): Product!
  category(id: ID!): Category!
  categories(parentId: Int): [Category!]!
}

input ProductFilterInput {
  categoryId: Int
  brand: String
  search: String
  isActive: Boolean
  minPrice: Float
  maxPrice: Float
}

input PageInput {
  first: Int      # 1..=100 (default 20)
  after: String   # Base64(id)
}

type ProductConnection {
  edges: [ProductEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type ProductEdge { cursor: String!, node: Product! }

type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}

type Product {
  id: ID!
  categoryId: ID
  name: String!
  slug: String!
  description: String
  brand: String
  isActive: Boolean!
  createdAt: DateTime!
  updatedAt: DateTime!
  variants: [Variant!]!        # resolved lazily من DB
}

type Category {
  id: ID!
  name: String!
  slug: String!
  parentId: ID
  createdAt: DateTime!
  updatedAt: DateTime!
  children: [Category!]!       # resolved lazily من DB
}

type Variant {
  id: ID!
  productId: ID!
  sku: String!
  price: String!               # BigDecimal → String
  compareAtPrice: String
  imageUrl: String
  attributes: JSON!
  isActive: Boolean!
  createdAt: DateTime!
  updatedAt: DateTime!
}
```

### Mutation

كل المهام تتطلب `Role::Admin` (يُحقن عبر `middleware/auth.rs` كـ `AccessToken` في السياق).

```graphql
type MutationRoot {
  createCategory(input: CreateCategoryInput!): Boolean!
  updateCategory(id: ID!, name: String, slug: String, parentId: Int): Boolean!
  deleteCategory(id: ID!): Boolean!

  createProduct(input: CreateProductInput!): Boolean!
  updateProduct(
    id: ID!,
    categoryId: Int,
    name: String,
    slug: String,
    description: String,
    brand: String,
    isActive: Boolean,
  ): Boolean!
  deleteProduct(id: ID!): Boolean!

  createVariant(input: CreateVariantInput!): Boolean!
}

input CreateCategoryInput { name: String!, slug: String, parentId: Int }
input CreateProductInput {
  categoryId: Int
  name: String!
  slug: String
  description: String
  brand: String
  isActive: Boolean
}
input CreateVariantInput {
  productId: Int!
  sku: String!
  price: String!
  compareAtPrice: String
  imageUrl: String
  attributes: JSON!
  isActive: Boolean
}
```

### أمثلة استعلام

```graphql
# قائمة منتجات مع فلترة
query {
  products(
    filter: { categoryId: 1, isActive: true, search: "phone" }
    page: { first: 20 }
  ) {
    edges {
      cursor
      node { id name slug brand variants { sku price } }
    }
    pageInfo { hasNextPage endCursor }
    totalCount
  }
}
```

```graphql
# إنشاء منتج (يتطلب Admin)
mutation {
  createProduct(input: {
    categoryId: 1
    name: "iPhone 16"
    description: "Flagship"
    brand: "Apple"
  })
}
```

---

## تدفّق الكتابة (Write Flow)

1. **الاستلام**: `Mutation` يستقبل الطلب من HTTP.
2. **التحقق**: يتحقق من `Role::Admin`.
3. **النشر**: يبني `CatalogEvent` ويستدعي `publish()` → `XADD` إلى `REDIS_STREAM_KEY`.
4. **الاستجابة**: يرجع `true` فوراً (الكتابة الفعلية غير متزامنة).
5. **المستهلك**: `stream_worker::consumer` يقرأ الأحداث عبر `XREADGROUP`.
6. **التنفيذ**:
   - يكتب إلى Postgres عبر `*Repo`.
   - يحدّث `JSON.SET` في Redis (`product:{id}` أو `category:{id}`).
   - عند `ProductUpdated` مع `sku` → ينشئ variant ويستدعي `inventory.ensure_sku`.
7. **ACK**: `XACK` بعد النجاح فقط؛ عند الفشل يبقى الحدث معلّقاً (retry).

### حالات خاصة

| الحدث | ما يفعله المستهلك |
|---|---|
| `CategoryCreated` | `CategoryRepo::insert` + `JSON.SET category:{id}` |
| `CategoryUpdated` | `CategoryRepo::update` + `JSON.SET` |
| `CategoryDeleted` | `CategoryRepo::delete` + `JSON.DEL` |
| `ProductCreated` | `ProductRepo::insert` + `JSON.SET product:{id}` |
| `ProductUpdated` (مع sku) | `ProductRepo::update` + `VariantRepo::insert` + `ensure_sku` |
| `ProductUpdated` (بدون sku) | `ProductRepo::update` فقط |
| `ProductDeleted` | `ProductRepo::delete` + `JSON.DEL` |

---

## تدفّق القراءة (Read Flow)

1. **Query `products`**:
   - يبني استعلام RediSearch من `ProductFilterInput`.
   - `FT.SEARCH` على `REDIS_INDEX_PRODUCTS`.
   - إن كان عدد النتائج أقل من المطلوب → يعبّئ الباقي من Postgres (fallback) ويُحدّث Redis (`JSON.SET`).
   - يرجع `ProductConnection` مع cursors (Base64 لـ `id`).
2. **Query `product(id)` / `category(id)`**:
   - `JSON.GET product:{id}`.
   - عند miss → `ProductRepo::get` + `JSON.SET`.
3. **Query `categories`**:
   - `FT.SEARCH` على `REDIS_INDEX_CATEGORIES` عبر `parent_id`.
   - عند miss → `CategoryRepo::list`.

---

## نموذج البيانات

```sql
categories(id, name, slug UNIQUE, parent_id→categories, created_at, updated_at)
products(id, category_id→categories, name, slug UNIQUE, description, brand, is_active, ...)
product_variants(id, product_id→products ON DELETE CASCADE, sku UNIQUE, price NUMERIC(14,2), ...)
outbox_events(id UUID, aggregate_type, aggregate_id, event_type, payload JSONB, ...)
```

- حذف فئة: `ON DELETE SET NULL` لأبنائها، لكن `products.category_id` يمنع الحذف (`ON DELETE RESTRICT`).
- حذف منتج: يحذف كل الـ variants (CASCADE).

---

## فهارس Redis Search

### `catalog:products:idx`

```text
ON JSON PREFIX 1 product:
SCHEMA
  $.name         AS name         TEXT
  $.brand        AS brand        TAG
  $.category_id  AS category_id  NUMERIC
  $.is_active    AS is_active    TAG
  $.price_min    AS price_min    NUMERIC
```

### `catalog:categories:idx`

```text
ON JSON PREFIX 1 category:
SCHEMA
  $.name       AS name       TEXT
  $.slug       AS slug       TAG
  $.parent_id  AS parent_id  NUMERIC
```

تُنشأ تلقائياً عند الإقلاع عبر `redis::search::ensure_indexes`؛ تتجاهل خطأ "Index already exists".

---

## Redis Streams

- **Stream**: `REDIS_STREAM_KEY`
- **Group**: `REDIS_CONSUMER_GROUP`
- **Consumer**: `REDIS_CONSUMER_NAME`
- **الحقول**: `kind`, `id`, `payload` (JSON)

### قراءة يدوية من Redis CLI

```bash
redis-cli XINFO STREAM catalog:events
redis-cli XINFO GROUPS catalog:events
redis-cli XPENDING catalog:events catalog-workers
redis-cli XRANGE catalog:events - + COUNT 10
```

### إعادة معالجة رسائل فاشلة

```bash
# استخدم XAUTOCLAIM في سكربت أو نسخة مطوّرة من consumer
redis-cli XAUTOCLAIM catalog:events catalog-workers worker-2 0-0 COUNT 10
```

---

## gRPC Inventory

العميل يستدعي:

```protobuf
service InventoryService {
  rpc Reserve(ReserveRequest) returns (ReserveResponse);
  rpc Release(ReleaseRequest) returns (ReleaseResponse);
  rpc AdjustStock(AdjustStockRequest) returns (AdjustStockResponse);  // ensure_sku → delta=0
  ...
}
```

`InventoryClient::ensure_sku(variant_id)` = `AdjustStock { delta: 0 }` — آمن للتكرار (idempotent).

---

## الاختبارات

### هيكل الاختبارات

```
tests/
├── common/                     # helpers مشتركة (DB + Redis + uniq keys)
├── unit_*.rs                   # اختبارات لا تحتاج خدمات خارجية
├── integration_*_repo.rs       # تحتاج Postgres
├── integration_redis_*.rs      # تحتاج Redis Stack (وُسمت بـ #[ignore])
└── integration_http.rs         # end-to-end (تحتاج كل شيء، #[ignore])
```

### تصنيف الاختبارات

| النوع | عدد الملفات | متطلبات |
|---|---|---|
| **Unit** (بدون خدمات) | 7 ملفات | — (تُشغَّل دائماً في CI) |
| **Integration (Postgres)** | 3 ملفات | `TEST_DATABASE_URL` |
| **Integration (Redis)** | 3 ملفات | `TEST_REDIS_URL` (Stack) — `#[ignore]` |
| **E2E HTTP** | ملف واحد | DB + Redis + gRPC — `#[ignore]` |

### تغطية الحالات

#### Unit
- **`unit_cursor`**: ترميز/فك ترميز، URL-safe، مدخلات غير صالحة، `decode_required` خطأ/نجاح.
- **`unit_pagination`**: `limit` الافتراضي، القصّ (1..=100)، استخراج cursor من `after`، cursor غير صالح، `ProductFilterInput::default`.
- **`unit_config`**: متغير ناقص → `AppError::Config`، قيم افتراضية، تجاوزات.
- **`unit_error`**: `Display`، `From<AppError> for tonic::Status`، `ErrorExtensions`، `From<sqlx::Error>`.
- **`unit_stream_events`**: `kind()` / `id()` لكل variant، serde tagged roundtrip.
- **`unit_query_helpers`**: `build_redis_query` (فارغ، category، brand، active، range، search، مركّب).
- **`unit_variant_type`**: `From<VariantRow> for Variant` بحقول ممتلئة/`None`.

#### Integration (Postgres)
- **`integration_category_repo`**: CRUD، slug conflict → `Conflict`، not-found على update/get/delete، children، pagination.
- **`integration_product_repo`**: CRUD، `get_by_slug`، فلاتر (category، brand، active، search ILIKE)، cursor pagination، update متعدد.
- **`integration_variant_repo`**: insert/list/delete، sku conflict، cascade delete عند حذف المنتج.

#### Integration (Redis)
- **`integration_redis_json`**: `set/get/del` roundtrip مع JSON معقّد، get مفقود.
- **`integration_redis_search`**: بحث بـ category/brand/active/price، limit/offset بدون تداخل، index مؤقت يُنشأ ويُحذف.
- **`integration_redis_stream`**: `publish` → `XLEN`، قراءة الحقول، consumer group مع `XREADGROUP`.

#### E2E
- **`integration_http`**: Playground يُعرض، استعلام `products` يرجع `ProductConnection` صالح، mutation بدون توكن.

---

## تشغيل الاختبارات

### أولاً: تحويل المشروع إلى lib + bin

`Cargo.toml`:

```toml
[lib]
name = "category_service"
path = "src/lib.rs"

[[bin]]
name = "category_service"
path = "src/main.rs"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
tower = { version = "0.5", features = ["util"] }
uuid = { version = "1", features = ["v7", "serde"] }
bigdecimal = { version = "0.4", features = ["serde"] }
```

### الوحدات فقط (بدون خدمات)

```bash
cargo test --tests -- --skip integration
# أو:
cargo test --lib           # للاختبارات داخل src (إن وجدت)
cargo test --test unit_cursor --test unit_pagination ...
```

### الكل مع الخدمات

```bash
# 1. شغّل الخدمات
docker run -d --name pg-test -p 5433:5432 \
  -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=catalog_test postgres:16
docker run -d --name redis-test -p 6380:6379 redis/redis-stack:latest

# 2. صدّر المتغيرات
export TEST_DATABASE_URL=postgres://postgres:postgres@localhost:5433/catalog_test
export TEST_REDIS_URL=redis://127.0.0.1:6380

# 3. شغّل الكل (بما فيه المتجاهَل)
cargo test -- --include-ignored --test-threads=1
```

> `--test-threads=1` مطلوب لأن `unit_config` يعدّل متغيرات env، واختبارات `_repo` تتعامل مع نفس الجداول (TRUNCATE).

### تشغيل اختبار واحد

```bash
cargo test --test integration_product_repo filters_and_cursor_pagination -- --nocapture
```

### تشغيل معروض (ignored) فقط

```bash
TEST_DATABASE_URL=... TEST_REDIS_URL=... \
  cargo test -- --ignored --test-threads=1
```

---

## CI المقترح (GitHub Actions)

```yaml
name: ci

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: catalog_test
        ports: ["5432:5432"]
        options: >-
          --health-cmd pg_isready --health-interval 5s
          --health-timeout 5s --health-retries 10
      redis:
        image: redis/redis-stack:latest
        ports: ["6379:6379"]
        options: >-
          --health-cmd "redis-cli ping" --health-interval 5s
          --health-timeout 5s --health-retries 10

    env:
      TEST_DATABASE_URL: postgres://postgres:postgres@localhost:5432/catalog_test
      TEST_REDIS_URL: redis://127.0.0.1:6379
      DATABASE_URL: postgres://postgres:postgres@localhost:5432/catalog_test
      REDIS_URL: redis://127.0.0.1:6379
      REDIS_STREAM_KEY: ci:stream
      REDIS_CONSUMER_GROUP: ci-group
      INVENTORY_GRPC_URL: http://127.0.0.1:50051

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt, clippy }
      - name: Install protoc
        run: sudo apt-get update && sudo apt-get install -y protobuf-compiler
      - uses: Swatinem/rust-cache@v2
      - name: fmt
        run: cargo fmt --check
      - name: clippy
        run: cargo clippy --all-targets -- -D warnings
      - name: unit tests
        run: cargo test --test 'unit_*' -- --test-threads=1
      - name: integration tests
        run: cargo test -- --include-ignored --test-threads=1
```

---

## قائمة TODO / تحسينات مستقبلية

### حرجة
- [ ] **`mutation.rs` يستخدم `data_unchecked::<AccessToken>()`** — يسبب panic عند غياب الـ middleware. يجب تحويله إلى `ctx.data::<AccessToken>()?` وإرجاع GraphQL error نظيف.
- [ ] **`createVariant` ينشر `ProductUpdated`** بدلاً من نوع حدث مخصص `VariantCreated` — يخلط المسؤوليات ويصعّب التتبع.
- [ ] **`ProductUpdated` يعالج الـ variant inline** — يحتاج نوع حدث مستقل وقاعدة تحقق.
- [ ] **`min_price`/`max_price` في `ProductFilterInput`** غير مستخدمة فعلياً في `ProductRepo::list` (فقط في RediSearch، ومفتاح `price_min` غير موجود في مستندات المنتج). يجب إضافته عبر join مع `product_variants` أو حقل مشتق.
- [ ] **`total_count`** في Query غير محسوب بعد التصفية (COUNT بدون WHERE).

### بنية
- [ ] **Outbox pattern**: جدول `outbox_events` موجود لكن `outbox::publisher` معطّل. تفعيله لضمان at-least-once publish عند فشل XADD.
- [ ] **`AppError` لا يميّز** بين 4xx و 5xx في tonic/GraphQL — إضافة mapping (NotFound → `Code::NotFound`, Conflict → `AlreadyExists`, etc.).
- [ ] **JWT Middleware**: `middleware/auth.rs` غير مربوط بـ `router`. إضافة `axum` middleware layer.
- [ ] **Pagination**: `has_next_page` في `products` مُثبّت على `false` — يجب حسابه بناءً على `docs.len() == limit`.
- [ ] **Cursor**: يستخدم `id` فقط؛ لفرز حسب `created_at` أو `price` يجب توسيعه.

### اختبار
- [ ] **Trait abstraction للـ repos** لتمكين mock كامل في اختبارات GraphQL بدون DB.
- [ ] **`temp-env` / `serial_test`** بدلاً من `Mutex + unsafe env` في `unit_config`.
- [ ] **gRPC mock service** لاختبار `InventoryClient` بـ `tonic::transport::Server`.
- [ ] **Benchmark** لـ `build_redis_query` و`parse_ft_search`.

### تشغيل
- [ ] **Health/Readiness endpoints** (`/health`, `/ready`).
- [ ] **OpenTelemetry** tracing بدل `tracing-subscriber` فقط.
- [ ] **Graceful shutdown** (SIGTERM → إيقاف consumer ثم axum).
- [ ] **Retry/backoff** في `stream_worker` عند فشل DB.
- [ ] **Dead-letter queue** للأحداث الفاشلة بشكل متكرر.

---

## الترخيص

غير محدد — أضف ملف `LICENSE` قبل النشر.
```

---

## ملاحظات على الملف

- يغطي الملف **الخدمة** و**الاختبارات** معاً كما طلبت.
- يُوثّق الـ **gaps والـ TODO** بصراحة حتى يعرف القارئ أين الحدود الحالية (خصوصاً `data_unchecked`، وقيد `min_price`/`max_price` الذي لم يُنفّذ فعلياً في `ProductRepo`).
- قسم **الاختبارات** مصمَّم ليطابق تماماً بنية `tests/` التي أرسلتها في الرد السابق (7 unit + 3 integration pg + 3 integration redis + 1 e2e).
- قسم **CI** يوضّح الأوامر الفعلية للفصل بين unit وintegration عبر `--test 'unit_*'` و `--include-ignored`.