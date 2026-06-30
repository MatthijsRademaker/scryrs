# scryrs-server — containerized central trace ingest service + live dashboard
#
# Repository packaging / maintainer asset. This Dockerfile is the SOURCE of the
# published image; consumers do NOT build it. Released versions are published by
# the release workflow to:
#   ghcr.io/matthijsrademaker/scryrs-server:<version>  (and :latest, linux/amd64)
#
# Consumer workspaces use `scryrs init --mode live` + `scryrs up`, which scaffold
# `.scryrs/compose.yml` referencing the published ghcr.io image — no local build.
#
# Maintainer/dev build:
#   docker build -t scryrs-server .
#
# Run the published image standalone:
#   docker run -p 8081:8081 -v scryrs-data:/data/scryrs \
#     ghcr.io/matthijsrademaker/scryrs-server:latest
#
# Or use repository-root docker-compose.yml for maintainer/dev smoke workflows.

FROM rust:1.85.0 AS builder

WORKDIR /build

# The scryrs-dashboard crate's build.rs compiles the frontend with Bun, so the
# JS toolchain must be present in the builder. Mirrors the CI toolchain.
RUN apt-get update && apt-get install -y --no-install-recommends curl unzip \
    && rm -rf /var/lib/apt/lists/* \
    && curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:${PATH}"

COPY . .

# Build scryrs-cli with server, core, and dashboard features in release mode.
RUN cargo build -p scryrs-cli --features server,core,dashboard --release

# Minimal runtime image.
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/scryrs /usr/local/bin/scryrs

# Server-owned data directory.
RUN mkdir -p /data/scryrs
VOLUME /data/scryrs

# Default server runtime: bind all interfaces, use the documented port and store path.
EXPOSE 8081
ENTRYPOINT ["scryrs", "server", "--bind", "0.0.0.0", "--port", "8081", "--store", "/data/scryrs/server.db"]
