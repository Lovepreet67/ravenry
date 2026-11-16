# syntax=docker/dockerfile:1.7

# ---------- build ----------
FROM rust:1-bookworm AS builder
WORKDIR /app
ENV SQLX_OFFLINE=true

# Build context is the workspace root so path dependencies
# (iam-contracts, etc.) are available.
COPY . .

# Cache mounts keep the cargo registry and target dir between builds,
# so only changed crates recompile.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p iam \
    && cp target/release/iam /usr/local/bin/iam

# ---------- runtime ----------
FROM debian:bookworm-slim

# ca-certificates: TLS to IAM / Outbound / Postgres if they use HTTPS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 10001 -s /usr/sbin/nologin iam

COPY --from=builder /usr/local/bin/iam /usr/local/bin/iam

USER iam
EXPOSE 3001

ENTRYPOINT ["iam"]
