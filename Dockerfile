FROM rust:bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY node ./node
COPY cli ./cli
COPY desktop ./desktop

RUN apt-get update && apt-get install -y \
    cmake \
    libopus-dev \
    pkg-config \
    libasound2-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release -p agora-node

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libopus0 \
    libasound2 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false agora

COPY --from=builder /app/target/release/agora-node /usr/local/bin/agora-node

RUN mkdir -p /etc/agora /var/lib/agora /var/log/agora && \
    chown -R agora:agora /var/lib/agora /var/log/agora

COPY docker/node.toml /etc/agora/node.toml

EXPOSE 7001/tcp 8080/tcp 9090/tcp

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

USER agora

ENTRYPOINT ["agora-node"]
CMD ["start", "--config", "/etc/agora/node.toml"]
