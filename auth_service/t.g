name: Tests

on:
  push:
    branches: [main, master]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  # يجب أن يتطابق مع TestContext::new في اختباراتك
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

      - name: Verify Docker is available
        run: docker info

      - name: Run tests
        run: cargo test --tests -- --test-threads=1