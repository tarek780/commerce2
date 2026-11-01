# 🛒 Cart Service

خدمة سلة تسوق مكتوبة بـ **Rust** تعتمد على **Axum** و**PostgreSQL** و**Redis**.
تدعم سلال الزوّار (Guest) غير المسجّلين، وسلال المستخدمين المسجّلين، ودمج سلة الزائر في سلة المستخدم عند تسجيل الدخول.

---

## 📑 المحتويات

1. [الميزات](#-الميزات)
2. [البنية المعمارية](#-البنية-المعمارية)
3. [هيكل المشروع](#-هيكل-المشروع)
4. [المتطلبات](#-المتطلبات)
5. [الإعداد والتشغيل](#-الإعداد-والتشغيل)
6. [متغيرات البيئة](#-متغيرات-البيئة)
7. [واجهات HTTP API](#-واجهات-http-api)
8. [دليل قاعدة البيانات](#-دليل-قاعدة-البيانات)
9. [اختبارات الخدمة](#-اختبارات-الخدمة)
10. [استكشاف الأخطاء](#-استكشاف-الأخطاء)

---

## ✨ الميزات

| الميزة | الزائر (Guest) | المستخدم (User) |
|---|---|---|
| إنشاء سلة | ✅ | ✅ (تلقائي عند أول قراءة) |
| تخزين | Redis فقط | PostgreSQL + Redis cache |
| مدة البقاء | 7 أيام (TTL) | دائم |
| الإضافة فردية/دفعات | فردية | فردية + دفعات |
| المصادقة | لا | JWT عبر `AuthLayer` |
| دمج سلة الزائر | — | ✅ `POST /api/cart/merge` |

- **ذاكرة تخزين مؤقت للمستخدم**: Redis بمدة ساعة، تُبطَل تلقائياً عند التعديل.
- **إضافة دفعات عبر JSONB** في Postgres لتحسين الأداء.
- **تسجيل هيكلي** عبر `tracing` مع `TraceLayer` لكل طلب.
- **إدارة أخطاء موحّدة**: كل خطأ يعود كـ JSON `{ "error": "..." }` مع رمز HTTP مناسب.

---

## 🏗 البنية المعمارية

```
┌──────────────┐  HTTP/JSON  ┌──────────────┐
│   Client     │ ──────────► │  Axum Router │
└──────────────┘             └──────┬───────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
             ┌────────────┐  ┌────────────┐  ┌────────────┐
             │ AuthLayer  │  │  Handlers  │  │  AppState  │
             │   (JWT)    │  │            │  │            │
             └────────────┘  └─────┬──────┘  └─────┬──────┘
                                   │               │
                                   ▼               │
                          ┌────────────────┐       │
                          │  CartService   │◄──────┘
                          └───────┬────────┘
                                  │
                     ┌────────────┴────────────┐
                     ▼                         ▼
              ┌──────────────┐         ┌──────────────┐
              │ GuestRepo    │         │  UserRepo    │
              │ (Redis only) │         │ (PG + Redis) │
              └──────┬───────┘         └──────┬───────┘
                     │                        │
                     ▼                        ▼
              ┌──────────┐         ┌─────────────────┐
              │  Redis   │         │ PostgreSQL +Redis│
              └──────────┘         └─────────────────┘
```

**الأنماط المستخدمة:**
- **Layered Architecture**: Router → Handlers → Service → Repo.
- **Repository Pattern**: كل مصدر بيانات له repo مستقل.
- **Cache-Aside**: للمستخدم، نقرأ من Redis أولاً ثم نُحدّث DB.
- **Bulk Upsert** عبر `jsonb_array_elements` بدل N استعلامات.
- **State Injection** عبر `Arc<AppState>`.

---

## 📂 هيكل المشروع

```
cart-service/
├── Cargo.toml
├── .env.example
├── migrations/
│   └── 0001_carts.sql
├── src/
│   ├── main.rs              # نقطة الانطلاق + بناء الراوتر
│   ├── lib.rs               # تصدير الوحدات للاختبارات
│   ├── state.rs             # AppState
│   ├── error.rs             # CartError + IntoResponse
│   ├── auth/
│   │   └── layer.rs         # AuthLayer + AccessToken (JWT)
│   └── cart/
│       ├── model.rs         # CartItem, CachedCart, AddItemRequest...
│       ├── repository.rs    # GuestCartRepo, UserCartRepo
│       ├── service.rs       # CartService
│       └── handlers.rs      # Axum handlers
└── tests/
    ├── common/mod.rs        # مساعدات: بناء state, router
    ├── guest_cart.rs        # اختبارات سلة الزائر
    ├── user_cart.rs         # اختبارات سلة المستخدم
    ├── merge_cart.rs        # اختبارات دمج السلال
    └── http_guest.rs        # اختبارات HTTP end-to-end
```

---

## 🧰 المتطلبات

- Rust **1.75+** (Edition 2021)
- PostgreSQL **14+**
- Redis **6+**
- Docker (اختياري للتشغيل السريع)
- `sqlx-cli` (اختياري للـ migrations اليدوية)

---

## 🚀 الإعداد والتشغيل

### 1. استنساخ وتهيئة البيئة

```bash
git clone <repo-url> cart-service
cd cart-service
cp .env.example .env
```

عدّل `.env`:

```env
DATABASE_URL=postgres://postgres:pass@localhost:5432/cart
REDIS_URL=redis://localhost:6379
BIND_ADDRESS=0.0.0.0:3000
RUST_LOG=cart_service=debug,tower_http=info
JWT_SECRET=super-secret-change-me
```

### 2. تشغيل الخدمات عبر Docker

```bash
docker run -d --name cart-pg \
  -e POSTGRES_PASSWORD=pass \
  -e POSTGRES_DB=cart \
  -p 5432:5432 postgres:16

docker run -d --name cart-redis -p 6379:6379 redis:7
```

### 3. بناء وتشغيل

```bash
cargo build --release
cargo run --release
```

عند الإقلاع:
- تُطبَّق الـ migrations تلقائياً (`sqlx::migrate!`).
- يُفتح اتصال Redis متعدد.
- يُشغَّل الخادم على `BIND_ADDRESS`.

سترى في السجل:

```
INFO cart service listening on 0.0.0.0:3000
```

---

## 🔧 متغيرات البيئة

| المتغير | إلزامي | الافتراضي | الوصف |
|---|---|---|---|
| `DATABASE_URL` | ✅ | — | رابط Postgres |
| `REDIS_URL` | ✅ | — | رابط Redis |
| `BIND_ADDRESS` | ❌ | `0.0.0.0:3000` | عنوان الاستماع |
| `RUST_LOG` | ❌ | `cart_service=debug,tower_http=info` | مستوى التسجيل |
| `JWT_SECRET` | ✅ | — | مفتاح توقيع JWT |

---

## 🌐 واجهات HTTP API

كل المسارات تحت البادئة `/api`.

### سلة الزائر (عامة)

#### `POST /api/guest/cart` — إنشاء سلة زائر

**الاستجابة `200`:**
```json
{ "guest_cart_id": "018f2a1b-3c4d-7e5f-..." }
```

---

#### `GET /api/guest/cart/:guest_id` — قراءة سلة الزائر

**الاستجابة `200`:**
```json
{
  "items": [
    {
      "sku_id": "SKU-123",
      "quantity": 2,
      "price_at_add": 9.99,
      "added_at": "2026-10-29T10:15:30Z"
    }
  ],
  "updated_at": "2026-10-29T10:15:30Z"
}
```

**أخطاء:** `404` إذا لم تكن السلة موجودة أو انتهت مدتها.

---

#### `POST /api/guest/cart/:guest_id/items` — إضافة عنصر

**الجسم:**
```json
{ "sku_id": "SKU-123", "quantity": 2, "price": 9.99 }
```

**السلوك:**
- إذا كان `sku_id` موجوداً → تُجمع الكميات ويُحدَّث السعر.
- `quantity <= 0` → `400 invalid quantity`.

---

### سلة المستخدم (محمية بـ JWT)

> جميعها تتطلب هيدر: `Authorization: Bearer <jwt>`

#### `GET /api/cart` — قراءة سلة المستخدم

يقوم تلقائياً بـ:
1. قراءة Redis cache إن وُجد.
2. إن لم يوجد، ينشئ/يجلب `cart_id` من Postgres، يقرأ العناصر، ثم يُخزّن في Redis لمدة ساعة.

---

#### `POST /api/cart/items` — إضافة عنصر واحد

**الجسم:** نفس `AddItemRequest` أعلاه.

---

#### `POST /api/cart/items/bulk` — إضافة دفعة

**الجسم:**
```json
[
  { "sku_id": "SKU-1", "quantity": 2, "price": 5.0 },
  { "sku_id": "SKU-2", "quantity": 1, "price": 12.5 }
]
```

**ملاحظات:**
- قائمة فارغة → `200` بدون تأثير.
- أي كمية `<= 0` → `400 invalid quantity`.

---

#### `POST /api/cart/merge` — دمج سلة الزائر

**الجسم:**
```json
{ "guest_cart_id": "018f2a1b-..." }
```

**السلوك:**
- تُقرأ سلة الزائر، تُدمج عناصرها في سلة المستخدم (upsert).
- إذا كانت سلة الزائر فارغة → تُحذف فقط.
- بعد النجاح، تُحذف سلة الزائر من Redis.
- الاستجابة هي سلة المستخدم المحدّثة.

---

## 🗄 دليل قاعدة البيانات

### جدول `carts`

| العمود | النوع | ملاحظات |
|---|---|---|
| `id` | `TEXT PK` | UUID v7 |
| `user_id` | `TEXT UNIQUE` | معرّف المستخدم |
| `created_at` | `TIMESTAMPTZ` | افتراضي `now()` |
| `updated_at` | `TIMESTAMPTZ` | يُحدّث عند كل تعديل |

### جدول `cart_items`

| العمود | النوع | ملاحظات |
|---|---|---|
| `id` | `TEXT PK` | UUID v7 |
| `cart_id` | `TEXT FK` | → `carts.id` |
| `sku_id` | `TEXT` | ضمن `UNIQUE(cart_id, sku_id)` |
| `quantity` | `INT` | > 0 |
| `price_at_add` | `DOUBLE PRECISION` | |
| `added_at` | `TIMESTAMPTZ` | |

### مفاتيح Redis

| المفتاح | TTL | القيمة |
|---|---|---|
| `cart:guest:{guest_id}` | 7 أيام | JSON لـ `CachedCart` |
| `cart:user:{user_id}` | ساعة | JSON لـ `CachedCart` |

---

## 🧪 اختبارات الخدمة

### ⚙️ تحضير البيئة

الاختبارات **تتطلب** PostgreSQL وRedis حقيقيين (لا mocks). إذا لم تكن متاحة، **تُتخطّى تلقائياً** بدل الفشل.

```bash
# قاعدة بيانات اختبار (منفصلة عن الإنتاج!)
docker run -d --name cart-pg-test \
  -e POSTGRES_PASSWORD=pass \
  -e POSTGRES_DB=cart_test \
  -p 5433:5432 postgres:16

docker run -d --name cart-redis-test -p 6380:6379 redis:7
```

```bash
export DATABASE_URL=postgres://postgres:pass@localhost:5433/cart_test
export REDIS_URL=redis://localhost:6380
export JWT_SECRET=test-secret
```

> ⚠️ **مهم**: المشروع يعرض الوحدات من `src/lib.rs`. تأكد من وجوده ومن أن `main.rs` يستورد من اسم الحزمة.

### ▶️ تشغيل الاختبارات

```bash
# الكل
cargo test -- --test-threads=1

# ملف محدد
cargo test --test guest_cart -- --nocapture

# ملف بملاحظات تسجيل مفصّلة
RUST_LOG=debug cargo test --test http_guest -- --nocapture

# اختبار واحد بالاسم
cargo test reserve_idempotent_same_params_returns_same_id -- --exact --nocapture
```

### 📋 تغطية الاختبارات

#### `tests/guest_cart.rs`
- ✅ `create_get_and_add_items_to_guest_cart` — إنشاء + قراءة + إضافة + تجميع الكمية + تنظيف.
- ✅ `add_guest_item_rejects_non_positive_quantity` — رفض `quantity <= 0`.
- ✅ `get_missing_guest_cart_returns_not_found` — `CartError::NotFound`.

#### `tests/user_cart.rs`
- ✅ `user_cart_roundtrip_and_cache_invalidation` — دورة كاملة + إبطال cache + قراءة من DB.
- ✅ `bulk_add_and_validation` — دفعات + رفض قائمة فيها كمية سالبة.

#### `tests/merge_cart.rs`
- ✅ `merge_guest_into_user_combines_and_deletes_guest` — دمج SKU موجود + جديد، والتأكد من حذف سلة الزائر.
- ✅ `merge_empty_guest_cart_just_deletes_it` — سلة زائر فارغة تُحذف بدون تأثير.

#### `tests/http_guest.rs`
- ✅ `http_guest_flow_create_add_get` — E2E عبر `Router::oneshot`.
- ✅ `http_missing_guest_cart_returns_404` — رمز 404 صحيح.

### 🧹 تنظيف البيانات بعد الاختبارات

الاختبارات تستخدم معرفات عشوائية (`uuid::Uuid::now_v7()`)، لذا لا تتصادم، لكنها **تُنشئ بيانات دائمة** في Postgres. للتنظيف:

```sql
DELETE FROM cart_items WHERE cart_id IN (
  SELECT id FROM carts WHERE user_id LIKE 'test-user-%'
);
DELETE FROM carts WHERE user_id LIKE 'test-user-%';
```

أو أعد إنشاء الـ DB:

```bash
docker exec -it cart-pg-test psql -U postgres -c "DROP DATABASE cart_test;"
docker exec -it cart-pg-test psql -U postgres -c "CREATE DATABASE cart_test;"
```

### 🚫 تشغيل بدون خدمات

```bash
unset DATABASE_URL REDIS_URL
cargo test
# سترى: skip: services unavailable
# النتيجة: كل الاختبارات تنجح (تخطّي)
```

---

## 🔍 استكشاف الأخطاء

| العرض | السبب المحتمل | الحل |
|---|---|---|
| `env variable error: DATABASE_URL` | `.env` مفقود | `cp .env.example .env` |
| `connection refused` عند Redis | Redis لم يبدأ | `docker start cart-redis` |
| `relation "carts" does not exist` | migrations لم تُطبَّق | الخدمة تطبقها تلقائياً، أو `sqlx migrate run` |
| `401 unauthorized` | JWT مفقود/توقيع خاطئ | تحقق من `JWT_SECRET` و`Authorization` header |
| `404` على قراءة سلة زائر موجودة | TTL منتهي (7 أيام) | أنشئ سلة جديدة |
| الاختبارات تُتخطّى دائماً | `DATABASE_URL`/`REDIS_URL` غير مضبوطة | `export` صحيح في نفس الجلسة |
| `duplicate key value` في `cart_items` | migration ناقص `UNIQUE(cart_id, sku_id)` | تحقق من المخطط |

### أوامر تشخيص سريعة

```bash
# فحص Redis
redis-cli -p 6380 KEYS 'cart:*'

# فحص Postgres
psql $DATABASE_URL -c "SELECT * FROM carts LIMIT 5;"

# تفعيل SQL logging
RUST_LOG=sqlx=debug cargo run
```

---

## 📝 ملاحظات تصميمية

1. **`saturating_add`** على الكمية يمنع overflow i32.
2. **bulk upsert** يستخدم `ON CONFLICT (cart_id, sku_id)` — يُقلّل round-trips.
3. **Cache invalidation** يحدث عبر `refresh_user_cache` بعد كل تعديل (لا يُترك للساعة).
4. **`AuthLayer`** يضع `AccessToken` في `Extension` — لا يُلمس بالـ handler مباشرة.
5. **قيم `f64` للسعر** مبسّطة للعرض؛ في الإنتاج يُفضّل `DECIMAL` عبر `rust_decimal`.