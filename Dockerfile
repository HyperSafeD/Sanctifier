# Stage 1: Build the Sanctifier CLI
FROM rust:1.75-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY tooling/sanctifier-core/Cargo.toml tooling/sanctifier-core/
COPY tooling/sanctifier-cli/Cargo.toml tooling/sanctifier-cli/
COPY contracts/vulnerable-contract/Cargo.toml contracts/vulnerable-contract/
COPY contracts/kani-poc/Cargo.toml contracts/kani-poc/
COPY contracts/amm-pool/Cargo.toml contracts/amm-pool/

# Create dummy source files for dependency caching
RUN mkdir -p tooling/sanctifier-core/src && echo "pub fn dummy() {}" > tooling/sanctifier-core/src/lib.rs \
    && mkdir -p tooling/sanctifier-cli/src && echo "fn main() {}" > tooling/sanctifier-cli/src/main.rs \
    && mkdir -p contracts/vulnerable-contract/src && echo "#![no_std]" > contracts/vulnerable-contract/src/lib.rs \
    && mkdir -p contracts/kani-poc/src && echo "#![no_std]" > contracts/kani-poc/src/lib.rs \
    && mkdir -p contracts/amm-pool/src && echo "" > contracts/amm-pool/src/lib.rs

# Build dependencies only (cached layer)
RUN cargo build --release --package sanctifier-cli 2>/dev/null || true

# Copy actual source code
COPY tooling/ tooling/
COPY contracts/ contracts/

# Build the real binary
RUN cargo build --release --package sanctifier-cli

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/sanctifier-cli /usr/local/bin/sanctifier

WORKDIR /workspace

ENTRYPOINT ["sanctifier"]
CMD ["--help"]
