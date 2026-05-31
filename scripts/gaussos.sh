#!/bin/bash

# GaussOS Unified Management Script
# Combines main platform scripts and web-ui scripts

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
TOOLS="🔧"
CLOCK="⏰"
WEB="🌐"

# Script paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
WEB_UI_DIR="$PROJECT_ROOT/web-ui"

# Main scripts
START_SCRIPT="$SCRIPT_DIR/start.sh"
DATABASE_SCRIPT="$SCRIPT_DIR/database.sh"
MONITOR_SCRIPT="$SCRIPT_DIR/monitor.sh"
MANAGE_SCRIPT="$SCRIPT_DIR/manage.sh"

# Web-ui scripts
WEB_START_SCRIPT="$WEB_UI_DIR/start.sh"
WEB_STOP_SCRIPT="$WEB_UI_DIR/stop.sh"
WEB_RESTART_SCRIPT="$WEB_UI_DIR/restart.sh"
WEB_STATUS_SCRIPT="$WEB_UI_DIR/status.sh"
WEB_MONITOR_SCRIPT="$WEB_UI_DIR/monitor.sh"

# Function to print header
print_header() {
    clear
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════════════════════╗"
    echo "║                    ${STAR} GaussOS Unified Management ${STAR}                    ║"
    echo "║              ${BRAIN} Advanced AI Memory Management Platform ${BRAIN}              ║"
    echo "╚══════════════════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Function to print status
print_status() {
    local status=$1
    local message=$2
    
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

# Function to check script availability
check_scripts() {
    local missing_scripts=()
    
    # Check main scripts
    if [ ! -f "$START_SCRIPT" ]; then
        missing_scripts+=("scripts/start.sh")
    fi
    
    if [ ! -f "$DATABASE_SCRIPT" ]; then
        missing_scripts+=("scripts/database.sh")
    fi
    
    if [ ! -f "$MONITOR_SCRIPT" ]; then
        missing_scripts+=("scripts/monitor.sh")
    fi
    
    if [ ! -f "$MANAGE_SCRIPT" ]; then
        missing_scripts+=("scripts/manage.sh")
    fi
    
    # Check web-ui scripts
    if [ ! -f "$WEB_START_SCRIPT" ]; then
        missing_scripts+=("web-ui/start.sh")
    fi
    
    if [ ! -f "$WEB_STOP_SCRIPT" ]; then
        missing_scripts+=("web-ui/stop.sh")
    fi
    
    if [ ! -f "$WEB_STATUS_SCRIPT" ]; then
        missing_scripts+=("web-ui/status.sh")
    fi
    
    if [ ${#missing_scripts[@]} -gt 0 ]; then
        print_status "error" "Missing scripts: ${missing_scripts[*]}"
        return 1
    fi
    
    return 0
}

# Function to show main menu
show_main_menu() {
    print_header
    echo -e "${CYAN}${ROCKET} Available Operations:${NC}"
    echo ""
    echo -e "${WHITE}${ARROW} Platform Management:${NC}"
    echo "  1) ${GEAR} Start All Services"
    echo "  2) ${SHIELD} Stop All Services"
    echo "  3) ${SPEED} Restart All Services"
    echo "  4) ${MONITOR} System Status"
    echo "  5) ${TOOLS} Database Operations"
    echo "  6) ${CLOCK} Performance Monitoring"
    echo ""
    echo -e "${WHITE}${ARROW} Web Interface Management:${NC}"
    echo "  7) ${WEB} Start Web UI"
    echo "  8) ${WEB} Stop Web UI"
    echo "  9) ${WEB} Restart Web UI"
    echo "  10) ${WEB} Web UI Status"
    echo "  11) ${WEB} Monitor Web UI"
    echo ""
    echo -e "${WHITE}${ARROW} Advanced Operations:${NC}"
    echo "  12) ${BRAIN} Full System Management"
    echo "  13) ${DATABASE} Database Management"
    echo "  14) ${MONITOR} Advanced Monitoring"
    echo "  15) ${TOOLS} System Maintenance"
    echo ""
    echo -e "${WHITE}${ARROW} Information:${NC}"
    echo "  16) ${INFO} Show Help"
    echo "  17) ${EXIT} Exit"
    echo ""
    echo -e "${YELLOW}Enter your choice (1-17):${NC} "
}

# Function to handle platform operations
handle_platform_operation() {
    local operation=$1
    
    case $operation in
        "start")
            print_status "loading" "Starting all GaussOS services..."
            if [ -f "$START_SCRIPT" ]; then
                "$START_SCRIPT" start
                print_status "success" "All services started successfully"
            else
                print_status "error" "Start script not found"
            fi
            ;;
        "stop")
            print_status "loading" "Stopping all GaussOS services..."
            if [ -f "$START_SCRIPT" ]; then
                "$START_SCRIPT" stop
                print_status "success" "All services stopped successfully"
            else
                print_status "error" "Start script not found"
            fi
            ;;
        "restart")
            print_status "loading" "Restarting all GaussOS services..."
            if [ -f "$START_SCRIPT" ]; then
                "$START_SCRIPT" restart
                print_status "success" "All services restarted successfully"
            else
                print_status "error" "Start script not found"
            fi
            ;;
        "status")
            print_status "info" "Checking system status..."
            if [ -f "$START_SCRIPT" ]; then
                "$START_SCRIPT" status
            else
                print_status "error" "Start script not found"
            fi
            ;;
        "database")
            print_status "info" "Opening database management..."
            if [ -f "$DATABASE_SCRIPT" ]; then
                "$DATABASE_SCRIPT"
            else
                print_status "error" "Database script not found"
            fi
            ;;
        "monitor")
            print_status "info" "Starting performance monitoring..."
            if [ -f "$MONITOR_SCRIPT" ]; then
                "$MONITOR_SCRIPT"
            else
                print_status "error" "Monitor script not found"
            fi
            ;;
        "manage")
            print_status "info" "Opening full system management..."
            if [ -f "$MANAGE_SCRIPT" ]; then
                "$MANAGE_SCRIPT"
            else
                print_status "error" "Manage script not found"
            fi
            ;;
    esac
}

