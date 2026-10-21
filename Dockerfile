# ==============================================================================
# BOS (Business Operating System) - Production Multi-Stage Container Image
# Architecture: Svelte 5 Frontend + Rust Axum Core Engine + QuickJS Sandbox
# ==============================================================================

# ------------------------------------------------------------------------------
# Stage 1: Build Svelte 5 Frontend UI
# ------------------------------------------------------------------------------
FROM node:22-alpine AS ui-builder
WORKDIR /app/ui

# Install dependencies with lockfile fidelity
COPY ui/package.json ui/package-lock.json ./
RUN npm ci --prefer-offline --no-audit

# Copy UI source files and build
COPY ui/ ./
RUN npm run build

# ------------------------------------------------------------------------------
# Stage 2: Build Rust Backend Binaries
# ------------------------------------------------------------------------------
FROM rust:1.85-bookworm AS rust-builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace cargo files
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY packages/ packages/

# Build optimized release binaries
RUN cargo build --release --bin bosc

# ------------------------------------------------------------------------------
# Stage 3: Ultra-Lean Minimal Production Runtime
# ------------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install production runtime dependencies & security tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    postgresql-client \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# Set timezone to Asia/Kolkata (Indian Standard Time: UTC+05:30)
ENV TZ=Asia/Kolkata
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

# Create non-root system user for security
RUN groupadd -g 10001 bos && \
    useradd -u 10001 -g bos -s /bin/bash -m bos

WORKDIR /home/bos/app

# Copy compiled Rust binary
COPY --from=rust-builder /app/target/release/bosc /usr/local/bin/bosc

# Copy compiled Svelte 5 UI assets
COPY --from=ui-builder /app/ui/dist /home/bos/app/ui-dist

# Copy core meta-object packages
COPY --chown=bos:bos packages/ /home/bos/app/packages/

# Set up storage and log directories
RUN mkdir -p /home/bos/app/data /home/bos/app/logs && \
    chown -R bos:bos /home/bos/app

USER bos:bos

# Expose standard BOS HTTP API server port
EXPOSE 3000

# Default environment configuration
ENV PORT=3000 \
    DATABASE_URL=postgres://postgres:postgres@localhost:5432/bos_dev \
    RUST_LOG=info

# Health check probe against live Axum endpoint
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3000/api/health || exit 1

ENTRYPOINT ["/usr/local/bin/bosc"]
CMD ["serve", "--port", "3000"]
