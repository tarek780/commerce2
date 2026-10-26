
FROM rust:1.99.1-slim-trixie AS builder
WORKDIR /usr/src/app


RUN apt-get update && apt-get upgrade -y && \
    apt-get install -y protobuf-compiler libssl-dev pkg-config && \
    rm -rf /var/lib/apt/lists/*


COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() { println!(\"dummy\"); }" > src/main.rs
RUN cargo build --release
RUN rm -rf src


COPY migrations ./migrations
COPY proto ./proto
COPY src ./src


RUN touch src/main.rs && cargo build --release


FROM debian:trixie-slim
RUN apt-get update && apt-get upgrade -y && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*


COPY --from=builder /usr/src/app/target/release/payment_service /usr/local/bin/payment_service

CMD ["payment_service"]
