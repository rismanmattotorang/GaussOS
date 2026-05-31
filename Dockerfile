# GaussOS — multi-stage build producing a small runtime image.
# Build:  docker build -t gaussos:latest .
# Run:    docker run -p 8080:8080 gaussos:latest

# ---- Builder ----
FROM rust:1.83-slim AS builder
WORKDIR /app

# System deps for the build (rocksdb/surrealdb need clang/libclang; tls needs pkg-config).
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev clang libclang-dev cmake \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies first.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/dummy.rs \
    && cargo fetch

# Build the release server binary (cli-bin feature provides `gaussos`).
COPY . .
RUN cargo build --release --features cli-bin --bin gaussos

# ---- Runtime ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10001 gaussos

COPY --from=builder /app/target/release/gaussos /usr/local/bin/gaussos

USER gaussos
EXPOSE 8080
# Default backend is the embedded SurrealDB (no external DB needed).
ENV RUST_LOG=info
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD ["/usr/local/bin/gaussos", "--version"]
ENTRYPOINT ["/usr/local/bin/gaussos"]
CMD ["server", "--host", "0.0.0.0", "--port", "8080"]
