# ============================================================
# SmartTradeAI C2 Engine - Multi-Stage Docker Build
# ============================================================
# Stage 1: Build Rust binary
# Stage 2: Runtime image
# ============================================================

# --- Stage 1: Rust Builder ---
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY plugins/ ./plugins/
COPY rust/ ./rust/


WORKDIR /build/rust
RUN cargo build --release --bin c2-engine

# --- Stage 2: Runtime ---
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/rust/target/release/c2-engine /usr/local/bin/c2-engine
COPY CLAW.md /workspace/CLAW.md
COPY .claw/settings.json /workspace/.claw/settings.json

WORKDIR /workspace
RUN mkdir -p /workspace/strategies

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["/usr/local/bin/c2-engine"]
