# Stage 1: build frontend
FROM node:22-alpine AS frontend
WORKDIR /app
COPY web/package*.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# Stage 2: build daemon
FROM rust:1.88-slim AS builder
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libzmq3-dev cmake git ca-certificates gcc g++ \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependency compilation separately from source changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs \
    && cargo build --release 2>/dev/null; rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release

# Stage 3: minimal runtime image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libssl3 libzmq5 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/marketd /usr/local/bin/marketd
COPY --from=frontend /app/dist /usr/share/marketd/web

ENV MARKETD_LISTEN_ADDR=0.0.0.0:3000
ENV MARKETD_STATIC_DIR=/usr/share/marketd/web
EXPOSE 3000

ENTRYPOINT ["marketd"]
