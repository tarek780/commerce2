# 📘 README — `storefront-service`

```markdown
# storefront-service

خدمة الواجهة الأمامية (BFF) لمنصة المتاجر الإلكترونية. تُنسّق رحلة الشراء الكاملة (checkout) بين خدمات السلة، الكتالوج، الطلبات، والدفع، وتوفّر REST API موحّداً مع مصادقة JWT، وidempotency، واستجابات وفق RFC 7807.

---

## 📑 المحتويات

- [نظرة عامة](#-نظرة-عامة)
- [المعمارية](#-المعمارية)
- [المتطلبات](#-المتطلبات)
- [التشغيل السريع](#-التشغيل-السريع)
- [الإعدادات (Environment)](#-الإعدادات-environment)
- [واجهات API](#-واجهات-api)
- [المصادقة (JWT)](#-المصادقة-jwt)
- [Idempotency](#-idempotency)
- [معالجة الأخطاء (RFC 7807)](#-معالجة-الأخطاء-rfc-7807)
- [بنية المشروع](#-بنية-المشروع)
- [الاختبارات](#-الاختبارات)
- [التطوير المحلي](#-التطوير-المحلي)
- [النشر](#-النشر)
- [الترخيص](#-الترخيص)

---

## 🎯 نظرة عامة

`storefront-service` هو BFF (Backend-for-Frontend) يقف بين عميل الويب وخدمات microservices الداخلية:

| المسؤولية | التفاصيل |
|-----------|----------|
| **رحلة الشراء** | checkout ذرّي (سبق: إنشاء طلب → دفع) مع تفريغ السلة |
| **المصادقة** | JWT (HS256) عبر `AuthLayer` — access tokens فقط |
| **Idempotency** | حماية checkout من التكرار عبر `Idempotency-Key` + Redis |
| **التجميع** | دمج بيانات السلة + أسعار الكتالوج + حالة الطلب في DTO موحّد |
| **الأمان** | التحقق من ملكية المستخدم للموارد (orders) |
| **المراقبة** | health/ready + structured JSON logs |

**المزايا الأساسية:**

- ✅ تحقق من ملكية الطلب — مستخدم لا يرى طلبات غيره (404، لا 403)
- ✅ تدهور رشيق — فشل تفريغ السلة لا يُفشل checkout (warnings فقط)
- ✅ idempotent checkout — تكرار الطلب بنفس المفتاح = نفس الاستجابة
- ✅ Problem Details (RFC 7807) موحّد
- ✅ اختبارات تكامل بلا خدمات خارجية

---

## 🏗 المعمارية

```
                    ┌─────────────────────────────────────────┐
                    │           storefront-service             │
  Client (SPA)      │                                         │
   ────────────►    │  AuthLayer (JWT)                        │
   Bearer JWT       │      │                                  │
                    │      ▼                                  │
                    │  Router ─► Handlers ─► Clients          │
                    │               │            │            │
                    │               │            ├─► cart      │ HTTP
                    │               │            ├─► category  │ GraphQL
                    │               │            ├─► order     │ gRPC
                    │               │            └─► payment   │ gRPC
                    │               │                         │
                    │               └─► IdempotencyStore      │ Redis
                    │                                         │
                    └─────────────────────────────────────────┘
```

**الخدمات الخارجية المتوقعة:**

| Service | Protocol | Purpose |
|---------|----------|---------|
| `cart-service` | HTTP REST | قراءة/تفريغ السلة |
| `category-service` | GraphQL | أسعار المتغيّرات (variants) |
| `order-service` | gRPC | إنشاء/جلب/إلغاء/دفع الطلبات |
| `payment-service` | gRPC | استعلام الدفع |
| `redis` | RESP | تخزين مفاتيح idempotency |

---

## 📦 المتطلبات

- **Rust** 1.82+ (edition 2021)
- **Redis** 7+ (للتشغيل الفعلي فقط — الاختبارات لا تحتاجه)
- الوصول إلى خدمات `cart / category / order / payment` (أو mocks في الاختبارات)

---

## 🚀 التشغيل السريع

```bash
# 1. استنساخ
git clone <repo> && cd storefront-service

