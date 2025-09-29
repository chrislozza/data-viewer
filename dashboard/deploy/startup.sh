#!/bin/bash

# Startup script for Data Viewer Dashboard
# Starts Cloud SQL Proxy first, then the Rust application

set -e

echo "Starting Data Viewer Dashboard..."

# Check if Cloud SQL connection string is provided
if [ -n "$CLOUD_SQL_CONNECTION_STRING" ]; then
    echo "Starting Cloud SQL Proxy..."
    
    # Set default port if not provided
    CLOUD_SQL_PORT=${CLOUD_SQL_PORT:-5432}
    
    # Build cloud-sql-proxy command
    PROXY_CMD="./cloud-sql-proxy $CLOUD_SQL_CONNECTION_STRING -p=$CLOUD_SQL_PORT"
    
    # Add credentials if provided
    if [ -n "$GOOGLE_APPLICATION_CREDENTIALS" ]; then
        echo "Using credentials from: $GOOGLE_APPLICATION_CREDENTIALS"
        PROXY_CMD="$PROXY_CMD -c=$GOOGLE_APPLICATION_CREDENTIALS"
    fi
    
    # Redirect output to suppress logs
    PROXY_CMD="$PROXY_CMD > /dev/null 2>&1"
    
    # Start Cloud SQL Proxy in the background
    eval "$PROXY_CMD &"
    
    PROXY_PID=$!
    echo "Cloud SQL Proxy started with PID: $PROXY_PID on port $CLOUD_SQL_PORT"
    
    # Wait for proxy to be ready (max 30 seconds)
    echo "Waiting for Cloud SQL Proxy to be ready..."
    for i in {1..30}; do
        if nc -z localhost $CLOUD_SQL_PORT 2>/dev/null; then
            echo "Cloud SQL Proxy is ready on port $CLOUD_SQL_PORT!"
            break
        fi
        if [ $i -eq 30 ]; then
            echo "Cloud SQL Proxy failed to start within 30 seconds"
            kill $PROXY_PID 2>/dev/null || true
            exit 1
        fi
        sleep 1
    done
else
    echo "No CLOUD_SQL_CONNECTION_STRING provided, skipping Cloud SQL Proxy"
fi

# Start the Rust application
echo "Starting Data Viewer application..."
exec ./data-viewer --settings config.json
