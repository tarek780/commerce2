FROM rust:1.99.1-slim-trixie AS builder
WORKDIR /usr/src/app


RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libssl-dev \
    pkg-config \
    curl \
    ca-certificates \
    cmake \
    g++ \
    && rm -rf /var/lib/apt/lists/*


COPY Cargo.toml Cargo.lock ./


COPY migrations ./migrations
COPY src ./src


RUN --mount=type=cache,id=auth_service-cargo-registry,target=/usr/local/cargo/registry \

    --mount=type=cache,id=auth_service-cargo-target-deps,target=/usr/src/app/target/release/deps \
    cargo test --release && \
    cargo build --release && \
    cp target/release/auth_service /usr/local/bin/auth_service


FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*


COPY --from=builder /usr/local/bin/auth_service /usr/local/bin/auth_service

CMD ["auth_service"]