# 2. إعداد متغيرات البيئة
cp .env.example .env
# عدّل .env حسب خدماتك

# 3. تشغيل
cargo run --release

# 4. التحقق
curl http://localhost:8080/health
# {"status":"ok"}
```

---

## ⚙️ الإعدادات (Environment)

جميع الإعدادات تُقرأ من متغيرات البيئة عبر `AppConfig::from_env()`:

| Variable | Default | الوصف |
|----------|---------|-------|
| `HTTP_ADDR` | `0.0.0.0:8080` | عنوان الاستماع |
| `SECRET_KEY` | **مطلوب** | مفتاح HS256 لتوقيع JWT |
| `CURRENCY` | `USD` | العملة الافتراضية |
| `CART_BASE_URL` | **مطلوب** | URL خدمة السلة |
| `CART_CLEAR_ENDPOINT` | `/api/cart` | مسار تفريغ السلة |
| `CATEGORY_GRAPHQL_URL` | **مطلوب** | endpoint GraphQL للكتالوج |
| `ORDER_GRPC_URL` | **مطلوب** | gRPC endpoint لخدمة الطلبات |
| `PAYMENT_GRPC_URL` | **مطلوب** | gRPC endpoint لخدمة الدفع |
| `INVENTORY_GRPC_URL` | **مطلوب** | gRPC endpoint للمخزون |
| `REDIS_URL` | `redis://127.0.0.1:6379` | اتصال Redis |
| `IDEMPOTENCY_TTL_SECS` | `86400` | TTL مفاتيح idempotency |
| `IDEMPOTENCY_REQUIRED` | `true` | هل `Idempotency-Key` إلزامي على checkout |
| `UPSTREAM_TIMEOUT_SECS` | `10` | مهلة استدعاء الخدمات الخارجية |

### `.env.example`

```dotenv
HTTP_ADDR=0.0.0.0:8080
SECRET_KEY=change-me-in-production-min-32-chars
CURRENCY=USD

CART_BASE_URL=http://cart-service:8080
CART_CLEAR_ENDPOINT=/api/cart

CATEGORY_GRAPHQL_URL=http://category-service:8080/graphql
ORDER_GRPC_URL=http://order-service:50051
PAYMENT_GRPC_URL=http://payment-service:50052
INVENTORY_GRPC_URL=http://inventory-service:50053

REDIS_URL=redis://redis:6379
IDEMPOTENCY_TTL_SECS=86400
IDEMPOTENCY_REQUIRED=true
UPSTREAM_TIMEOUT_SECS=10

RUST_LOG=storefront_service=debug,tower_http=info
```

---

## 🌐 واجهات API

### Health (public)

| Method | Path | Response |
|--------|------|----------|
| `GET` | `/health` | `{"status":"ok"}` |
| `GET` | `/ready` | `{"status":"ready"}` |

### Checkout (protected)

#### `POST /api/v1/checkout`

ينفّذ رحلة الشراء كاملة: قراءة السلة → حجز الأسعار → إنشاء طلب → بدء دفع → تفريغ السلة.

**Headers (إلزامية):**

| Header | Value |
|--------|-------|
| `Authorization` | `Bearer <access_jwt>` |
| `Idempotency-Key` | UUID (إلزامي إذا `IDEMPOTENCY_REQUIRED=true`) |

**Request body:** `{}` (فارغ — السلة تُقرأ من خدمة السلة)

**Response `201 Created`:**

```json
{
  "order_id": "order-1",
  "payment_id": "pay-1",
  "payment_url": "https://pay.example.com/session/abc",
  "status": "processing",
  "total_amount": "60.00",
  "currency": "USD",
  "warnings": []
}
```

**Response مع فشل تفريغ السلة (لا يزال 201):**

```json
{
  ...
  "warnings": ["cart_not_cleared"]
}
```

### Orders (protected)

| Method | Path | الوصف |
|--------|------|-------|
| `GET` | `/api/v1/orders?first=10&after=<cursor>&status=<filter>` | قائمة الطلبات (cursor pagination) |
| `GET` | `/api/v1/orders/{id}` | طلب واحد (ownership) |
| `POST` | `/api/v1/orders/{id}/cancel` | إلغاء (body: `{"reason":"..."}`) |
| `POST` | `/api/v1/orders/{id}/pay` | بدء الدفع |

