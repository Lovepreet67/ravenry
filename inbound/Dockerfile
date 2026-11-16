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
    cargo build --release -p inbound \
    && cp target/release/inbound /usr/local/bin/inbound

# ---------- runtime ----------
FROM debian:bookworm-slim

# ca-certificates: TLS to IAM / Outbound / Postgres if they use HTTPS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 10001 -s /usr/sbin/nologin inbound

COPY --from=builder /usr/local/bin/inbound /usr/local/bin/inbound

USER inbound
EXPOSE 8081


ENTRYPOINT ["inbound"]