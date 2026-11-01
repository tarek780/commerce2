# README.md

````markdown
# Auth Service

خدمة مصادقة (Authentication) مبنية بـ **Rust** و **Axum**، توفر تسجيل المستخدمين، تسجيل الدخول، تدوير رموز التحديث (Refresh Token Rotation)، التحقق من البريد الإلكتروني، وإدارة الملف الشخصي.

---

## المحتويات

- [الميزات](#الميزات)
- [البنية التقنية](#البنية-التقنية)
- [المتطلبات](#المتطلبات)
- [الإعداد](#الإعداد)
- [متغيرات البيئة](#متغيرات-البيئة)
- [التشغيل](#التشغيل)
- [واجهة API](#واجهة-api)
- [قاعدة البيانات](#قاعدة-البيانات)
- [الاختبارات](#الاختبارات)
- [CI/CD](#cicd)
- [البنية](#البنية)
- [الأمان](#الأمان)

---

## الميزات

- ✅ تسجيل مستخدم مع تشفير كلمة المرور بـ **Argon2** (قابل للضبط).
- ✅ تسجيل الدخول وإصدار **Access Token** و **Refresh Token** (JWT).
- ✅ **Refresh Token Rotation** مع كشف إعادة الاستخدام (Token Reuse Detection) وإلغاء العائلة بالكامل عند السرقة.
- ✅ التحقق من البريد الإلكتروني عبر رمز مؤقت في **Redis** مع Rate Limiting.
- ✅ إدارة الملف الشخصي (عرض/تحديث الاسم المعروض).
- ✅ طبقة مصادقة (`AuthLayer`) تستخرج المستخدم من ترويسة `Authorization: Bearer ...`.
- ✅ حذف المستخدم وجميع رموزه.
- ✅ تتبّع الطلبات (Tracing) مع `request_id`.
- ✅ تغطية اختبارات شاملة (تكامل + وحدات) باستخدام **Testcontainers**.

---

## البنية التقنية

| المكوّن | التقنية |
|---|---|
| اللغة | Rust (Edition 2021+) |
| إطار HTTP | Axum 0.7+ |
| قاعدة البيانات | PostgreSQL + SQLx |
| التخزين المؤقت | Redis (ConnectionManager) |
| البريد | Lettre (Async SMTP) |
| JWT | jsonwebtoken (HS256) |
| التشفير | Argon2 |
| الاختبارات | Testcontainers (Postgres + Redis) + Tower ServiceExt |

---

## المتطلبات

- **Rust** 1.75 أو أحدث (`rustup update stable`).
- **Docker** (مطلوب لتشغيل الاختبارات عبر Testcontainers).
- **PostgreSQL** 14+ (أو Docker).
- **Redis** 7+ (أو Docker).
- **SQLx CLI** (اختياري، لإدارة الهجرات):
  ```bash
  cargo install sqlx-cli --no-default-features --features postgres
  ```

---

## الإعداد

```bash
# 1) استنسخ المستودع
git clone <repo-url> auth-service
cd auth-service

# 2) انسخ القالب وعدّل القيم
cp .env.example .env

# 3) شغّل قاعدة البيانات و Redis (عبر Docker Compose)
docker compose up -d postgres redis

# 4) طبّق الهجرات
sqlx migrate run

# 5) شغّل الخدمة
cargo run
```

### مثال `docker-compose.yml`

```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: auth
    ports: ["5432:5432"]
    volumes: ["pgdata:/var/lib/postgresql/data"]

  redis:
    image: redis:7-alpine
    ports: ["6379:6379"]

volumes:
  pgdata:
```

---

## متغيرات البيئة

| المتغير | الوصف | مثال |
|---|---|---|
| `BIND_ADDRESS` | عنوان الاستماع | `0.0.0.0:8080` |
| `DATABASE_URL` | رابط PostgreSQL | `postgres://postgres:postgres@localhost:5432/auth` |
| `REDIS_URL` | رابط Redis | `redis://localhost:6379` |
| `SECRET_KEY` / `SECRSECRET_KEY` | مفتاح توقيع JWT (**≥ 32 بايت**) | `...` |
| `JWT_ACCESS_EXPIRY` | صلاحية Access Token | `15m` |
| `JWT_REFRESH_EXPIRY` | صلاحية Refresh Token | `7d` |
| `JWT_ISSUER` | مُصدر الرمز | `auth-service` |
| `JWT_AUDIENCE` | الجمهور | `auth-service` |
| `ARGON2_MEMORY_COST` | ذاكرة Argon2 (KiB) | `19456` |
| `ARGON2_TIME_COST` | تكرارات Argon2 | `2` |
| `ARGON2_PARALLELISM` | توازي Argon2 | `1` |
| `EMAIL_VERIFICATION_EXPIRY` | صلاحية رمز التحقق | `600` |
| `EMAIL_VERIFY_RATE_LIMIT_SECS` | نافذة Rate Limit | `60` |
| `SMTP_HOST` | مضيف SMTP | `smtp.example.com` |
| `SMTP_PORT` | منفذ SMTP | `587` |
| `SMTP_USERNAME` | مستخدم SMTP | `...` |
| `SMTP_PASSWORD` | كلمة سر SMTP | `...` |
| `SMTP_FROM` | المرسل | `noreply@example.com` |
| `APP_PUBLIC_URL` | رابط الخدمة العام (لبناء رابط التحقق) | `https://api.example.com` |

> ⚠️ **تنبيه**: في الكود الحالي يقرأ `Config` متغير `SECRSECRET_KEY` (خطأ إملائي)، بينما يقرأ `AuthLayer` متغير `SECRET_KEY` مباشرةً. تأكد من تطابق قيمتهما، أو وحّدهما إلى اسم واحد.

---

## التشغيل

### تطوير محلي

```bash
# مع إعادة التحميل التلقائي (يتطلب cargo-watch)
cargo install cargo-watch
cargo watch -x run
```

### إنتاج (Release)

```bash
cargo build --release
./target/release/auth-service
```

### باستخدام Docker

```dockerfile
# Dockerfile
FROM rust:1.79 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/auth-service /usr/local/bin/
COPY --from=builder /app/migrations /app/migrations
WORKDIR /app
EXPOSE 8080
CMD ["auth-service"]
```

```bash
docker build -t auth-service .
docker run --env-file .env -p 8080:8080 auth-service
```

---

## واجهة API

جميع المسارات تبدأ بـ `/api/v1`.

### 🔓 مسارات عامة

#### `POST /api/v1/auth/register`

```http
POST /api/v1/auth/register
Content-Type: application/json

{
  "full_name": "Tarek Ahmed",
  "username": "tarek",
  "u_role": "user",
  "email": "tarek@example.com",
  "password": "supersecret1",
  "display_name": "Tarek"
}
```

**Response 200:**
```json
{
  "user_id": "018f...",
  "tokens": {
    "access_token": "eyJ...",
    "refresh_token": "eyJ...",
    "token_type": "Bearer",
    "expires_in": 900
  }
}
```

**أخطاء:**
- `409 Conflict` — البريد مسجّل مسبقاً.
- `422 Unprocessable Entity` — بيانات غير صحيحة (بريد/كلمة سر).

#### `POST /api/v1/auth/login`

```json
{ "email": "tarek@example.com", "password": "supersecret1" }
```

**Response 200:** نفس بنية التسجيل.

**أخطاء:** `401 Unauthorized` إذا البريد غير موجود أو كلمة السر خاطئة.

#### `POST /api/v1/auth/refresh`

```json
{ "refresh_token": "eyJ..." }
```

**Response 200:**
```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...(جديد)",
  "token_type": "Bearer",
  "expires_in": 900
}
```

**السلوك:** يُدوّر رمز التحديث (mark_rotated) ويُصدر رمزاً جديداً في نفس العائلة.
**كشف السرقة:** إذا أُعيد استخدام رمز مُدوَّر → `401` + إلغاء العائلة كاملة.

#### `POST /api/v1/auth/logout`

```json
{ "refresh_token": "eyJ..." }
```

**Response 200:** `{ "ok": true }`

#### `GET /api/v1/auth/verify-email?token=<vtoken>`

**Response 200:**
```json
{ "ok": true, "email": "tarek@example.com" }
```

**أخطاء:** `4xx` إذا الرمز غير صالح أو منتهٍ أو مُستخدَم.

#### `POST /api/v1/auth/delete`

```json
{ "user_id": "018f..." }
```

> ⚠️ هذا المسار **غير مُسجَّل حالياً** في `routes::build_router`. لتشغيله أضف:
> ```rust
> .route("/api/v1/auth/delete", post(auth::delete))
> ```

**Response 200:** `{ "ok": true }` — يحذف المستخدم وكل رموز التحديث.

### 🔒 مسارات محمية (تتطلب `Authorization: Bearer <access_token>`)

#### `GET /api/v1/users/me`

**Response 200:**
```json
{
  "id": "018f...",
  "email": "tarek@example.com",
  "display_name": "Tarek",
  "email_verified": false,
  "created_at": "2025-01-01T00:00:00Z"
}
```

#### `PATCH /api/v1/users/me`

```json
{ "display_name": "New Name" }
```

**Response 200:** نفس بنية `GET /me` بعد التحديث.
**أخطاء:** `422` إذا `display_name` فارغ.

---

## قاعدة البيانات

### جداول

```sql
-- users
CREATE TABLE users (
    id               TEXT PRIMARY KEY,
    email            TEXT NOT NULL UNIQUE,      -- users_email_key
    password_hash    TEXT NOT NULL,
    display_name     TEXT,
    full_name        TEXT NOT NULL,
    username         TEXT NOT NULL,
    u_role           TEXT NOT NULL DEFAULT 'user',
    email_verified_at TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- refresh_tokens
CREATE TABLE refresh_tokens (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    family_id  TEXT NOT NULL,
    rotated    BOOLEAN NOT NULL DEFAULT FALSE,
    revoked    BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rt_user   ON refresh_tokens(user_id);
CREATE INDEX idx_rt_family ON refresh_tokens(family_id);
CREATE INDEX idx_rt_hash   ON refresh_tokens(token_hash);
```

> ⚠️ **تنبيه توافق**: `RefreshTokenRow` في `db/token_repo.rs` يتوقع حقل `u_role` من `SELECT * FROM refresh_tokens`. لكن الجدول أعلاه لا يحتوي `u_role`. الحل: عدّل `find_by_hash` إلى:
> ```sql
> SELECT rt.*, u.u_role
> FROM refresh_tokens rt
> JOIN users u ON u.id = rt.user_id
> WHERE rt.token_hash = $1
> ```

### الهجرات

```bash
sqlx migrate add create_users
sqlx migrate run
```

---

## الاختبارات

### البنية

```
tests/
├── common/
│   └── mod.rs                 # TestContext، مساعدات HTTP، ثوابت المسارات
├── auth_register_login.rs     # تسجيل / دخول
├── auth_refresh_logout.rs     # تدوير، كشف سرقة، إلغاء
├── auth_me.rs                 # /me، توكنات منتهية/مزوّرة
├── auth_verify_email.rs       # التحقق، لمرة واحدة، rate limit
├── auth_delete.rs             # حذف المستخدم
├── auth_layer.rs              # طبقة المصادقة مباشرة
├── email_store.rs             # issue/consume/rate limit/TTL
├── email_sending.rs           # SMTP فاشل لا يُفشل التسجيل
├── token_repo.rs              # مستودع رموز التحديث
└── user_repo.rs               # مستودع المستخدم
```

### كيف تعمل

- `TestContext::new()` يبدأ حاويتين عبر **Testcontainers**: PostgreSQL + Redis.
- يطبّق الهجرات تلقائياً على قاعدة الاختبار.
- يبني `AppState` مع `Config` مُعدَّة للاختبار (`new_for_tests`): SMTP معطَّل، Argon2 خفيف.
- يبني `Router` من `routes::build_router(state.clone())`.
- كل اختبار يستخدم `truncate_all` لضمان بيئة نظيفة.

### التبعيات (`Cargo.toml`)

```toml
[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
testcontainers = "0.20"
testcontainers-modules = { version = "0.8", features = ["postgres", "redis"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
serde_json = "1"
uuid = { version = "1", features = ["v7"] }
chrono = { version = "0.4", features = ["serde"] }
redis = { version = "0.27", features = ["tokio-comp", "aio", "connection-manager"] }
```

### تشغيل محلي

```bash
# كل الاختبارات (متسلسلة لتفادي تعارض الحاويات)
cargo test --tests -- --test-threads=1

# ملف اختبار واحد
cargo test --test auth_register_login -- --test-threads=1

# اختبار دالة واحدة
cargo test --test auth_register_login register_success_returns_tokens -- --test-threads=1

# مع مخرجات السجل
RUST_LOG=info cargo test --tests -- --test-threads=1 --nocapture
```

### متطلبات الاختبارات

- **Docker** يعمل على الجهاز.
- متغير `SECRET_KEY` يُضبط تلقائياً داخل `TestContext::new`.
- مجلد `migrations/` موجود.

### ملخص التغطية

| الملف | يغطي |
|---|---|
| `auth_register_login.rs` | تسجيل، دخول، تكرار بريد، بريد/كلمة سر غير صحيحة |
| `auth_refresh_logout.rs` | تدوير، كشف إعادة استخدام، إلغاء، logout |
| `auth_me.rs` | me، بدون/مع توكن سيئ/منتهٍ/مزوّر، تحديث الاسم |
| `auth_verify_email.rs` | تحقق ناجح، استخدام لمرة واحدة، رمز غير صالح، rate limit |
| `auth_delete.rs` | حذف المستخدم ورموزه |
| `auth_layer.rs` | قبول/رفض الطبقة وحقن `AccessToken` |
| `email_store.rs` | issue/consume/rate limit/TTL |
| `email_sending.rs` | التسجيل ينجح رغم فشل SMTP |
| `token_repo.rs` | insert/find/mark_rotated/revoke_family/delete |
| `user_repo.rs` | create/duplicate/id/update/mark_verified/delete |

---

## CI/CD

### GitHub Actions

`.github/workflows/test.yml`:

```yaml
name: Tests

on:
  push:
    branches: [main, master]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  SECRET_KEY: "test-secret-key-32-chars-minimum-xx"
  RUST_LOG: "info"

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-

      - name: Verify Docker
        run: docker info

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-targets -- -D warnings

      - name: Run tests
        run: cargo test --tests -- --test-threads=1
```

> 💡 **لماذا لا نحتاج `services`؟** مُشغّل `ubuntu-latest` يأتي مع Docker جاهزاً، و`testcontainers` يتصل بـ `/var/run/docker.sock` ويبدأ الحاويات تلقائياً. هذا يضمن تطابق بيئة CI مع البيئة المحلية.

---

## البنية

```
src/
├── main.rs                 # نقطة الدخول: config، pool، redis، mailer، router
├── config.rs               # قراءة متغيرات البيئة + ConfigError
├── state.rs                # AppState + new_for_tests
├── error.rs                # AppError مع IntoResponse
├── routes.rs               # build_router (public + protected)
├── models/
│   └── user.rs             # User (FromRow)
├── dto/
│   └── auth.rs             # RegisterRequest, LoginRequest, ...
├── db/
│   ├── user_repo.rs        # create/find/update/delete/mark_verified
│   └── token_repo.rs       # insert/find/rotate/revoke
├── helpers/
│   ├── jwt.rs              # issue_token / decode
│   ├── password.rs         # Argon2 hash/verify
│   ├── token.rs            # opaque token + hash_token
│   ├── validation.rs       # validator wrapper
│   ├── email.rs            # SMTP via lettre
│   └── email_store.rs      # Redis: issue/consume verification
├── handlers/
│   ├── auth.rs             # register/login/refresh/logout/delete/verify_email
│   └── user.rs             # get_me / update_me
└── middleware/
    └── auth_layer.rs       # AuthLayer + AccessToken + AuthUser extraction
```

### تدفق الطلب

```
Client → TraceLayer → SetRequestId → SensitiveHeaders
       → [public router | AuthLayer → protected router]
       → handler → repo (SQLx / Redis) → response
```

### تدفق تدوير Refresh Token

```
1. POST /refresh(token)
2. hash_token(token) → find_by_hash → row?
   ├─ غير موجود → 401
   ├─ revoked/rotated → revoke_family + 401  (كشف سرقة)
   └─ منتهٍ → 401
3. mark_rotated(row.id)
4. issue access + refresh جديدين
5. insert_refresh_token(new_hash, نفس family_id)
6. 200 { access, refresh }
```

---

## الأمان

### المُطبَّق

- **Argon2** لتخزين كلمات المرور (قابل للضبط عبر البيئة).
- **JWT HS256** مع `exp` وتحقق إلزامي.
- **Refresh Token Rotation** مع كشف إعادة الاستخدام وإلغاء العائلة.
- **Rate Limiting** على إصدار رموز التحقق عبر Redis.
- **رموز تحقق أحادية الاستخدام** (تُحذف بعد الاستهلاك).
- **`SetSensitiveRequestHeadersLayer`** لإخفاء `Authorization` في السجلات.
- **`request_id`** لكل طلب لتتبّع أفضل.
- **`#[serde(skip_serializing)]`** على `password_hash` لمنع تسريبه.

### توصيات لاحقة

- [ ] إضافة `Cookie` HttpOnly للـ refresh بدل body (لمنع XSS).
- [ ] إضافة حد أقصى لمحاولات تسجيل الدخول (account lockout).
- [ ] استخدام `SECRET_KEY` واحد متسق (توحيد الاسم بين `Config` و`AuthLayer`).
- [ ] إضافة `JWT_ISSUER` و`JWT_AUDIENCE` إلى `Validation` في `decode_jwt`.
- [ ] إضافة **CSRF** عند الانتقال إلى الكوكيز.
- [ ] إضافة **expiración تلقائي** لتنظيف `refresh_tokens` المنتهية (cron/`pg_cron`).
- [ ] تدوير `SECRET_KEY` عبر `kid` (متعدد المفاتيح).

---

## الترخيص

MIT
````

## ملاحظات

- ضع الملف في جذر المستودع باسم `README.md`.
- أضف `.env.example` بجانبه يحتوي على أسماء المتغيرات فقط دون قيم.
- إذا أردت إضافة شارات (badges) أعلى الملف، الصق بعد العنوان الأول:
  ```markdown
  ![Tests](https://github.com/<owner>/<repo>/actions/workflows/test.yml/badge.svg)
  ```