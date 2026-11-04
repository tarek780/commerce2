
FROM rust:1.99.1-slim-trixie AS builder
WORKDIR /usr/src/app

RUN apt-get update && apt-get upgrade -y

RUN apt-get install -y protobuf-compiler libssl-dev pkg-config curl ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() { println!(\"dummy\"); }" > src/main.rs
RUN cargo build --release
RUN rm -rf src



COPY proto ./proto
COPY src ./src


RUN touch src/main.rs 
RUN cargo test  --release
RUN cargo build --release


FROM debian:trixie-slim
RUN apt-get update && apt-get upgrade -y && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*


COPY --from=builder /usr/src/app/target/release/storefront-service /usr/local/bin/storefront-service

CMD ["storefront-service"]
