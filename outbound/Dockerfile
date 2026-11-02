
# Chef stage
FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Planner stage
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage
FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json

# Cache dependencies
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release -p outbound

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/outbound /usr/local/bin/outbound

CMD ["outbound"]
