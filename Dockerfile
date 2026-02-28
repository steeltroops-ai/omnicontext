# OmniContext Enterprise - Multi-stage Docker Build
# Produces minimal runtime image (~50MB) with both CLI and MCP binaries.

# ---------------------------------------------------------------------------
# Stage 1: Build
# ---------------------------------------------------------------------------
FROM rust:1.80-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

RUN cargo build --release --workspace \
    && strip target/release/omnicontext \
    && strip target/release/omnicontext-mcp

# ---------------------------------------------------------------------------
# Stage 2: Runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -s /bin/bash omni

COPY --from=builder /build/target/release/omnicontext /usr/local/bin/
COPY --from=builder /build/target/release/omnicontext-mcp /usr/local/bin/

USER omni
WORKDIR /home/omni

# Default: run the MCP server on stdio
ENTRYPOINT ["omnicontext-mcp"]
CMD ["--repo", "/repo"]

# For REST API server, override:
# docker run -p 9090:9090 -v /path/to/repo:/repo omnicontext \
#   omnicontext serve --addr 0.0.0.0 --port 9090

LABEL org.opencontainers.image.source="https://github.com/steeltroops-ai/omnicontext"
LABEL org.opencontainers.image.description="OmniContext - Universal code context engine for AI agents"
LABEL org.opencontainers.image.licenses="Apache-2.0"

EXPOSE 9090
