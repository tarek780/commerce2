
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
COPY proto ./proto
COPY src ./src


RUN --mount=type=cache,id=inventory_servise-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=inventory_servise-cargo-target-deps,target=/usr/src/app/target/release/deps \
    cargo test --release && \
    cargo build --release && \
    cp target/release/inventory_servise /usr/local/bin/inventory_servise


FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*


COPY --from=builder /usr/local/bin/inventory_servise /usr/local/bin/inventory_servise

CMD ["inventory_servise"]
