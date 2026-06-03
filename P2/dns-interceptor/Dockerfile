# Stage 1: build
FROM rust:1.88-slim AS builder

WORKDIR /app

# Copiar manifiestos primero para aprovechar cache de capas
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs

# Copiar código fuente real y compilar
COPY src ./src
RUN touch src/main.rs && cargo build --release

# Stage 2: imagen final mínima
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/dns-interceptor .

# La base de datos GeoIP se monta como volumen en Kubernetes
RUN mkdir -p /app/geoip

EXPOSE 53/udp

CMD ["./dns-interceptor"]
