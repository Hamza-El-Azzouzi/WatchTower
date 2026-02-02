FROM rust:1.83-slim AS builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the agent
RUN cargo build --release

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

# Volume for configuration
VOLUME ["/etc/monitor-agent"]

# Run the agent
CMD ["monitor-agent", "-c", "/etc/monitor-agent/agent.toml"]
