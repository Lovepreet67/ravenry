FROM rust:1-bookworm AS builder
WORKDIR /app

COPY . .
RUN cargo build --release -p iam

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/iam /usr/local/bin/iam
CMD ["iam"]