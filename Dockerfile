# Build stage - uses a heavy image to compile dependencies
FROM rust:1.80-slim-bookworm AS builder

# Optimized build: install only necessary build-essential tools
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

# Build all workspace binaries in release mode
RUN cargo build --release --workspace

# Strip binaries to minimize size
RUN strip target/release/omnicontext && \
    strip target/release/omnicontext-mcp && \
    strip target/release/omnicontext-daemon

# Runtime stage - final minimal image
FROM debian:bookworm-slim

# Install runtime dependencies (git for code analysis, ca-certificates for downloads)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -s /bin/bash omni

# Copy binaries from builder
COPY --from=builder /build/target/release/omnicontext /usr/local/bin/
COPY --from=builder /build/target/release/omnicontext-mcp /usr/local/bin/
COPY --from=builder /build/target/release/omnicontext-daemon /usr/local/bin/

USER omni
WORKDIR /home/omni

# Create directory for mounting repositories
RUN mkdir /home/omni/repo

# Default: run the MCP server on stdio
ENTRYPOINT ["omnicontext-mcp"]
CMD ["--repo", "/home/omni/repo"]

LABEL org.opencontainers.image.source="https://github.com/steeltroops-ai/omnicontext"
LABEL org.opencontainers.image.description="OmniContext - High-performance code context engine for AI agents"
LABEL org.opencontainers.image.licenses="Apache-2.0"

EXPOSE 9090
