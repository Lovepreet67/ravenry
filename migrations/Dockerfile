
FROM rust:1-bookworm AS builder

RUN cargo install sqlx-cli \
    --version 0.8.6 \
    --locked \
    --no-default-features \
    --features postgres

FROM rust:1-slim-bookworm

WORKDIR /app

COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx
COPY iam/migrations/ /app/migrations/

CMD ["sqlx", "migrate", "run", "--source", "/app/migrations"]
