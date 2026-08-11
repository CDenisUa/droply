# syntax=docker/dockerfile:1
FROM rust:1-slim-bookworm AS build
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps ./apps
COPY migrations ./migrations

RUN cargo build --release -p droply-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/droply-api /usr/local/bin/droply-api

EXPOSE 8080
CMD ["/usr/local/bin/droply-api"]