# Function to handle web-ui operations
handle_web_ui_operation() {
    local operation=$1
    
    case $operation in
        "start")
            print_status "loading" "Starting Web UI..."
            if [ -f "$WEB_START_SCRIPT" ]; then
                "$WEB_START_SCRIPT"
                print_status "success" "Web UI started successfully"
            else
                print_status "error" "Web UI start script not found"
            fi
            ;;
        "stop")
            print_status "loading" "Stopping Web UI..."
            if [ -f "$WEB_STOP_SCRIPT" ]; then
                "$WEB_STOP_SCRIPT"
                print_status "success" "Web UI stopped successfully"
            else
                print_status "error" "Web UI stop script not found"
            fi
            ;;
        "restart")
            print_status "loading" "Restarting Web UI..."
            if [ -f "$WEB_RESTART_SCRIPT" ]; then
                "$WEB_RESTART_SCRIPT"
                print_status "success" "Web UI restarted successfully"
            else
                print_status "error" "Web UI restart script not found"
            fi
            ;;
        "status")
            print_status "info" "Checking Web UI status..."
            if [ -f "$WEB_STATUS_SCRIPT" ]; then
                "$WEB_STATUS_SCRIPT"
            else
                print_status "error" "Web UI status script not found"
            fi
            ;;
        "monitor")
            print_status "info" "Starting Web UI monitoring..."
            if [ -f "$WEB_MONITOR_SCRIPT" ]; then
                "$WEB_MONITOR_SCRIPT" -c
            else
                print_status "error" "Web UI monitor script not found"
            fi
            ;;
    esac
}

