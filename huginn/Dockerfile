# syntax=docker/dockerfile:1.7

# ---------- build ----------
FROM rust:1-bookworm AS builder
WORKDIR /app

# Build context is the workspace root so path dependencies
# (iam-contracts, etc.) are available.
COPY . .

# Cache mounts keep the cargo registry and target dir between builds,
# so only changed crates recompile.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p huginn \
    && cp target/release/huginn /usr/local/bin/huginn

# ---------- runtime ----------
FROM debian:bookworm-slim

# ca-certificates: TLS to IAM / Outbound / Postgres if they use HTTPS.
# curl: used only by the HEALTHCHECK below.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 10001 -s /usr/sbin/nologin huginn

COPY --from=builder /usr/local/bin/huginn /usr/local/bin/huginn

USER huginn
EXPOSE 8081

HEALTHCHECK --interval=15s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -fsS http://localhost:8081/readyz || exit 1

ENTRYPOINT ["huginn"]