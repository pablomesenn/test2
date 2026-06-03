# Stage 1: build
FROM rust:1.88-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs

COPY src ./src
RUN touch src/main.rs && cargo build --release

# Stage 2: imagen final
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/zonal-cache .

RUN mkdir -p /app/cache_data

EXPOSE 8080

CMD ["./zonal-cache"]