# Function to show help
show_help() {
    print_header
    echo -e "${CYAN}${STAR} GaussOS Unified Management Help${NC}"
    echo ""
    echo -e "${WHITE}${ARROW} Quick Commands:${NC}"
    echo ""
    echo -e "${GREEN}Platform Management:${NC}"
    echo "  ./scripts/gaussos.sh platform start     # Start all services"
    echo "  ./scripts/gaussos.sh platform stop      # Stop all services"
    echo "  ./scripts/gaussos.sh platform restart   # Restart all services"
    echo "  ./scripts/gaussos.sh platform status    # Check system status"
    echo ""
    echo -e "${GREEN}Web UI Management:${NC}"
    echo "  ./scripts/gaussos.sh web start          # Start Web UI"
    echo "  ./scripts/gaussos.sh web stop           # Stop Web UI"
    echo "  ./scripts/gaussos.sh web restart        # Restart Web UI"
    echo "  ./scripts/gaussos.sh web status         # Check Web UI status"
    echo "  ./scripts/gaussos.sh web monitor        # Monitor Web UI"
    echo ""
    echo -e "${GREEN}Advanced Operations:${NC}"
    echo "  ./scripts/gaussos.sh database           # Database management"
    echo "  ./scripts/gaussos.sh monitor            # Performance monitoring"
    echo "  ./scripts/gaussos.sh manage             # Full system management"
    echo ""
    echo -e "${GREEN}Direct Script Access:${NC}"
    echo "  ./scripts/manage.sh                     # Main management interface"
    echo "  ./scripts/start.sh                      # Service management"
    echo "  ./scripts/database.sh                   # Database operations"
    echo "  ./scripts/monitor.sh                    # Performance monitoring"
    echo "  ./web-ui/start.sh                       # Web UI start"
    echo "  ./web-ui/stop.sh                        # Web UI stop"
    echo "  ./web-ui/status.sh                      # Web UI status"
    echo ""
    echo -e "${YELLOW}Press Enter to continue...${NC}"
    read -r
}

# Function to handle command line arguments
handle_cli_args() {
    local command=$1
    local subcommand=$2
    
    case $command in
        "platform")
            handle_platform_operation "$subcommand"
            ;;
        "web")
            handle_web_ui_operation "$subcommand"
            ;;
        "database")
            if [ -f "$DATABASE_SCRIPT" ]; then
                "$DATABASE_SCRIPT"
            else
                print_status "error" "Database script not found"
            fi
            ;;
        "monitor")
            if [ -f "$MONITOR_SCRIPT" ]; then
                "$MONITOR_SCRIPT"
            else
                print_status "error" "Monitor script not found"
            fi
            ;;
        "manage")
            if [ -f "$MANAGE_SCRIPT" ]; then
                "$MANAGE_SCRIPT"
            else
                print_status "error" "Manage script not found"
            fi
            ;;
        "help"|"--help"|"-h")
            show_help
            ;;
        *)
            echo -e "${RED}Unknown command: $command${NC}"
            echo "Use './scripts/gaussos.sh help' for usage information"
            exit 1
            ;;
    esac
}

# Main function
main() {
    # Check if scripts are available
    if ! check_scripts; then
        exit 1
    fi
    
    # Handle command line arguments
    if [ $# -gt 0 ]; then
        handle_cli_args "$@"
        return
    fi
    
    # Interactive mode
    while true; do
        show_main_menu
        read -r choice
        
        case $choice in
            1)
                handle_platform_operation "start"
                ;;
            2)
                handle_platform_operation "stop"
                ;;
            3)
                handle_platform_operation "restart"
                ;;
            4)
                handle_platform_operation "status"
                ;;
            5)
                handle_platform_operation "database"
                ;;
            6)
                handle_platform_operation "monitor"
                ;;
            7)
                handle_web_ui_operation "start"
                ;;
            8)
                handle_web_ui_operation "stop"
                ;;
            9)
                handle_web_ui_operation "restart"
                ;;
            10)
                handle_web_ui_operation "status"
                ;;
            11)
                handle_web_ui_operation "monitor"
                ;;
            12)
                handle_platform_operation "manage"
                ;;
            13)
                handle_platform_operation "database"
                ;;
            14)
                handle_platform_operation "monitor"
                ;;
            15)
                print_status "info" "System maintenance options..."
                if [ -f "$MANAGE_SCRIPT" ]; then
                    "$MANAGE_SCRIPT"
                else
                    print_status "error" "Manage script not found"
                fi
                ;;
            16)
                show_help
                ;;
            17)
                print_status "info" "Exiting GaussOS Management Suite"
                exit 0
                ;;
            *)
                print_status "error" "Invalid choice. Please select 1-17."
                ;;
        esac
        
        echo ""
        echo -e "${YELLOW}Press Enter to continue...${NC}"
        read -r
    done
}

# Handle script interruption
cleanup() {
    echo ""
    print_status "info" "GaussOS Management Suite interrupted"
    exit 1
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

# Run main function
main "$@"
