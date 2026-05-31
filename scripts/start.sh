#!/bin/bash

# GaussOS Management Suite - Startup Script
# A beautiful, professional console interface for managing GaussOS services

set -e

# Colors for beautiful UI
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Unicode symbols for UI
CHECK_MARK="✓"
CROSS_MARK="✗"
ARROW="→"
STAR="★"
ROCKET="🚀"
BRAIN="🧠"
GEAR="⚙️"
DATABASE="💾"
GLOBE="🌐"
SHIELD="🛡️"
SPEED="⚡"
MONITOR="📊"

# Configuration
GAUSSOS_BIN="./target/release/gaussos"
WEB_UI_DIR="./web-ui"
LOG_DIR="./logs"
PID_DIR="./pids"
CONFIG_FILE="./config.toml"

# Create necessary directories
mkdir -p "$LOG_DIR" "$PID_DIR"

# Function to print header
print_header() {
    clear
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════════════════════╗"
    echo "║                           ${STAR} GaussOS Management Suite ${STAR}                           ║"
    echo "║                    ${BRAIN} Advanced AI Memory Management Platform ${BRAIN}                    ║"
    echo "╚══════════════════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Function to print status
print_status() {
    local status=$1
    local message=$2
    local icon=$3
    
    case $status in
        "success")
            echo -e "  ${GREEN}${CHECK_MARK}${NC} $message"
            ;;
        "error")
            echo -e "  ${RED}${CROSS_MARK}${NC} $message"
            ;;
        "warning")
            echo -e "  ${YELLOW}⚠${NC} $message"
            ;;
        "info")
            echo -e "  ${BLUE}${ARROW}${NC} $message"
            ;;
        "loading")
            echo -e "  ${CYAN}${GEAR}${NC} $message"
            ;;
    esac
}

# Function to check if service is running
is_service_running() {
    local service_name=$1
    local pid_file="$PID_DIR/${service_name}.pid"
    
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if ps -p "$pid" > /dev/null 2>&1; then
            return 0
        else
            rm -f "$pid_file"
        fi
    fi
    return 1
}

# Function to start backend service
start_backend() {
    echo -e "${CYAN}${ROCKET} Starting GaussOS Backend Service${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    if is_service_running "gaussos-backend"; then
        print_status "warning" "Backend service is already running"
        return 0
    fi
    
    print_status "loading" "Initializing GaussOS backend..."
    
    # Check if binary exists
    if [ ! -f "$GAUSSOS_BIN" ]; then
        print_status "error" "GaussOS binary not found. Please build the project first:"
        echo "    cargo build --release --features cli-bin"
        return 1
    fi
    
    # Start backend service
    print_status "loading" "Starting backend server on port 8080..."
    
    nohup "$GAUSSOS_BIN" server --port 8080 > "$LOG_DIR/backend.log" 2>&1 &
    local backend_pid=$!
    echo $backend_pid > "$PID_DIR/gaussos-backend.pid"
    
    # Wait a moment for service to start
    sleep 2
    
    if is_service_running "gaussos-backend"; then
        print_status "success" "Backend service started successfully (PID: $backend_pid)"
        print_status "info" "Logs: $LOG_DIR/backend.log"
        print_status "info" "API: http://localhost:8080"
        return 0
    else
        print_status "error" "Failed to start backend service"
        print_status "info" "Check logs: $LOG_DIR/backend.log"
        return 1
    fi
}

