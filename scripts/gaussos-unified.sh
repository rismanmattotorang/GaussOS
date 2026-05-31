#!/bin/bash

# GaussOS Unified Service Manager
# Manages both frontend and backend services

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
WEB_UI_DIR="$PROJECT_ROOT/web-ui"
PID_DIR="$PROJECT_ROOT/pids"
LOG_DIR="$PROJECT_ROOT/logs"

# Service configs
BACKEND_PORT=8080
FRONTEND_PORT=3000
BACKEND_PID_FILE="$PID_DIR/gaussos-backend.pid"
FRONTEND_PID_FILE="$PID_DIR/gaussos-frontend.pid"

# Create directories
mkdir -p "$PID_DIR" "$LOG_DIR"

# Helper functions
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1"
}

# Check if service is running
is_service_running() {
    local pid_file=$1
    local port=$2
    
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if ps -p "$pid" > /dev/null 2>&1 && lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
            return 0
        else
            rm -f "$pid_file"
        fi
    fi
    return 1
}

# Start backend service
start_backend() {
    log "Starting backend service..."
    
    if is_service_running "$BACKEND_PID_FILE" "$BACKEND_PORT"; then
        warn "Backend is already running"
        return 0
    fi
    
    # Check if backend binary exists
    local backend_bin="./target/release/gaussos"
    if [ ! -f "$backend_bin" ]; then
        backend_bin="./target/debug/gaussos"
    fi
    
    if [ ! -f "$backend_bin" ]; then
        error "Backend binary not found. Run: cargo build --release"
        return 1
    fi
    
    # Start backend
    cd "$PROJECT_ROOT"
    nohup "$backend_bin" server --host 0.0.0.0 --port "$BACKEND_PORT" > "$LOG_DIR/backend.log" 2>&1 &
    echo $! > "$BACKEND_PID_FILE"
    
    # Wait for backend to start
    local attempts=0
    while [ $attempts -lt 30 ]; do
        if curl -s "http://localhost:$BACKEND_PORT/health" >/dev/null 2>&1; then
            log "Backend started successfully"
            return 0
        fi
        sleep 1
        attempts=$((attempts + 1))
    done
    
    error "Backend failed to start"
    return 1
}

# Start frontend service
start_frontend() {
    log "Starting frontend service..."
    
    if is_service_running "$FRONTEND_PID_FILE" "$FRONTEND_PORT"; then
        warn "Frontend is already running"
        return 0
    fi
    
    # Check if Deno is installed
    if ! command -v deno &> /dev/null; then
        error "Deno is not installed"
        return 1
    fi
    
    # Start frontend
    cd "$WEB_UI_DIR"
    nohup deno run --allow-net --allow-read --allow-env main.ts > "$LOG_DIR/frontend.log" 2>&1 &
    echo $! > "$FRONTEND_PID_FILE"
    cd "$PROJECT_ROOT"
    
    # Wait for frontend to start
    local attempts=0
    while [ $attempts -lt 30 ]; do
        if curl -s "http://localhost:$FRONTEND_PORT" >/dev/null 2>&1; then
            log "Frontend started successfully"
            return 0
        fi
        sleep 1
        attempts=$((attempts + 1))
    done
    
    error "Frontend failed to start"
    return 1
}

# Stop service
stop_service() {
    local service_name=$1
    local pid_file=$2
    local port=$3
    
    if [ ! -f "$pid_file" ]; then
        warn "$service_name is not running"
        return 0
    fi
    
    local pid=$(cat "$pid_file")
    log "Stopping $service_name (PID: $pid)..."
    
    # Try graceful shutdown
    if kill -TERM "$pid" 2>/dev/null; then
        local attempts=0
        while [ $attempts -lt 30 ]; do
            if ! ps -p "$pid" > /dev/null 2>&1; then
                log "$service_name stopped gracefully"
                rm -f "$pid_file"
                return 0
            fi
            sleep 1
            attempts=$((attempts + 1))
        done
        
        # Force kill
        warn "$service_name did not stop gracefully. Force killing..."
        kill -KILL "$pid" 2>/dev/null || true
        rm -f "$pid_file"
        log "$service_name force stopped"
    else
        error "Failed to stop $service_name"
        return 1
    fi
}

# Stop backend
stop_backend() {
    stop_service "Backend" "$BACKEND_PID_FILE" "$BACKEND_PORT"
}

# Stop frontend
stop_frontend() {
    stop_service "Frontend" "$FRONTEND_PID_FILE" "$FRONTEND_PORT"
}

