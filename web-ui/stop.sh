#!/bin/bash

# GaussOS Frontend Server Management Script
# Stop script for the GaussOS Web Management Interface

set -e  # Exit on any error

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_FILE="$SCRIPT_DIR/pids/gaussos-frontend.pid"
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

# Check if server is running
check_running() {
    if [ ! -f "$PID_FILE" ]; then
        warn "No PID file found. Server may not be running."
        return 1
    fi
    
    PID=$(cat "$PID_FILE")
    if ! ps -p "$PID" > /dev/null 2>&1; then
        warn "Server process (PID: $PID) not found. Removing stale PID file."
        rm -f "$PID_FILE"
        return 1
    fi
    
    return 0
}

# Stop the server gracefully
stop_server() {
    if ! check_running; then
        info "Server is not running."
        return 0
    fi
    
    PID=$(cat "$PID_FILE")
    log "Stopping GaussOS Frontend Server (PID: $PID)..."
    
    # Try graceful shutdown first
    if kill -TERM "$PID" 2>/dev/null; then
        log "Sent SIGTERM to process $PID"
        
        # Wait for graceful shutdown (up to 30 seconds)
        local count=0
        while [ $count -lt 30 ]; do
            if ! ps -p "$PID" > /dev/null 2>&1; then
                log "Server stopped gracefully"
                rm -f "$PID_FILE"
                return 0
            fi
            sleep 1
            count=$((count + 1))
        done
        
        # Force kill if still running
        warn "Server did not stop gracefully. Force killing..."
        if kill -KILL "$PID" 2>/dev/null; then
            log "Server force stopped"
            rm -f "$PID_FILE"
        else
            error "Failed to force stop server"
            return 1
        fi
    else
        error "Failed to send SIGTERM to process $PID"
        return 1
    fi
}

# Clean up lock files
cleanup() {
    if [ -f "$LOCK_FILE" ]; then
        log "Removing lock file..."
        rm -f "$LOCK_FILE"
    fi
}

# Check if any Deno processes are still running
check_deno_processes() {
    local deno_pids=$(pgrep -f "deno.*main.ts" 2>/dev/null || true)
    if [ -n "$deno_pids" ]; then
        warn "Found remaining Deno processes: $deno_pids"
        read -p "Do you want to kill these processes? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "$deno_pids" | xargs kill -KILL 2>/dev/null || true
            log "Killed remaining Deno processes"
        fi
    fi
}

# Main execution
main() {
    log "=== GaussOS Frontend Server Stop Script ==="
    
    # Stop server
    if stop_server; then
        log "Server stopped successfully"
    else
        error "Failed to stop server"
        exit 1
    fi
    
    # Clean up
    cleanup
    
    # Check for remaining processes
    check_deno_processes
    
    log "=== Stop script completed ==="
}

# Handle script interruption
interrupt() {
    log "Stop script interrupted"
    exit 1
}

# Set up signal handlers
trap interrupt SIGINT SIGTERM

# Run main function
main "$@"