# Function to start web frontend
start_frontend() {
    echo -e "${CYAN}${GLOBE} Starting GaussOS Web Frontend${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    if is_service_running "gaussos-frontend"; then
        print_status "warning" "Frontend service is already running"
        return 0
    fi
    
    print_status "loading" "Initializing web frontend..."
    
    # Check if web-ui directory exists
    if [ ! -d "$WEB_UI_DIR" ]; then
        print_status "error" "Web UI directory not found: $WEB_UI_DIR"
        return 1
    fi
    
    # Check if deno is available
    if ! command -v deno &> /dev/null; then
        print_status "error" "Deno is not installed. Please install Deno first:"
        echo "    curl -fsSL https://deno.land/x/install/install.sh | sh"
        return 1
    fi
    
    # Start frontend service
    print_status "loading" "Starting web frontend on port 3000..."
    
    cd "$WEB_UI_DIR"
    nohup deno run --allow-net --allow-read main.ts > "../$LOG_DIR/frontend.log" 2>&1 &
    local frontend_pid=$!
    echo $frontend_pid > "../$PID_DIR/gaussos-frontend.pid"
    cd ..
    
    # Wait a moment for service to start
    sleep 3
    
    if is_service_running "gaussos-frontend"; then
        print_status "success" "Frontend service started successfully (PID: $frontend_pid)"
        print_status "info" "Logs: $LOG_DIR/frontend.log"
        print_status "info" "Web UI: http://localhost:3000"
        return 0
    else
        print_status "error" "Failed to start frontend service"
        print_status "info" "Check logs: $LOG_DIR/frontend.log"
        return 1
    fi
}

# Function to check system status
check_status() {
    echo -e "${CYAN}${MONITOR} System Status${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    local all_running=true
    
    # Check backend
    if is_service_running "gaussos-backend"; then
        local backend_pid=$(cat "$PID_DIR/gaussos-backend.pid")
        print_status "success" "Backend Service: Running (PID: $backend_pid)"
    else
        print_status "error" "Backend Service: Not running"
        all_running=false
    fi
    
    # Check frontend
    if is_service_running "gaussos-frontend"; then
        local frontend_pid=$(cat "$PID_DIR/gaussos-frontend.pid")
        print_status "success" "Frontend Service: Running (PID: $frontend_pid)"
    else
        print_status "error" "Frontend Service: Not running"
        all_running=false
    fi
    
    # Check database connectivity
    print_status "loading" "Checking database connectivity..."
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        print_status "success" "Database: Connected and healthy"
    else
        print_status "warning" "Database: Connection failed (backend may not be ready)"
    fi
    
    echo ""
    if [ "$all_running" = true ]; then
        print_status "success" "All services are running!"
        echo -e "${GREEN}${STAR} GaussOS is ready to use!${NC}"
        echo -e "${BLUE}${ARROW} API: http://localhost:8080${NC}"
        echo -e "${BLUE}${ARROW} Web UI: http://localhost:3000${NC}"
    else
        print_status "warning" "Some services are not running"
    fi
}

# Function to stop all services
stop_services() {
    echo -e "${CYAN}${SHIELD} Stopping GaussOS Services${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    local stopped_count=0
    
    # Stop backend
    if is_service_running "gaussos-backend"; then
        local backend_pid=$(cat "$PID_DIR/gaussos-backend.pid")
        print_status "loading" "Stopping backend service (PID: $backend_pid)..."
        kill "$backend_pid" 2>/dev/null || true
        rm -f "$PID_DIR/gaussos-backend.pid"
        print_status "success" "Backend service stopped"
        ((stopped_count++))
    else
        print_status "info" "Backend service is not running"
    fi
    
    # Stop frontend
    if is_service_running "gaussos-frontend"; then
        local frontend_pid=$(cat "$PID_DIR/gaussos-frontend.pid")
        print_status "loading" "Stopping frontend service (PID: $frontend_pid)..."
        kill "$frontend_pid" 2>/dev/null || true
        rm -f "$PID_DIR/gaussos-frontend.pid"
        print_status "success" "Frontend service stopped"
        ((stopped_count++))
    else
        print_status "info" "Frontend service is not running"
    fi
    
    echo ""
    if [ $stopped_count -gt 0 ]; then
        print_status "success" "Stopped $stopped_count service(s)"
    else
        print_status "info" "No services were running"
    fi
}