#### `GET /api/v1/orders` — Response

```json
{
  "edges": [
    { "node": { "id": "order-1", "status": "awaiting_payment", ... } }
  ],
  "page_info": { "has_next_page": true, "end_cursor": "cursor2" }
}
```

### Payments (protected)

| Method | Path | الوصف |
|--------|------|-------|
| `GET` | `/api/v1/payments/{id}` | حالة الدفع |

---

## 🔐 المصادقة (JWT)

كل endpoint محمي يحتاج **access token** صالحاً في ترويسة `Authorization`.

### مواصفات التوكن

- **Algorithm:** HS256
- **Secret:** من `SECRET_KEY`
- **Claims:**
  ```json
  {
    "typ": "Access",
    "user_id": "<uuid>",
    "token_id": "<uuid>",
    "role": "User | Admin",
    "exp": 1735689600
  }
  ```

### مثال

```bash
curl -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
     -H "Idempotency-Key: $(uuidgen)" \
     -X POST http://localhost:8080/api/v1/checkout
```

### سلوك `AuthLayer`

| الحالة | النتيجة |
|--------|---------|
| لا `Authorization` | `401 Unauthorized` |
| scheme ليس `Bearer ` | `401` |
| توكن غير صالح / توقيع خاطئ / منتهي | `401` |
| توكن صالح | استمرار — يُحقن `AccessToken { user_id }` في extensions |

### ⚠️ توصية أمنية

الـ `AuthLayer` الحالي لا يتحقق من `typ` claim. **يُنصح بشدة** بإضافة:

```rust
if token_data.typ != Typ::Access {
    return Ok(StatusCode::UNAUTHORIZED.into_response());
}
```

لمنع استخدام `Refresh` tokens على المسارات المحمية.

---

## 🔁 Idempotency

`POST /api/v1/checkout` آمن ضد التكرار بفضل `Idempotency-Key`.

### آلية العمل

```
1. Client يرسل Idempotency-Key: <uuid>
2. الخدمة تحاول SET NX في Redis
   ├─ نجح → المفتاح جديد → نفّذ العملية
   └─ فشل → المفتاح موجود
       ├─ القيمة "__PENDING__" → 409 Conflict (طلب متزامن)
       └─ القيمة استجابة كاملة → أعِد نفس الاستجابة
3. بعد نجاح العملية → SET المفتاح = <body> EX IDEMPOTENCY_TTL_SECS
```

### سيناريوهات

| السيناريو | السلوك |
|-----------|--------|
| أول طلب بمفتاح جديد | 201 + تخزين النتيجة |
| إعادة الطلب بعد ثوانٍ | 201 بنفس الجسم (cached) |
| طلب متزامن أثناء المعالجة | 409 Conflict |
| انقطاع أثناء المعالجة (pending لم يُكمل) | 409 حتى انتهاء TTL |

### متى يستخدم العميل مفتاحاً جديداً؟

- لكل محاولة شراء **مستقلة**.
- عند إعادة المحاولة بعد فشل شبكي — **نفس المفتاح**.
- بعد تعديل السلة — **مفتاح جديد**.

---

## 🧯 معالجة الأخطاء (RFC 7807)

كل استجابة خطأ تتبع `application/problem+json`:

```json
{
  "type": "https://storefront.example.com/errors/insufficient-stock",
  "title": "Insufficient Stock",
  "status": 402,
  "detail": "variant 1 requested 5, available 3",
  "instance": "/api/v1/checkout"
}
```

### جدول الاستجابات

| Status | `type` (slug) | السيناريو |
|--------|--------------|-----------|
| `400` | `validation` | جسم طلب غير صالح، idempotency-key مفقود |
| `401` | — | توكن مفقود/غير صالح (body فارغ من AuthLayer) |
| `402` | `insufficient-stock` | مخزون غير كافٍ |
| `404` | `not-found` | طلب/دفع غير موجود أو ليس للمستخدم |
| `409` | `idempotency-conflict` | طلب متزامن بنفس المفتاح |
| `500` | `internal` | خطأ داخلي |
| `502` | `upstream` | فشل خدمة خارجية (category/cart/order) |

