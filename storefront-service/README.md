Here is the complete, fully translated English version of the storefront-service README documentation, retaining all structure, diagrams, and RFC 7807 specifications:
# storefront-service

The Backend-for-Frontend (BFF) service for the e-commerce platform. It orchestrates the full checkout journey across cart, category, order, and payment microservices, delivering a unified REST API with JWT authentication, idempotency, and RFC 7807 problem details error responses.

---

## 📑 Table of Contents

- [Overview](#-overview)
- [Architecture](#-architecture)
- [Prerequisites](#-prerequisites)
- [Quick Start](#-quick-start)
- [Environment Configuration](#-environment-configuration)
- [API Endpoints](#-api-endpoints)
- [Authentication (JWT)](#-authentication-jwt)
- [Idempotency](#-idempotency)
- [Error Handling (RFC 7807)](#-error-handling-rfc-7807)
- [Project Structure](#-project-structure)
- [Testing](#-testing)
- [Local Development](#-local-development)
- [Deployment](#-deployment)
- [License](#-license)

---

## 🎯 Overview

`storefront-service` acts as a BFF (Backend-for-Frontend) sitting between web/mobile clients and internal microservices:

| Responsibility | Details |
|----------------|---------|
| **Checkout Journey** | Atomic checkout (price reservation → order creation → payment initiation) with cart flush |
| **Authentication** | JWT (HS256) via `AuthLayer` — access tokens only |
| **Idempotency** | Prevents duplicate checkout processing using `Idempotency-Key` + Redis |
| **Aggregation** | Merges cart state + category catalog pricing + order status into unified DTOs |
| **Security** | Ensures strict resource ownership (users cannot access others' orders) |
| **Observability** | Health/ready checks + structured JSON logging |

**Key Capabilities:**

- ✅ Order ownership checks — returns `404` instead of `403` to prevent resource enumeration leaks.
- ✅ Graceful degradation — failures during cart clearing do not abort a successful checkout (surfaced as `warnings`).
- ✅ Idempotent checkout — repeated requests using the same idempotency key return the cached response.
- ✅ Standardized RFC 7807 Problem Details responses.
- ✅ Hermetic integration tests running without external network dependencies.

---

## 🏗 Architecture


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

**Expected Upstream Microservices:**

| Service | Protocol | Purpose |
|---------|----------|---------|
| `cart-service` | HTTP REST | Read items / clear user shopping cart |
| `category-service` | GraphQL | Resolve variant pricing and metadata |
| `order-service` | gRPC | Create, query, cancel, and initiate order payments |
| `payment-service` | gRPC | Query payment session status |
| `redis` | RESP | Store idempotency locks and cached payloads |

---

## 📦 Prerequisites

- **Rust** 1.82+ (Edition 2021)
- **Redis** 7+ (Required for runtime only; test suite uses an in-memory mock)
- Reachable endpoints for `cart`, `category`, `order`, and `payment` services (or mocks during development)

---

## 🚀 Quick Start

```bash
# 1. Clone repository
git clone <repo> && cd storefront-service

# 2. Configure environment
cp .env.example .env
# Edit .env according to your local microservices setup

# 3. Build and run
cargo run --release

# 4. Verify health check
curl http://localhost:8080/health
# {"status":"ok"}

⚙️ Environment Configuration
Configuration options are parsed via AppConfig::from_env():
| Variable | Default | Description |
|---|---|---|
| HTTP_ADDR | 0.0.0.0:8080 | Bind address for HTTP server |
| SECRET_KEY | Required | Secret key for JWT HS256 signature verification |
| CURRENCY | USD | Default currency code |
| CART_BASE_URL | Required | Base URL for cart-service |
| CART_CLEAR_ENDPOINT | /api/cart | Endpoint path to clear cart |
| CATEGORY_GRAPHQL_URL | Required | GraphQL endpoint for catalog prices |
| ORDER_GRPC_URL | Required | gRPC endpoint for order-service |
| PAYMENT_GRPC_URL | Required | gRPC endpoint for payment-service |
| INVENTORY_GRPC_URL | Required | gRPC endpoint for inventory-service |
| REDIS_URL | redis://127.0.0.1:6379 | Redis connection URL |
| IDEMPOTENCY_TTL_SECS | 86400 | TTL for stored idempotency keys (seconds) |
| IDEMPOTENCY_REQUIRED | true | Whether Idempotency-Key header is mandatory for checkout |
| UPSTREAM_TIMEOUT_SECS | 10 | Timeout threshold for upstream calls (seconds) |
Sample .env.example
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

🌐 API Endpoints
Health Checks (Public)
| Method | Path | Response |
|---|---|---|
| GET | /health | {"status":"ok"} |
| GET | /ready | {"status":"ready"} |
Checkout (Protected)
POST /api/v1/checkout
Executes the atomic checkout flow: fetch cart → lock catalog prices → create order → trigger payment session → flush cart.
Required Headers:
| Header | Value |
|---|---|
| Authorization | Bearer <access_jwt> |
| Idempotency-Key | UUID string (Mandatory if IDEMPOTENCY_REQUIRED=true) |
Request Body: {} (Empty JSON object; cart items are retrieved from cart-service)
Response 201 Created:
{
  "order_id": "order-1",
  "payment_id": "pay-1",
  "payment_url": "[https://pay.example.com/session/abc](https://pay.example.com/session/abc)",
  "status": "processing",
  "total_amount": "60.00",
  "currency": "USD",
  "warnings": []
}

Response when Cart Flush fails (remains 201 Created):
{
  "order_id": "order-1",
  "payment_id": "pay-1",
  "payment_url": "[https://pay.example.com/session/abc](https://pay.example.com/session/abc)",
  "status": "processing",
  "total_amount": "60.00",
  "currency": "USD",
  "warnings": ["cart_not_cleared"]
}

Orders (Protected)
| Method | Path | Description |
|---|---|---|
| GET | /api/v1/orders?first=10&after=<cursor>&status=<filter> | List user orders (cursor pagination) |
| GET | /api/v1/orders/{id} | Get single order (with ownership check) |
| POST | /api/v1/orders/{id}/cancel | Cancel order (body: {"reason":"..."}) |
| POST | /api/v1/orders/{id}/pay | Initiate payment for an existing order |
GET /api/v1/orders — Response Schema
{
  "edges": [
    { "node": { "id": "order-1", "status": "awaiting_payment" } }
  ],
  "page_info": { "has_next_page": true, "end_cursor": "cursor2" }
}

Payments (Protected)
| Method | Path | Description |
|---|---|---|
| GET | /api/v1/payments/{id} | Fetch payment session status |
🔐 Authentication (JWT)
Protected endpoints require a valid Access Token passed in the Authorization header.
Token Specification
 * Algorithm: HS256
 * Secret Key: Parsed from SECRET_KEY
 * Payload Claims:
   {
  "typ": "Access",
  "user_id": "<uuid>",
  "token_id": "<uuid>",
  "role": "User | Admin",
  "exp": 1735689600
}

Example Request
curl -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
     -H "Idempotency-Key: $(uuidgen)" \
     -X POST http://localhost:8080/api/v1/checkout

AuthLayer Behavior Matrix
| State | Result |
|---|---|
| Missing Authorization header | 401 Unauthorized |
| Authorization scheme is not Bearer  | 401 Unauthorized |
| Invalid token format / wrong secret / expired | 401 Unauthorized |
| Valid Access Token | Request proceeds — injects AccessToken { user_id } into request extensions |
🔁 Idempotency
POST /api/v1/checkout is protected against duplicate operations via Idempotency-Key.
Execution Flow
1. Client sends Idempotency-Key: <uuid> header.
2. Service executes atomic SET NX key in Redis.
   ├─ Success → Key is new → Process checkout execution.
   └─ Failure → Key already exists:
       ├─ Value is "__PENDING__" → 409 Conflict (Concurrent in-flight request).
       └─ Value is a serialized response payload → Return cached response immediately.
3. Upon completion → SET key = <response_body> EX IDEMPOTENCY_TTL_SECS.

Idempotency Scenarios
| Scenario | Behavior |
|---|---|
| First request with unique key | Executes checkout, returns 201, caches payload |
| Re-sent request within TTL window | Returns 201 with cached response body |
| Concurrent request in progress | Returns 409 Conflict |
| System disconnect during pending state | Returns 409 Conflict until key TTL expires |
🧯 Error Handling (RFC 7807)
Errors follow the application/problem+json format:
{
  "type": "[https://storefront.example.com/errors/insufficient-stock](https://storefront.example.com/errors/insufficient-stock)",
  "title": "Insufficient Stock",
  "status": 402,
  "detail": "variant 1 requested 5, available 3",
  "instance": "/api/v1/checkout"
}

Status Mapping
| HTTP Status | type (Slug) | Trigger Condition |
|---|---|---|
| 400 | validation | Invalid JSON body or missing Idempotency-Key header |
| 401 | — | Token missing or invalid (AuthLayer returns empty body) |
| 402 | insufficient-stock | Out-of-stock items in cart |
| 404 | not-found | Order/Payment resource not found or unauthorized access |
| 409 | idempotency-conflict | In-flight request currently processing with the same key |
| 500 | internal | Unhandled runtime or database error |
| 502 | upstream | Failed integration with cart, category, or order upstream services |
📂 Project Structure
storefront-service/
├── Cargo.toml
├── README.md
├── .env.example
├── src/
│   ├── lib.rs                 # Core library module (exported for tests)
│   ├── main.rs                # Application entry point
│   ├── config.rs              # AppConfig::from_env parser
│   ├── state.rs               # AppState dependency injection context
│   ├── routes.rs              # Axum router construction & layer application
│   ├── error.rs               # AppError mapping to RFC 7807 ProblemDetails
│   ├── problem.rs             # RFC 7807 problem details structure
│   ├── openapi.rs             # OpenAPI/utoipa specs
│   │
│   ├── middleware/
│   │   └── auth.rs            # AuthLayer, AccessToken, and DecodedToken definitions
│   │
│   ├── helpers/
│   │   ├── idempotency.rs     # IdempotencyStore trait (Redis & Memory implementations)
│   │   ├── cursor.rs          # Cursor encoding/decoding for pagination
│   │   └── jwt.rs             # JWT verification logic
│   │
│   ├── handlers/
│   │   ├── checkout.rs        # POST /api/v1/checkout handler
│   │   ├── orders.rs          # /api/v1/orders/* handlers
│   │   ├── payments.rs        # /api/v1/payments/* handlers
│   │   └── health.rs          # /health and /ready handlers
│   │
│   ├── clients/
│   │   ├── cart.rs            # CartClient trait + HttpCartClient
│   │   ├── category.rs        # CategoryClient trait
│   │   ├── order.rs           # OrderClient trait + gRPC client bindings
│   │   └── payment.rs         # PaymentClient trait + gRPC client bindings
│   │
│   └── dto/
│       ├── cart.rs
│       ├── checkout.rs
│       ├── orders.rs
│       └── payments.rs
│
└── tests/
    ├── common/
    │   ├── mod.rs             # Test harness and HTTP helper functions
    │   ├── auth.rs            # Local JWT token generation helpers
    │   ├── fixtures.rs        # Mock data builders
    │   └── mocks/
    │       ├── mod.rs
    │       ├── cart.rs
    │       ├── category.rs
    │       ├── order.rs
    │       └── payment.rs
    │
    ├── auth_test.rs           # JWT authentication test suite
    ├── checkout_test.rs       # Checkout integration tests
    ├── orders_test.rs         # Order CRUD and authorization tests
    ├── payments_test.rs       # Payment status query tests
    ├── idempotency_test.rs    # Idempotent behavior tests
    ├── health_test.rs         # Liveness/Readiness endpoint tests
    └── problem_test.rs        # RFC 7807 schema compliance tests

🧪 Testing
Test Suite Principles
Integration tests run completely in-memory without external services:
 * ✅ No network containers required: Runs without Postgres, Redis, gRPC servers, or external authentication providers.
 * ✅ Trait-based Client Mocks: All microservice communication utilizes in-memory mocks inside tests/common/mocks/.
 * ✅ In-Memory Idempotency: Swaps Redis with MemoryIdempotencyStore.
 * ✅ In-Process Router Invocation: Tests dispatch directly via tower::ServiceExt::oneshot.
 * ✅ Deterministic Tokens: Signs JWTs locally using the shared SECRET_KEY.
Running Tests
# Execute entire test suite
cargo test -p storefront-service

# Run a specific test binary
cargo test -p storefront-service --test checkout_test

# Run a single test case
cargo test -p storefront-service --test auth_test expired_jwt_returns_401

# Run with stdout visible
cargo test -p storefront-service -- --nocapture

# Run sequentially (if needed)
cargo test -p storefront-service -- --test-threads=1

# Generate test coverage report
cargo install cargo-tarpaulin
cargo tarpaulin -p storefront-service --out Html --output-dir coverage

🛠 Local Development
Running with Local Microservices
# Spin up infrastructure dependencies via Docker Compose
docker compose -f docker-compose.dev.yml up -d

# Run application locally
cargo run

# Mint dev token and test checkout
TOKEN=$(./scripts/mint-jwt.sh 550e8400-e29b-41d4-a716-446655440000)
curl -H "Authorization: Bearer $TOKEN" \
     -H "Idempotency-Key: $(uuidgen)" \
     -X POST http://localhost:8080/api/v1/checkout

JWT Development Minting Script (scripts/mint-jwt.sh)
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

🚢 Deployment
Multi-Stage Dockerfile
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

Kubernetes Health Probes
livenessProbe:
  httpGet: { path: /health, port: 8080 }
  initialDelaySeconds: 5
  periodSeconds: 10

readinessProbe:
  httpGet: { path: /ready, port: 8080 }
  initialDelaySeconds: 5
  periodSeconds: 5

📝 License
Distributed under the MIT License.