# Function to show logs
show_logs() {
    local service=$1
    
    case $service in
        "backend"|"b")
            if [ -f "$LOG_DIR/backend.log" ]; then
                echo -e "${CYAN}${MONITOR} Backend Logs${NC}"
                echo "══════════════════════════════════════════════════════════════════════════════"
                tail -f "$LOG_DIR/backend.log"
            else
                print_status "error" "Backend log file not found"
            fi
            ;;
        "frontend"|"f")
            if [ -f "$LOG_DIR/frontend.log" ]; then
                echo -e "${CYAN}${MONITOR} Frontend Logs${NC}"
                echo "══════════════════════════════════════════════════════════════════════════════"
                tail -f "$LOG_DIR/frontend.log"
            else
                print_status "error" "Frontend log file not found"
            fi
            ;;
        *)
            print_status "error" "Invalid service. Use 'backend' or 'frontend'"
            ;;
    esac
}

# Function to show main menu
show_menu() {
    echo -e "${WHITE}Available Commands:${NC}"
    echo ""
    echo -e "  ${GREEN}start${NC}     ${ARROW} Start all GaussOS services"
    echo -e "  ${GREEN}stop${NC}      ${ARROW} Stop all GaussOS services"
    echo -e "  ${GREEN}restart${NC}   ${ARROW} Restart all GaussOS services"
    echo -e "  ${GREEN}status${NC}    ${ARROW} Show system status"
    echo -e "  ${GREEN}logs${NC}      ${ARROW} Show service logs (backend|frontend)"
    echo -e "  ${GREEN}build${NC}     ${ARROW} Build GaussOS project"
    echo -e "  ${GREEN}test${NC}      ${ARROW} Run tests and benchmarks"
    echo -e "  ${GREEN}clean${NC}     ${ARROW} Clean logs and temporary files"
    echo -e "  ${GREEN}help${NC}      ${ARROW} Show this help message"
    echo ""
    echo -e "Examples:"
    echo -e "  ${YELLOW}./scripts/start.sh start${NC}"
    echo -e "  ${YELLOW}./scripts/start.sh logs backend${NC}"
    echo -e "  ${YELLOW}./scripts/start.sh status${NC}"
}

# Main script logic
main() {
    local command=$1
    local service=$2
    
    print_header
    
    case $command in
        "start")
            start_backend
            echo ""
            start_frontend
            echo ""
            check_status
            ;;
        "stop")
            stop_services
            ;;
        "restart")
            stop_services
            echo ""
            sleep 2
            start_backend
            echo ""
            start_frontend
            echo ""
            check_status
            ;;
        "status")
            check_status
            ;;
        "logs")
            show_logs "$service"
            ;;
        "build")
            echo -e "${CYAN}${GEAR} Building GaussOS Project${NC}"
            echo "══════════════════════════════════════════════════════════════════════════════"
            print_status "loading" "Building GaussOS with optimizations..."
            cargo build --release --features cli-bin
            print_status "success" "Build completed successfully"
            ;;
        "test")
            echo -e "${CYAN}${SPEED} Running Tests and Benchmarks${NC}"
            echo "══════════════════════════════════════════════════════════════════════════════"
            print_status "loading" "Running unit tests..."
            cargo test
            print_status "loading" "Running integration tests..."
            cargo test --test integration_tests
            print_status "loading" "Running benchmarks..."
            cargo bench
            print_status "success" "All tests completed"
            ;;
        "clean")
            echo -e "${CYAN}${SHIELD} Cleaning System${NC}"
            echo "══════════════════════════════════════════════════════════════════════════════"
            print_status "loading" "Cleaning log files..."
            rm -rf "$LOG_DIR"/*
            print_status "loading" "Cleaning PID files..."
            rm -rf "$PID_DIR"/*
            print_status "loading" "Cleaning build artifacts..."
            cargo clean
            print_status "success" "System cleaned successfully"
            ;;
        "help"|"")
            show_menu
            ;;
        *)
            print_status "error" "Unknown command: $command"
            echo ""
            show_menu
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"