---

## 📂 بنية المشروع

```
storefront-service/
├── Cargo.toml
├── README.md
├── .env.example
├── src/
│   ├── lib.rs                 # مكتبة (للاستيراد في الاختبارات)
│   ├── main.rs                # نقطة الدخول
│   ├── config.rs              # AppConfig::from_env
│   ├── state.rs               # AppState (DI container)
│   ├── routes.rs              # Router + layers
│   ├── error.rs               # AppError → RFC 7807
│   ├── problem.rs             # ProblemDetails struct
│   ├── openapi.rs             # utoipa OpenAPI spec
│   │
│   ├── middleware/
│   │   └── auth.rs            # AuthLayer + AccessToken + DecodedToken
│   │
│   ├── helpers/
│   │   ├── idempotency.rs     # IdempotencyStore trait (Redis + Memory)
│   │   ├── cursor.rs          # cursor encoding/decoding
│   │   └── jwt.rs             # (اختياري) helpers للتوقيع في الاختبارات
│   │
│   ├── handlers/
│   │   ├── checkout.rs        # POST /api/v1/checkout
│   │   ├── orders.rs          # /api/v1/orders/*
│   │   ├── payments.rs        # /api/v1/payments/*
│   │   └── health.rs          # /health, /ready
│   │
│   ├── clients/
│   │   ├── cart.rs            # trait CartClient + HttpCartClient
│   │   ├── category.rs        # trait CategoryClient
│   │   ├── order.rs           # trait OrderClient + proto
│   │   └── payment.rs         # trait PaymentClient + proto
│   │
│   └── dto/
│       ├── cart.rs
│       ├── checkout.rs
│       ├── orders.rs
│       └── payments.rs
│
└── tests/
    ├── common/
    │   ├── mod.rs             # harness + HTTP helpers
    │   ├── auth.rs            # JWT issuance helpers
    │   ├── fixtures.rs        # DTO builders
    │   └── mocks/
    │       ├── mod.rs
    │       ├── cart.rs
    │       ├── category.rs
    │       ├── order.rs
    │       └── payment.rs
    │
    ├── auth_test.rs           # JWT validation matrix
    ├── checkout_test.rs       # checkout flow
    ├── orders_test.rs         # orders CRUD + ownership
    ├── payments_test.rs       # payments
    ├── idempotency_test.rs    # duplicate-key behavior
    ├── health_test.rs         # health/ready
    └── problem_test.rs        # RFC 7807
```

---

## 🧪 الاختبارات

### المبدأ

الاختبارات **مستقلة تماماً عن أي خدمة خارجية**:

- ✅ لا Postgres، لا Redis، لا Kafka، لا gRPC حقيقي
- ✅ كل `*Client` له mock في `tests/common/mocks/`
- ✅ Idempotency عبر `MemoryIdempotencyStore` (بدل Redis)
- ✅ Router يُختبر عبر `tower::ServiceExt::oneshot` (بلا network)
- ✅ JWT يُوقّع محلياً بنفس `SECRET_KEY` (بلا auth server)

### التشغيل

```bash
# كل الاختبارات
cargo test -p storefront-service

# ملف محدد
cargo test -p storefront-service --test checkout_test

# اختبار واحد
cargo test -p storefront-service --test auth_test expired_jwt_returns_401

# مع output
cargo test -p storefront-service -- --nocapture

# بالتوازي (افتراضي)، أو بالتسلسل
cargo test -p storefront-service -- --test-threads=1

# مع تغطية
cargo install cargo-tarpaulin
cargo tarpaulin -p storefront-service --out Html --output-dir coverage
```

### خريطة الاختبارات

#### 🔐 `auth_test.rs`

