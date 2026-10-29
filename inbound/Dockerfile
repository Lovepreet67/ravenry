FROM rust:1-bookworm AS builder
WORKDIR /app

COPY . .
RUN cargo build --release -p inbound

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/inbound /usr/local/bin/inbound
CMD ["inbound"]