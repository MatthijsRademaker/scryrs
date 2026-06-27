# scryrs-server — containerized central trace ingest service
#
# Build:
#   docker build -t scryrs-server .
#
# Run standalone:
#   docker run -p 8081:8081 -v scryrs-data:/data/scryrs scryrs-server
#
# Or use the provided docker-compose.yml for multi-agent networking.

FROM rust:1.85.0 AS builder

WORKDIR /build
COPY . .

# Build scryrs-cli with server + core features in release mode.
RUN cargo build -p scryrs-cli --features server,core --release

# Minimal runtime image.
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/scryrs /usr/local/bin/scryrs

# Server-owned data directory.
RUN mkdir -p /data/scryrs
VOLUME /data/scryrs

# Default server runtime: bind all interfaces, use the documented port and store path.
EXPOSE 8081
ENTRYPOINT ["scryrs", "server", "--bind", "0.0.0.0", "--port", "8081", "--store", "/data/scryrs/server.db"]