| Test | يتحقق من |
|------|----------|
| `missing_authorization_returns_401` | لا ترويسة Authorization |
| `non_bearer_scheme_returns_401` | `Basic`/`Digest` مرفوض |
| `malformed_jwt_returns_401` | توكن غير صالح التركيب |
| `jwt_signed_with_wrong_secret_returns_401` | توقيع بمفتاح خاطئ |
| `expired_jwt_returns_401` | انتهاء صلاحية |
| `valid_access_token_passes` | توكن صالح يعمل |
| `refresh_token_behavior` | (وثّق: الحالي يقبل — يُنصح بالرفض) |
| `admin_token_passes` | دور Admin يمر |
| `health_is_public`, `ready_is_public` | المسارات العامة بلا auth |

#### 🛒 `checkout_test.rs`

| Test | يتحقق من |
|------|----------|
| `checkout_success` | رحلة كاملة 201 |
| `checkout_empty_cart_returns_400` | سلة فارغة → 400 + problem+json |
| `checkout_insufficient_stock_maps_to_402` | خطأ مخزون → 402 + type صحيح + عدم تفريغ السلة |
| `checkout_invalid_quantity_rejected` | كمية 0 → 400، لا استدعاء create_order |
| `checkout_category_failure_maps_to_502` | فشل كتالوج → 502 |
| `checkout_cart_clear_failure_returns_warnings` | فشل تفريغ السلة لا يُفشل checkout |

#### 📦 `orders_test.rs`

| Test | يتحقق من |
|------|----------|
| `list_orders_empty` | قائمة فارغة |
| `list_orders_with_cursor_pagination` | cursor + page_info صحيحان |
| `get_order_success` | جلب طلب مملوك |
| `get_order_not_owner_returns_404` | **security**: 404 لا 403 (عدم تسريب وجود) |
| `get_order_not_found` | طلب غير موجود |
| `cancel_order_success` | إلغاء ناجح |
| `cancel_order_not_owner_returns_404` | ملكية عند الإلغاء |
| `pay_order_success` | بدء دفع |

#### 💳 `payments_test.rs`

| Test | يتحقق من |
|------|----------|
| `get_payment_success` | جلب دفع |
| `get_payment_not_found` | 404 |
| `get_payment_unauthenticated_returns_401` | auth إلزامي |

#### 🔁 `idempotency_test.rs`

| Test | يتحقق من |
|------|----------|
| `duplicate_idempotency_key_returns_cached_response` | نفس المفتاح = نفس الاستجابة + `create_order` مرة واحدة |

#### 🧯 `problem_test.rs`

| Test | يتحقق من |
|------|----------|
| `problem_details_follows_rfc7807` | content-type + status + type/title/detail |
| `unauthorized_response_is_plain_401` | AuthLayer يُرجع 401 بلا body |

#### ❤️ `health_test.rs`

| Test | يتحقق من |
|------|----------|
| `health_returns_ok` | `/health` = 200 |
| `ready_returns_ok` | `/ready` = 200 |

### بنية الاختبارات

#### `tests/common/mod.rs`

- `TestHarness` — يجمع Router + mocks للتحكم بالسلوك
- `harness()` — يبني حالة اختبار كاملة
- `get_json / post_json` — helpers تبني `Request` وتستدعي `oneshot`

#### `tests/common/auth.rs`

- `init_secret_once()` — يضبط `SECRET_KEY` عبر `Once`
- `access_token_for(user_id)` — توكن صالح
- `refresh_token_for(user_id)` — توكن Refresh
- `expired_token_for(user_id)` — منتهي
- `admin_token_for(user_id)` — دور Admin
- `bearer(token)` — يبني `Bearer <token>`

#### `tests/common/mocks/`

| Mock | يُحاكي | تحكم |
|------|--------|------|
| `MockCartClient` | HTTP cart | items, fail_clear, fail_get, clear_calls |
| `MockCategoryClient` | GraphQL category | prices map, fail_for(variant_id) |
| `MockOrderClient` | gRPC order | create/list/pay/cancel responses, call counters |
| `MockPaymentClient` | gRPC payment | payments map |

### إضافة اختبار جديد

**1. اختبار checkout جديد:**