# Show status
show_status() {
    echo -e "${CYAN}=== GaussOS Service Status ===${NC}"
    echo "Timestamp: $(date)"
    echo ""
    
    # Backend status
    echo -e "${BLUE}Backend (Port $BACKEND_PORT):${NC}"
    if is_service_running "$BACKEND_PID_FILE" "$BACKEND_PORT"; then
        local backend_pid=$(cat "$BACKEND_PID_FILE")
        echo -e "  ${GREEN}✅ Running (PID: $backend_pid)${NC}"
        
        if curl -s "http://localhost:$BACKEND_PORT/health" >/dev/null 2>&1; then
            echo -e "  ${GREEN}✅ Health check passed${NC}"
        else
            echo -e "  ${YELLOW}⚠️  Health check failed${NC}"
        fi
    else
        echo -e "  ${RED}❌ Not running${NC}"
    fi
    echo ""
    
    # Frontend status
    echo -e "${BLUE}Frontend (Port $FRONTEND_PORT):${NC}"
    if is_service_running "$FRONTEND_PID_FILE" "$FRONTEND_PORT"; then
        local frontend_pid=$(cat "$FRONTEND_PID_FILE")
        echo -e "  ${GREEN}✅ Running (PID: $frontend_pid)${NC}"
        
        if curl -s "http://localhost:$FRONTEND_PORT" >/dev/null 2>&1; then
            echo -e "  ${GREEN}✅ Accessible${NC}"
        else
            echo -e "  ${YELLOW}⚠️  Not accessible${NC}"
        fi
    else
        echo -e "  ${RED}❌ Not running${NC}"
    fi
    echo ""
    
    # URLs
    echo -e "${BLUE}Service URLs:${NC}"
    echo "  Backend API: http://localhost:$BACKEND_PORT"
    echo "  Frontend UI: http://localhost:$FRONTEND_PORT"
    echo "  Health Check: http://localhost:$BACKEND_PORT/health"
    echo ""
}

# Check health
check_health() {
    echo -e "${CYAN}=== Health Check ===${NC}"
    echo ""
    
    local all_healthy=true
    
    # Backend health
    echo -e "${BLUE}Backend Health:${NC}"
    if curl -s "http://localhost:$BACKEND_PORT/health" >/dev/null 2>&1; then
        echo -e "  ${GREEN}✅ Healthy${NC}"
    else
        echo -e "  ${RED}❌ Unhealthy${NC}"
        all_healthy=false
    fi
    
    # Frontend health
    echo -e "${BLUE}Frontend Health:${NC}"
    if curl -s "http://localhost:$FRONTEND_PORT" >/dev/null 2>&1; then
        echo -e "  ${GREEN}✅ Accessible${NC}"
    else
        echo -e "  ${RED}❌ Not accessible${NC}"
        all_healthy=false
    fi
    
    echo ""
    if $all_healthy; then
        echo -e "${GREEN}✅ All services are healthy${NC}"
    else
        echo -e "${RED}❌ Some services are unhealthy${NC}"
    fi
}

# Show help
show_help() {
    echo "GaussOS Unified Service Manager"
    echo ""
    echo "Usage: $0 [COMMAND] [SERVICE]"
    echo ""
    echo "Commands:"
    echo "  start [service]     Start services (backend, frontend, all)"
    echo "  stop [service]      Stop services (backend, frontend, all)"
    echo "  restart [service]   Restart services (backend, frontend, all)"
    echo "  status              Show service status"
    echo "  health              Check service health"
    echo "  help                Show this help"
    echo ""
    echo "Services:"
    echo "  backend             Backend API service"
    echo "  frontend            Frontend UI service"
    echo "  all                 Both services (default)"
    echo ""
    echo "Examples:"
    echo "  $0 start            Start both services"
    echo "  $0 start backend    Start only backend"
    echo "  $0 stop frontend    Stop only frontend"
    echo "  $0 status           Show status"
    echo "  $0 health           Check health"
}

# Main execution
main() {
    local command=${1:-"help"}
    local service=${2:-"all"}
    
    case $command in
        "start")
            case $service in
                "backend"|"b")
                    start_backend
                    ;;
                "frontend"|"f")
                    start_backend && start_frontend
                    ;;
                "all"|"")
                    start_backend && start_frontend
                    ;;
                *)
                    error "Invalid service: $service"
                    show_help
                    exit 1
                    ;;
            esac
            ;;
        "stop")
            case $service in
                "backend"|"b")
                    stop_backend
                    ;;
                "frontend"|"f")
                    stop_frontend
                    ;;
                "all"|"")
                    stop_frontend && stop_backend
                    ;;
                *)
                    error "Invalid service: $service"
                    show_help
                    exit 1
                    ;;
            esac
            ;;
        "restart")
            case $service in
                "backend"|"b")
                    stop_backend && sleep 2 && start_backend
                    ;;
                "frontend"|"f")
                    stop_frontend && sleep 2 && start_backend && start_frontend
                    ;;
                "all"|"")
                    stop_frontend && stop_backend && sleep 2 && start_backend && start_frontend
                    ;;
                *)
                    error "Invalid service: $service"
                    show_help
                    exit 1
                    ;;
            esac
            ;;
        "status")
            show_status
            ;;
        "health")
            check_health
            ;;
        "help"|"-h"|"--help")
            show_help
            ;;
        *)
            error "Unknown command: $command"
            show_help
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
