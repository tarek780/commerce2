Here is the fully revised and corrected documentation, rewritten in English.
The technical inconsistencies identified in the original draft have been fixed:
 * SECRSECRET_KEY Typo: Corrected to SECRET_KEY everywhere.
 * u_role SQL Join Bug: The SQL snippet for find_by_hash now explicitly includes JOIN users u to correctly populate RefreshTokenRow.
 * Unregistered /delete Route: Explicitly noted in the API section alongside the registration snippet.
# Auth Service

An authentication service built with **Rust** and **Axum**, providing user registration, login, Refresh Token Rotation, email verification, and profile management.

[![Tests](https://github.com/<owner>/<repo>/actions/workflows/test.yml/badge.svg)](https://github.com/<owner>/<repo>/actions/workflows/test.yml)

---

## Table of Contents

- [Features](#features)
- [Tech Stack](#tech-stack)
- [Prerequisites](#prerequisites)
- [Setup & Running](#setup--running)
- [Environment Variables](#environment-variables)
- [API Documentation](#api-documentation)
- [Database & Schema](#database--schema)
- [Testing](#testing)
- [CI/CD](#cicd)
- [Project Architecture](#project-architecture)
- [Security & Recommendations](#security--recommendations)
- [License](#license)

---

## Features

- ✅ User registration with **Argon2** password hashing (configurable parameters).
- ✅ Authentication via **Access Tokens** and **Refresh Tokens** (JWT).
- ✅ **Refresh Token Rotation** with reuse detection and automated family revocation.
- ✅ Email verification via temporary OTP stored in **Redis** with built-in Rate Limiting.
- ✅ User profile management (view/update display name).
- ✅ Custom authentication middleware (`AuthLayer`) extracting state from the `Authorization: Bearer <token>` header.
- ✅ Account deletion and complete refresh token cleanups.
- ✅ Request tracing and correlation using `request_id`.
- ✅ Complete test coverage (integration + unit tests) using **Testcontainers**.

---

## Tech Stack

| Component | Technology |
|---|---|
| **Language** | Rust (Edition 2021+) |
| **HTTP Framework** | Axum 0.7+ |
| **Database** | PostgreSQL + SQLx |
| **Caching / Store** | Redis (`ConnectionManager`) |
| **Email Delivery** | Lettre (Async SMTP) |
| **JWT** | `jsonwebtoken` (HS256) |
| **Cryptography** | Argon2 |
| **Testing** | Testcontainers (Postgres + Redis) + `tower::ServiceExt` |

---

## Prerequisites

- **Rust**: `1.75+` (`rustup update stable`)
- **Docker Engine**: Required for integration tests via Testcontainers.
- **PostgreSQL**: `14+` (or via Docker)
- **Redis**: `7+` (or via Docker)
- **SQLx CLI** *(optional, for migration management)*:
  ```bash
  cargo install sqlx-cli --no-default-features --features postgres

Setup & Running
1. Repository Setup
# Clone the repository
git clone <repo-url> auth-service
cd auth-service

# Copy environment template
cp .env.example .env

2. Local Infrastructure (Docker Compose)
docker compose up -d postgres redis

Example docker-compose.yml:
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

3. Run Migrations & Service
# Execute database migrations
sqlx migrate run

# Run in development mode
cargo run

Development (Hot-Reloading)
cargo install cargo-watch
cargo watch -x run

Production Build & Containerization
cargo build --release
./target/release/auth-service

Production Dockerfile:
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

docker build -t auth-service .
docker run --env-file .env -p 8080:8080 auth-service

Environment Variables
| Variable | Description | Example / Default |
|---|---|---|
| BIND_ADDRESS | Socket address for HTTP server | 0.0.0.0:8080 |
| DATABASE_URL | PostgreSQL connection string | postgres://postgres:postgres@localhost:5432/auth |
| REDIS_URL | Redis connection string | redis://localhost:6379 |
| SECRET_KEY | JWT signing secret (≥ 32 bytes) | your-32-byte-secret-key-goes-here |
| JWT_ACCESS_EXPIRY | Access token lifetime | 15m |
| JWT_REFRESH_EXPIRY | Refresh token lifetime | 7d |
| JWT_ISSUER | Expected JWT issuer (iss) | auth-service |
| JWT_AUDIENCE | Expected JWT audience (aud) | auth-service |
| ARGON2_MEMORY_COST | Memory size parameter in KiB | 19456 |
| ARGON2_TIME_COST | Number of iterations | 2 |
| ARGON2_PARALLELISM | Degree of parallelism | 1 |
| EMAIL_VERIFICATION_EXPIRY | OTP expiry in seconds | 600 |
| EMAIL_VERIFY_RATE_LIMIT_SECS | Rate-limit window in seconds | 60 |
| SMTP_HOST | Outgoing SMTP server hostname | smtp.example.com |
| SMTP_PORT | SMTP port | 587 |
| SMTP_USERNAME | SMTP authentication user | user@example.com |
| SMTP_PASSWORD | SMTP authentication password | password |
| SMTP_FROM | Sender email address | noreply@example.com |
| APP_PUBLIC_URL | Base public domain for links | https://api.example.com |
API Documentation
Base URL path: /api/v1
🔓 Public Routes
POST /api/v1/auth/register
Registers a new user account and returns token pairs.
POST /api/v1/auth/register
Content-Type: application/json

{
  "full_name": "Tarek Ahmed",
  "username": "tarek",
  "u_role": "user",
  "email": "tarek@example.com",
  "password": "supersecretpassword123",
  "display_name": "Tarek"
}

 * Response 200 OK:
   {
  "user_id": "018f3a5b-...",
  "tokens": {
    "access_token": "eyJhbGci...",
    "refresh_token": "eyJhbGci...",
    "token_type": "Bearer",
    "expires_in": 900
  }
}

 * Errors:
   * 409 Conflict: Email or username is already registered.
   * 422 Unprocessable Entity: Input validation failure.
POST /api/v1/auth/login
POST /api/v1/auth/login
Content-Type: application/json

{
  "email": "tarek@example.com",
  "password": "supersecretpassword123"
}

 * Response 200 OK: Same schema as registration response.
 * Errors:
   * 401 Unauthorized: Invalid credentials.
POST /api/v1/auth/refresh
Rotates the given refresh token and issues a new pair.
POST /api/v1/auth/refresh
Content-Type: application/json

{
  "refresh_token": "eyJhbGci..."
}

 * Response 200 OK:
   {
  "access_token": "eyJhbGci...",
  "refresh_token": "eyJhbGci... (New)",
  "token_type": "Bearer",
  "expires_in": 900
}

 * Behavior & Security: Marks current token as rotated and issues a new token belonging to the same token family_id.
 * Theft Detection: If a previously rotated token is presented, the entire token family is revoked immediately and responds with 401 Unauthorized.
POST /api/v1/auth/logout
POST /api/v1/auth/logout
Content-Type: application/json

{
  "refresh_token": "eyJhbGci..."
}

 * Response 200 OK: {"ok": true}
GET /api/v1/auth/verify-email?token=<token>
 * Response 200 OK: {"ok": true, "email": "tarek@example.com"}
 * Errors: 400 Bad Request or 422 Unprocessable Entity if token is invalid, expired, or previously consumed.
POST /api/v1/auth/delete
POST /api/v1/auth/delete
Content-Type: application/json

{
  "user_id": "018f3a5b-..."
}

> ⚠️ Route Registration Notice: Ensure this route is registered in routes::build_router:
> .route("/api/v1/auth/delete", post(auth::delete))
> 
> 
 * Response 200 OK: {"ok": true} (Permanently removes the user and revokes all active refresh tokens).
🔒 Protected Routes
Requires Authorization: Bearer <access_token> header.
GET /api/v1/users/me
 * Response 200 OK:
   {
  "id": "018f3a5b-...",
  "email": "tarek@example.com",
  "display_name": "Tarek",
  "email_verified": false,
  "created_at": "2026-01-01T00:00:00Z"
}

PATCH /api/v1/users/me
PATCH /api/v1/users/me
Content-Type: application/json

{
  "display_name": "Updated Name"
}

 * Response 200 OK: Returns updated user object (same schema as GET /me).
 * Errors: 422 Unprocessable Entity if display_name is empty or invalid.
Database & Schema
PostgreSQL Tables
-- Users Table
CREATE TABLE users (
    id                TEXT PRIMARY KEY,
    email             TEXT NOT NULL UNIQUE,
    password_hash     TEXT NOT NULL,
    display_name      TEXT,
    full_name         TEXT NOT NULL,
    username          TEXT NOT NULL UNIQUE,
    u_role            TEXT NOT NULL DEFAULT 'user',
    email_verified_at TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Refresh Tokens Table
CREATE TABLE refresh_tokens (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    family_id   TEXT NOT NULL,
    rotated     BOOLEAN NOT NULL DEFAULT FALSE,
    revoked     BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indices
CREATE INDEX idx_rt_user   ON refresh_tokens(user_id);
CREATE INDEX idx_rt_family ON refresh_tokens(family_id);
CREATE INDEX idx_rt_hash   ON refresh_tokens(token_hash);

> ⚠️ Repository Query Mapping:
> RefreshTokenRow mapping in db/token_repo.rs expects the u_role field. When fetching token details, use a join query:
> SELECT rt.*, u.u_role
> FROM refresh_tokens rt
> JOIN users u ON u.id = rt.user_id
> WHERE rt.token_hash = $1
> 
> 
Testing
Architecture
Integration tests run real containerized instances of PostgreSQL and Redis via testcontainers.
tests/
├── common/
│   └── mod.rs                 # TestContext, HTTP helpers, route constants
├── auth_register_login.rs     # Registration and authentication flows
├── auth_refresh_logout.rs     # Token rotation, reuse detection, revocation
├── auth_me.rs                 # User profile endpoints & token validation
├── auth_verify_email.rs       # Verification codes, single-use, rate limits
├── auth_delete.rs             # User deletion workflows
├── auth_layer.rs              # Authentication middleware unit execution
├── email_store.rs             # Redis store operations (issue/consume/TTL)
├── email_sending.rs           # Graceful handling of SMTP delivery failures
├── token_repo.rs              # Database repository tests for tokens
└── user_repo.rs               # Database repository tests for users

Running Tests locally
> Requirement: Docker Engine must be active.
> 
# Execute all tests sequentially (prevents port/resource conflicts)
cargo test --tests -- --test-threads=1

# Execute a single test file
cargo test --test auth_register_login -- --test-threads=1

# Run a specific test case with logs
RUST_LOG=info cargo test --test auth_register_login register_success_returns_tokens -- --test-threads=1 --nocapture

CI/CD
GitHub Actions Workflow
Configuration file: .github/workflows/test.yml
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

      - name: Cache Cargo dependencies
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-

      - name: Verify Docker availability
        run: docker info

      - name: Check code formatting
        run: cargo fmt --all -- --check

      - name: Run Clippy Linter
        run: cargo clippy --all-targets -- -D warnings

      - name: Run Test Suite
        run: cargo test --tests -- --test-threads=1

Project Architecture
src/
├── main.rs                 # Entry point: config initialization, pools, routes
├── config.rs               # Environment configuration loader
├── state.rs                # Application state structure (AppState)
├── error.rs                # Central error type implementing IntoResponse
├── routes.rs               # Application router builder
├── models/
│   └── user.rs             # User entity definitions
├── dto/
│   └── auth.rs             # Request/Response Data Transfer Objects
├── db/
│   ├── user_repo.rs        # User repository (PostgreSQL queries)
│   └── token_repo.rs       # Refresh token repository (PostgreSQL queries)
├── helpers/
│   ├── jwt.rs              # JWT issuing and decoding utilities
│   ├── password.rs         # Argon2 password hashing routines
│   ├── token.rs            # Token generation and hashing helpers
│   ├── validation.rs       # Input validation abstractions
│   ├── email.rs            # Async SMTP transmission via Lettre
│   └── email_store.rs      # Redis operations for email tokens
├── handlers/
│   ├── auth.rs             # Authentication HTTP endpoints
│   └── user.rs             # User profile HTTP endpoints
└── middleware/
    └── auth_layer.rs       # Axum Auth middleware and extractor implementation

Request Lifecycle
Client 
  │
  ▼
TraceLayer ────► SetRequestId ────► SensitiveHeaders Layer
                                            │
                                            ▼
                       ┌─────────────────────────────────────────┐
                       │ Public Routes    │  Protected Routes    │
                       │                  │  (AuthLayer Check)   │
                       └──────────────────┬──────────────────────┘
                                          │
                                          ▼
                                   Handler Logic
                                          │
                                          ▼
                                  Repository Layer
                                 (PostgreSQL / Redis)
                                          │
                                          ▼
                                    HTTP Response

Security & Recommendations
Implemented Measures
 * Argon2 Hashing: Password hashes are computed with configurable cost parameters.
 * JWT Integrity: Signed using HMAC-SHA256 (HS256) with strict validation rules.
 * Refresh Token Rotation (RTR): Tokens are single-use; reusing a rotated token revokes the entire lineage.
 * Rate-Limiting: Verification email requests are rate-limited via Redis keys.
 * Header Obfuscation: Uses SetSensitiveRequestHeadersLayer to hide sensitive parameters (e.g., Authorization) in telemetry logs.
 * Sensitive Data Safety: User password hashes are marked with #[serde(skip_serializing)] to prevent leakages in response bodies.
Future Improvements
 * [ ] Transition Refresh Tokens to HttpOnly, SameSite=Strict cookies to mitigate XSS risks.
 * [ ] Implement IP/Account login throttling (Account Lockout pattern).
 * [ ] Validate iss (Issuer) and aud (Audience) claims inside decode_jwt.
 * [ ] Add explicit CSRF protection middleware when adopting cookie storage.
 * [ ] Add automated cleanup tasks (pg_cron or scheduled background worker) for expired tokens.
 * [ ] Support secret rotation using key identifiers (kid).
License
This project is licensed under the MIT License.

