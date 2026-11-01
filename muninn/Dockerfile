FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p muninn

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 10001 muninn
COPY --from=builder /app/target/release/muninn /usr/local/bin/muninn
USER muninn
EXPOSE 8080
ENTRYPOINT ["muninn"]