```rust
// tests/checkout_test.rs
#[tokio::test]
async fn checkout_with_multiple_items() {
    let h = common::harness().await;
    *h.cart.items.lock().await = vec![cart_item(1, 1), cart_item(2, 2), cart_item(3, 3)];
    h.category.with_prices(vec![(1, "10.00"), (2, "20.00"), (3, "30.00")]).await;

    let headers = idem_headers_for(USER_ID);
    let (status, json, _) = post_json(
        &h.app, "/api/v1/checkout",
        &as_refs(&headers), serde_json::json!({}),
    ).await;

    assert_eq!(status, 201);
}
```

**2. اختبار ملكية (ownership) جديد:**

```rust
#[tokio::test]
async fn other_user_cannot_pay_order() {
    let h = common::harness().await;
    h.order.orders.lock().await.push(order_response("order-1", OTHER_USER_ID));
    let hdr = bearer_for(USER_ID);
    let (status, _, _) = post_json(
        &h.app, "/api/v1/orders/order-1/pay",
        &refs(&hdr), serde_json::json!({}),
    ).await;
    assert_eq!(status, 404);
}
```

**3. اختبار فشل خدمة:**

```rust
#[tokio::test]
async fn checkout_order_service_down() {
    let h = common::harness().await;
    *h.cart.items.lock().await = vec![cart_item(1, 1)];
    h.category.with_prices(vec![(1, "20.00")]).await;
    *h.order.create_response.lock().await = Some(Err(
        AppError::Upstream("order service unreachable".into())
    ));

    let headers = idem_headers_for(USER_ID);
    let (status, _, _) = post_json(
        &h.app, "/api/v1/checkout",
        &as_refs(&headers), serde_json::json!({}),
    ).await;
    assert_eq!(status, 502);
}
```

### CI Matrix (GitHub Actions)

```yaml
# .github/workflows/ci.yml
name: ci
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt, clippy }
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all-features
```

---

## 🛠 التطوير المحلي

### مع الخدمات الحقيقية

```bash
# شغّل الخدمات (docker-compose متوفر في repo المنصة الأم)
docker compose -f docker-compose.dev.yml up -d

# شغّل storefront-service
cargo run

# جرّب
TOKEN=$(./scripts/mint-jwt.sh 550e8400-e29b-41d4-a716-446655440000)
curl -H "Authorization: Bearer $TOKEN" \
     -H "Idempotency-Key: $(uuidgen)" \
     -X POST http://localhost:8080/api/v1/checkout
```

### سكربت توليد JWT للتطوير

```bash
# scripts/mint-jwt.sh
#!/usr/bin/env bash
set -euo pipefail
USER_ID="${1:-550e8400-e29b-41d4-a716-446655440000}"
SECRET="${SECRET_KEY:-change-me-in-production-min-32-chars}"
NOW=$(date +%s)
EXP=$((NOW + 3600))

HEADER=$(printf '{"alg":"HS256","typ":"JWT"}' | openssl base64 -A | tr '+/' '-_' | tr -d '=')
PAYLOAD=$(printf '{"typ":"Access","user_id":"%s","token_id":"%s","role":"User","exp":%d}' \
  "$USER_ID" "$(uuidgen)" "$EXP" | openssl base64 -A | tr '+/' '-_' | tr -d '=')
SIGNATURE=$(printf '%s.%s' "$HEADER" "$PAYLOAD" \
  | openssl dgst -sha256 -hmac "$SECRET" -binary \
  | openssl base64 -A | tr '+/' '-_' | tr -d '=')

echo "$HEADER.$PAYLOAD.$SIGNATURE"
```

### Hot reload

```bash
cargo install cargo-watch
cargo watch -x run
```

---

## 🚢 النشر

### Build

```bash
cargo build --release
# binary: target/release/storefront-service
```

### Dockerfile (multi-stage)

```dockerfile
FROM rust:1.82-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/storefront-service /usr/local/bin/
EXPOSE 8080
CMD ["storefront-service"]
```

### Health checks (Kubernetes)

```yaml
livenessProbe:
  httpGet: { path: /health, port: 8080 }
  initialDelaySeconds: 5
  periodSeconds: 10

readinessProbe:
  httpGet: { path: /ready, port: 8080 }
  initialDelaySeconds: 5
  periodSeconds: 5
```

### ملاحظات الإنتاج

