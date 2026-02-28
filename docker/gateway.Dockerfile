# Stage 1: Build
FROM rust:1.77-bookworm AS builder
WORKDIR /app
COPY gateway/Cargo.toml gateway/Cargo.lock ./
# Create dummy src for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src
COPY gateway/src ./src
COPY gateway/migrations ./migrations
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/openclaw-gateway .
COPY gateway/migrations ./migrations
EXPOSE 8080
ENV RUST_LOG=info
CMD ["./openclaw-gateway"]
