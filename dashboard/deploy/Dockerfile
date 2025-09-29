# Multi-stage build for Rust application
FROM rust:latest AS builder

# Install musl-tools for static linking
RUN apt-get update && apt-get install -y musl-tools

# Add the musl target
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app

# Copy workspace files (create minimal workspace for dashboard only)
COPY dashboard/Cargo.toml ./dashboard/
COPY common/ ./common/

# Create a minimal workspace Cargo.toml for dashboard build
RUN echo '[workspace]\nmembers = ["dashboard", "common"]\nresolver = "3"' > Cargo.toml

# Copy source code for dashboard only
COPY dashboard/src/ ./dashboard/src/

# Build the application
RUN cargo build --release --target x86_64-unknown-linux-musl --package data-viewer

# Runtime stage with minimal image
FROM alpine:3.18

# Install ca-certificates for HTTPS and netcat for health checking
RUN apk --no-cache add ca-certificates netcat-openbsd curl bash

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/data-viewer /app/data-viewer

# Copy configuration file
COPY dashboard/config.json /app/config.json

# Copy frontend assets if they exist
COPY --from=builder /app/dashboard/src/frontend/ /app/frontend/

# Download Cloud SQL Proxy
RUN curl -o cloud-sql-proxy https://storage.googleapis.com/cloud-sql-connectors/cloud-sql-proxy/v2.18.2/cloud-sql-proxy.linux.amd64 && \
    chmod +x cloud-sql-proxy

# Create a non-root user
RUN addgroup -g 1001 -S appgroup && \
    adduser -S appuser -G appgroup

COPY .tmp/service-client.json /app/.gcp/service-client.json

# Copy startup scripts and set permissions
COPY dashboard/deploy/startup.sh /app/startup.sh
RUN chmod +x /app/startup.sh && \
    chown appuser:appgroup /app/startup.sh
ENV GOOGLE_APPLICATION_CREDENTIALS="/app/.gcp/service-client.json"

USER appuser


# Expose the port (adjust if your app uses a different port)
EXPOSE 8080

# Run the startup script
CMD ["/bin/bash", "./startup.sh"]