- ⚠️ **`SECRET_KEY`** لا يُخزّن في الصورة — استخدم secrets (Vault/K8s secrets)
- ⚠️ **`IDEMPOTENCY_REQUIRED=true`** في الإنتاج
- ⚠️ فعّل **rate limiting** (tower_governor) قبل الواجهة العامة
- ⚠️ **CORS** — أضف `tower_http::cors::CorsLayer` للسماح للنطاقات المسموحة فقط
- ⚠️ **`RUST_LOG=storefront_service=info,tower_http=info`** (ليس `debug`)
- ✅ فعّل OpenTelemetry traces (tracing-opentelemetry)

---

## 🗺 Roadmap

- [ ] **رفض Refresh tokens** على المسارات المحمية في `AuthLayer`
- [ ] **Rate limiting** عبر `tower_governor`
- [ ] **CORS** سياسة صريحة
- [ ] **Circuit breaker** لخدمات upstream
- [ ] **OpenAPI UI** على `/docs`
- [ ] **Property-based tests** (`proptest`) لتشفير cursors
- [ ] **Snapshot tests** (`insta`) لاستجابات JSON
- [ ] **gRPC metadata propagation** (trace-id, user-id)
- [ ] **Retry with backoff** لخدمات gRPC
- [ ] **Metrics** (Prometheus exporter)

---

## 📝 الترخيص

MIT OR Apache-2.0 (اختر واحداً وحدّث `Cargo.toml`).

---

## 🤝 المساهمة

1. Fork → branch (`feature/xyz`)
2. أضف اختباراتك في `tests/`
3. `cargo fmt && cargo clippy -- -D warnings && cargo test`
4. Pull Request

---

## 📞 الدعم

- Issues: `<repo>/issues`
- Slack: `#platform-storefront`
- On-call: `<pager>`
```

---

## 📄 ملف مساعد إضافي — `tests/README.md`

```markdown
# اختبارات `storefront-service`

حزمة اختبارات تكامل **بلا اعتماديات خارجية** (لا Docker / لا Redis / لا gRPC / لا auth server).

## Quick start

```bash
cargo test --all
```

## مبادئ التصميم

1. **Isolation** — كل اختبار يبني `harness()` مستقلة.
2. **No network** — `tower::ServiceExt::oneshot` على Router مباشرة.
3. **No mocks framework** — `Arc<Mutex<...>>` بسيط + counters + injected failures.
4. **JWT real** — تواقيع HS256 حقيقية باستخدام نفس `SECRET_KEY` المستخدم في `AuthService`.
5. **DI عبر trait** — `IdempotencyStore`, `CartClient`, ... قابلة للاستبدال.

## تشغيل مجموعة فرعية

```bash
cargo test --test auth_test           # JWT matrix
cargo test --test checkout_test       # checkout flows
cargo test --test orders_test         # orders + ownership
cargo test -- --nocapture             # مع stdout
```

## إضافة test

1. أضف mock أو control knob في `common/mocks/*.rs`.
2. ابنِ الـ data عبر `common/fixtures.rs`.
3. استخدم `harness()` → `TestHarness`.
4. ولّد JWT عبر `common::auth::access_token_for(USER_ID)`.
5. استدعِ `get_json` / `post_json` helpers.
6. تحقّق من status + JSON + الـ counters في الـ mock.

## لماذا لا نستخدم testcontainers؟

- أسرع بكثير (بلا spawn/teardown containers)
- أكثر حتمية (لا network flakiness)
- يعمل في CI بلا Docker daemon
- الـ mocks تُشغّل السيناريوهات النادرة بسهولة (فشل فوري، timeout، 409, ...)

عندما نحتاج اختبارات E2E عبر الخدمات الحقيقية، سنضيفها في repo منفصل (system-tests) مع `docker-compose`.
```

---

**جاهز للاستخدام الفوري.** هل تريد:
- 🐳 **`docker-compose.dev.yml`** مع كل الخدمات لاختبار يدوي شامل؟
- 📜 **سكربت `scripts/mint-jwt.sh`** كملف فعلي في المشروع؟
- 🚦 **طبقة rate limiting** + اختباراتها؟
- 📊 **Prometheus metrics** + Grafana dashboard؟