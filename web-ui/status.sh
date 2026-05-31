#!/bin/bash

# GaussOS Frontend Server Management Script
# Quick status check script

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_FILE="$SCRIPT_DIR/pids/gaussos-frontend.pid"
SERVER_PORT=3000

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Check if server is running
check_running() {
    if [ ! -f "$PID_FILE" ]; then
        return 1
    fi
    
    PID=$(cat "$PID_FILE")
    if ! ps -p "$PID" > /dev/null 2>&1; then
        return 1
    fi
    
    return 0
}

# Check connectivity
check_connectivity() {
    if curl -s --max-time 3 http://localhost:$SERVER_PORT > /dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Main status check
main() {
    echo -e "${BLUE}GaussOS Frontend Server Status${NC}"
    echo "=================================="
    
    # Check if running
    if check_running; then
        PID=$(cat "$PID_FILE")
        echo -e "${GREEN}✅ Server is running (PID: $PID)${NC}"
        
        # Check connectivity
        if check_connectivity; then
            echo -e "${GREEN}✅ Server is accessible at http://localhost:$SERVER_PORT${NC}"
        else
            echo -e "${YELLOW}⚠️  Server is running but not responding${NC}"
        fi
    else
        echo -e "${RED}❌ Server is not running${NC}"
        echo "Use './start.sh' to start the server"
    fi
    
    echo ""
    echo "Quick Commands:"
    echo "  ./start.sh    - Start the server"
    echo "  ./stop.sh     - Stop the server"
    echo "  ./restart.sh  - Restart the server"
    echo "  ./monitor.sh  - Monitor the server"
}

main "$@"
