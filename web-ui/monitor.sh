#!/bin/bash

# GaussOS Frontend Server Management Script
# Monitor script for the GaussOS Web Management Interface

set -e  # Exit on any error

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_FILE="$SCRIPT_DIR/pids/gaussos-frontend.pid"
LOG_FILE="$SCRIPT_DIR/logs/frontend.log"
SERVER_PORT=3000
MONITOR_INTERVAL=5  # seconds

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1"
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO:${NC} $1"
}

# Check if server is running
check_server_status() {
    if [ ! -f "$PID_FILE" ]; then
        return 1
    fi
    
    PID=$(cat "$PID_FILE")
    if ! ps -p "$PID" > /dev/null 2>&1; then
        return 1
    fi
    
    return 0
}

# Get server process info
get_process_info() {
    if check_server_status; then
        PID=$(cat "$PID_FILE")
        echo "PID: $PID"
        echo "Command: $(ps -p $PID -o command= 2>/dev/null || echo 'N/A')"
        echo "CPU: $(ps -p $PID -o %cpu= 2>/dev/null || echo 'N/A')%"
        echo "Memory: $(ps -p $PID -o %mem= 2>/dev/null || echo 'N/A')%"
        echo "Uptime: $(ps -p $PID -o etime= 2>/dev/null || echo 'N/A')"
    else
        echo "Server is not running"
    fi
}

# Check server connectivity
check_connectivity() {
    if curl -s --max-time 5 http://localhost:$SERVER_PORT > /dev/null 2>&1; then
        echo "✅ Server is responding on port $SERVER_PORT"
        return 0
    else
        echo "❌ Server is not responding on port $SERVER_PORT"
        return 1
    fi
}

# Get system resources
get_system_resources() {
    echo "=== System Resources ==="
    echo "CPU Usage: $(top -l 1 | grep "CPU usage" | awk '{print $3}' | sed 's/%//')%"
    echo "Memory Usage: $(top -l 1 | grep "PhysMem" | awk '{print $2}' | sed 's/[^0-9]//g')%"
    echo "Disk Usage: $(df -h / | tail -1 | awk '{print $5}')"
    echo "Load Average: $(uptime | awk -F'load average:' '{print $2}' | sed 's/,//g')"
}

# Get network connections
get_network_info() {
    echo "=== Network Connections ==="
    local connections=$(lsof -i :$SERVER_PORT 2>/dev/null | wc -l)
    echo "Active connections on port $SERVER_PORT: $connections"
    
    if command -v netstat > /dev/null 2>&1; then
        echo "Network status:"
        netstat -an | grep ":$SERVER_PORT" | head -5
    fi
}

# Get log file info
get_log_info() {
    if [ -f "$LOG_FILE" ]; then
        echo "=== Log File Info ==="
        echo "Log file: $LOG_FILE"
        echo "Size: $(du -h "$LOG_FILE" | cut -f1)"
        echo "Last modified: $(stat -f "%Sm" "$LOG_FILE" 2>/dev/null || stat -c "%y" "$LOG_FILE" 2>/dev/null || echo 'N/A')"
        echo "Last 5 log entries:"
        tail -5 "$LOG_FILE" 2>/dev/null || echo "No log entries found"
    else
        echo "Log file not found: $LOG_FILE"
    fi
}

# Display status dashboard
show_dashboard() {
    clear
    echo -e "${CYAN}========================================${NC}"
    echo -e "${CYAN}    GaussOS Frontend Server Monitor    ${NC}"
    echo -e "${CYAN}========================================${NC}"
    echo "Timestamp: $(date)"
    echo ""
    
    # Server status
    echo -e "${BLUE}=== Server Status ===${NC}"
    if check_server_status; then
        echo -e "${GREEN}✅ Server is running${NC}"
        get_process_info
    else
        echo -e "${RED}❌ Server is not running${NC}"
    fi
    echo ""
    
    # Connectivity
    echo -e "${BLUE}=== Connectivity ===${NC}"
    if check_connectivity; then
        echo -e "${GREEN}✅ Server is accessible${NC}"
    else
        echo -e "${RED}❌ Server is not accessible${NC}"
    fi
    echo ""
    
    # System resources
    get_system_resources
    echo ""
    
    # Network info
    get_network_info
    echo ""
    
    # Log info
    get_log_info
    echo ""
    
    echo -e "${CYAN}========================================${NC}"
    echo "Press Ctrl+C to exit monitoring"
    echo "Refresh interval: ${MONITOR_INTERVAL}s"
    echo -e "${CYAN}========================================${NC}"
}

# Continuous monitoring
monitor_continuous() {
    while true; do
        show_dashboard
        sleep $MONITOR_INTERVAL
    done
}

# One-time status check
status_check() {
    echo -e "${CYAN}=== GaussOS Frontend Server Status ===${NC}"
    echo "Timestamp: $(date)"
    echo ""
    
    # Server status
    echo -e "${BLUE}Server Status:${NC}"
    if check_server_status; then
        echo -e "${GREEN}✅ Running${NC}"
        get_process_info
    else
        echo -e "${RED}❌ Not running${NC}"
    fi
    echo ""
    
    # Connectivity
    echo -e "${BLUE}Connectivity:${NC}"
    check_connectivity
    echo ""
    
    # Quick system info
    echo -e "${BLUE}System Info:${NC}"
    echo "CPU: $(top -l 1 | grep "CPU usage" | awk '{print $3}' | sed 's/%//')%"
    echo "Memory: $(top -l 1 | grep "PhysMem" | awk '{print $2}' | sed 's/[^0-9]//g')%"
    echo "Uptime: $(uptime | awk '{print $3, $4}' | sed 's/,//')"
}

# Show help
show_help() {
    echo "GaussOS Frontend Server Monitor"
    echo ""
    echo "Usage: $0 [OPTION]"
    echo ""
    echo "Options:"
    echo "  -c, --continuous    Continuous monitoring with auto-refresh"
    echo "  -s, --status        One-time status check"
    echo "  -h, --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                  One-time status check (default)"
    echo "  $0 -c               Continuous monitoring"
    echo "  $0 --status         One-time status check"
    echo ""
    echo "Monitoring includes:"
    echo "  - Server process status"
    echo "  - Connectivity check"
    echo "  - System resources"
    echo "  - Network connections"
    echo "  - Log file information"
}

# Handle script interruption
cleanup() {
    echo ""
    log "Monitoring stopped"
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

# Main execution
main() {
    case "${1:-}" in
        -c|--continuous)
            monitor_continuous
            ;;
        -s|--status)
            status_check
            ;;
        -h|--help)
            show_help
            ;;
        "")
            status_check
            ;;
        *)
            error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
