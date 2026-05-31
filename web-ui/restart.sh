#!/bin/bash

# GaussOS Frontend Server Management Script
# Restart script for the GaussOS Web Management Interface

set -e  # Exit on any error

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_FILE="$SCRIPT_DIR/pids/gaussos-frontend.pid"
LOG_FILE="$SCRIPT_DIR/logs/frontend.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1" | tee -a "$LOG_FILE"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1" | tee -a "$LOG_FILE"
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO:${NC} $1" | tee -a "$LOG_FILE"
}

# Check if stop script exists
check_scripts() {
    if [ ! -f "$SCRIPT_DIR/stop.sh" ]; then
        error "Stop script not found: $SCRIPT_DIR/stop.sh"
        exit 1
    fi
    
    if [ ! -f "$SCRIPT_DIR/start.sh" ]; then
        error "Start script not found: $SCRIPT_DIR/start.sh"
        exit 1
    fi
}

# Stop the server
stop_server() {
    log "Stopping server..."
    if "$SCRIPT_DIR/stop.sh"; then
        log "Server stopped successfully"
        return 0
    else
        error "Failed to stop server"
        return 1
    fi
}

# Start the server
start_server() {
    log "Starting server..."
    if "$SCRIPT_DIR/start.sh"; then
        log "Server started successfully"
        return 0
    else
        error "Failed to start server"
        return 1
    fi
}

# Wait for server to be ready
wait_for_server() {
    local max_attempts=30
    local attempt=0
    local server_port=3000
    
    log "Waiting for server to be ready..."
    
    while [ $attempt -lt $max_attempts ]; do
        if curl -s http://localhost:$server_port > /dev/null 2>&1; then
            log "Server is ready!"
            return 0
        fi
        
        attempt=$((attempt + 1))
        sleep 1
    done
    
    warn "Server may not be fully ready yet"
    return 1
}

# Main execution
main() {
    log "=== GaussOS Frontend Server Restart Script ==="
    
    # Check if required scripts exist
    check_scripts
    
    # Stop server
    if stop_server; then
        info "Server stopped successfully"
    else
        error "Failed to stop server during restart"
        exit 1
    fi
    
    # Wait a moment before starting
    sleep 2
    
    # Start server
    if start_server; then
        info "Server started successfully"
    else
        error "Failed to start server during restart"
        exit 1
    fi
    
    # Wait for server to be ready
    wait_for_server
    
    log "=== Restart script completed ==="
    info "Server has been restarted successfully"
    info "Access the interface at: http://localhost:3000"
}

# Handle script interruption
interrupt() {
    log "Restart script interrupted"
    exit 1
}

# Set up signal handlers
trap interrupt SIGINT SIGTERM

# Run main function
main "$@"
