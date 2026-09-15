FROM rust:1.92-bookworm AS builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./

RUN mkdir -p src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --locked --release \
    && rm -rf src

# Build the agent
COPY src ./src
RUN cargo build --locked --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/monitor-agent /usr/local/bin/monitor-agent

# Create config directory
RUN mkdir -p /etc/monitor-agent

# Default environment variables
ENV AGENT_ID=docker-agent
ENV AGENT_NAME=docker-agent
ENV SERVER_URL=http://host.docker.internal:8080
ENV COLLECTION_INTERVAL=15
ENV PROCESS_WATCH_ENABLED=false
ENV PROCESS_WATCH_NAMES=""
ENV SERVICE_WATCH_ENABLED=true
ENV SERVICE_WATCH_NAMES="watchtower-agent,docker,ssh"
ENV SERVICE_WATCH_INTERVAL_SECONDS=15
ENV DOCKER_MONITOR_ENABLED=false

# Volume for configuration
VOLUME ["/etc/monitor-agent"]

# Run from environment defaults. Mount a config and override the command with
# `monitor-agent -c /etc/monitor-agent/agent.toml` when file-based config is desired.
CMD ["monitor-agent"]
