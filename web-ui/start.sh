#!/bin/bash

# GaussOS Frontend Server Management Script
# Start script for the GaussOS Web Management Interface

set -e  # Exit on any error

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SERVER_PORT=3000
PID_FILE="$SCRIPT_DIR/gaussos-frontend.pid"
LOG_FILE="$SCRIPT_DIR/logs/frontend.log"
LOCK_FILE="$SCRIPT_DIR/gaussos-frontend.lock"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE" 2>/dev/null || echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1" | tee -a "$LOG_FILE" 2>/dev/null || echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1" | tee -a "$LOG_FILE" 2>/dev/null || echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1"
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO:${NC} $1" | tee -a "$LOG_FILE" 2>/dev/null || echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO:${NC} $1"
}

# Check if Deno is installed
check_deno() {
    if ! command -v deno &> /dev/null; then
        error "Deno is not installed. Please install Deno first."
        error "Visit: https://deno.land/#installation"
        exit 1
    fi
    
    DENO_VERSION=$(deno --version | head -n 1 | cut -d' ' -f2)
    log "Deno version: $DENO_VERSION"
}

# Create necessary directories
setup_directories() {
    mkdir -p "$SCRIPT_DIR/logs"
    mkdir -p "$SCRIPT_DIR/pids"
    
    # Update PID file path to pids directory
    PID_FILE="$SCRIPT_DIR/pids/gaussos-frontend.pid"
}

# Check if server is already running
check_running() {
    if [ -f "$PID_FILE" ]; then
        PID=$(cat "$PID_FILE")
        if ps -p "$PID" > /dev/null 2>&1; then
            warn "GaussOS frontend server is already running (PID: $PID)"
            info "Server URL: http://localhost:$SERVER_PORT"
            info "Use './stop.sh' to stop the server or './restart.sh' to restart"
            exit 0
        else
            warn "Stale PID file found. Removing..."
            rm -f "$PID_FILE"
        fi
    fi
    
    if [ -f "$LOCK_FILE" ]; then
        warn "Lock file found. Removing..."
        rm -f "$LOCK_FILE"
    fi
}

# Check port availability
check_port() {
    if lsof -Pi :$SERVER_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
        error "Port $SERVER_PORT is already in use"
        info "Please stop the service using port $SERVER_PORT or change the port in the script"
        exit 1
    fi
}

# Start the server
start_server() {
    log "Starting GaussOS Frontend Server..."
    
    # Change to the web-ui directory
    cd "$SCRIPT_DIR"
    
    # Create lock file
    echo $$ > "$LOCK_FILE"
    
    # Start the server in background
    nohup deno run --allow-net --allow-read --allow-env main.ts > "$LOG_FILE" 2>&1 &
    SERVER_PID=$!
    
    # Save PID
    echo $SERVER_PID > "$PID_FILE"
    
    # Remove lock file
    rm -f "$LOCK_FILE"
    
    log "Server started with PID: $SERVER_PID"
    log "Server URL: http://localhost:$SERVER_PORT"
    log "Log file: $LOG_FILE"
    
    # Wait a moment and check if server started successfully
    sleep 2
    
    if ps -p $SERVER_PID > /dev/null 2>&1; then
        if curl -s http://localhost:$SERVER_PORT > /dev/null 2>&1; then
            log "Server is running successfully!"
            info "You can now access the GaussOS Web Management Interface at:"
            info "  http://localhost:$SERVER_PORT"
            info ""
            info "To stop the server, run: ./stop.sh"
            info "To monitor the server, run: ./monitor.sh"
        else
            warn "Server started but may not be fully ready yet. Please wait a moment and try accessing:"
            warn "  http://localhost:$SERVER_PORT"
        fi
    else
        error "Failed to start server. Check logs: $LOG_FILE"
        rm -f "$PID_FILE"
        exit 1
    fi
}

# Main execution
main() {
    log "=== GaussOS Frontend Server Start Script ==="
    
    # Check prerequisites
    check_deno
    
    # Setup directories
    setup_directories
    
    # Check if already running
    check_running
    
    # Check port availability
    check_port
    
    # Start server
    start_server
    
    log "=== Start script completed ==="
}

# Handle script interruption
cleanup() {
    if [ -f "$LOCK_FILE" ]; then
        rm -f "$LOCK_FILE"
    fi
    log "Start script interrupted"
    exit 1
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

# Run main function
main "$@"
