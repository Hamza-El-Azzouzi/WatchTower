# Multi-stage build for DevOps Monitoring Server
# Stage 1: Build the Rust application
FROM rust:1.92-bookworm AS builder

# Install build dependencies (including PostgreSQL client libraries)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty project
WORKDIR /app

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./

# Build dependencies against a temporary target. The previous empty-project
# command failed before compiling anything, which made every ARM64 release
# rebuild the full dependency graph under emulation.
RUN mkdir -p src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --locked --release \
    && rm -rf src

# Copy actual source code
COPY src ./src
COPY migrations ./migrations
COPY benches ./benches
COPY config.toml ./config.toml

# Build for release with optimizations
RUN cargo build --locked --release

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies (including PostgreSQL client)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 monitor && \
    mkdir -p /app/data && \
    chown -R monitor:monitor /app

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/monitor-server /app/monitor-server
COPY --from=builder /app/config.toml /app/config.toml

# Copy migrations (needed for runtime schema initialization)
COPY --from=builder /app/migrations /app/migrations

# Set ownership
RUN chown -R monitor:monitor /app

# Switch to non-root user
USER monitor

# Expose the server port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default environment variables (can be overridden)
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV DATABASE_ENABLED=true

# Volume for persistent data
VOLUME ["/app/data"]

# Run the server
CMD ["/app/monitor-server", "-c", "/app/config.toml"]
