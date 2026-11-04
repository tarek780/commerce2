
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



COPY proto ./proto
COPY src ./src


RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo test --release && \
    cargo build --release && \
    cp target/release/storefront-service /usr/local/bin/storefront-service


FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*


COPY --from=builder /usr/local/bin/storefront-service /usr/local/bin/storefront-service

CMD ["storefront-service"]
