#!/bin/bash

# GaussOS Management Suite - Main Control Script
# Unified interface for all GaussOS management operations

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

# Script paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
START_SCRIPT="$SCRIPT_DIR/start.sh"
DATABASE_SCRIPT="$SCRIPT_DIR/database.sh"
MONITOR_SCRIPT="$SCRIPT_DIR/monitor.sh"

# Function to print header
print_header() {
    clear
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════════════════════╗"
    echo "║                    ${STAR} GaussOS Management Suite ${STAR}                    ║"
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
    
    if [ ! -f "$START_SCRIPT" ]; then
        missing_scripts+=("start.sh")
    fi
    
    if [ ! -f "$DATABASE_SCRIPT" ]; then
        missing_scripts+=("database.sh")
    fi
    
    if [ ! -f "$MONITOR_SCRIPT" ]; then
        missing_scripts+=("monitor.sh")
    fi
    
    if [ ${#missing_scripts[@]} -gt 0 ]; then
        print_status "error" "Missing management scripts: ${missing_scripts[*]}"
        return 1
    fi
    
    return 0
}

# Function to show main menu
show_main_menu() {
    echo -e "${WHITE}GaussOS Management Suite - Main Menu${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    echo ""
    echo -e "${CYAN}Service Management:${NC}"
    echo -e "  ${GREEN}1${NC} ${ARROW} ${ROCKET} Start all services"
    echo -e "  ${GREEN}2${NC} ${ARROW} ${SHIELD} Stop all services"
    echo -e "  ${GREEN}3${NC} ${ARROW} ${GEAR} Restart all services"
    echo -e "  ${GREEN}4${NC} ${ARROW} ${MONITOR} Check system status"
    echo ""
    echo -e "${CYAN}Database Operations:${NC}"
    echo -e "  ${GREEN}5${NC} ${ARROW} ${DATABASE} Database health check"
    echo -e "  ${GREEN}6${NC} ${ARROW} ${DATABASE} Create backup"
    echo -e "  ${GREEN}7${NC} ${ARROW} ${DATABASE} Restore from backup"
    echo -e "  ${GREEN}8${NC} ${ARROW} ${DATABASE} List backups"
    echo -e "  ${GREEN}9${NC} ${ARROW} ${SPEED} Optimize database"
    echo ""
    echo -e "${CYAN}Monitoring & Performance:${NC}"
    echo -e "  ${GREEN}10${NC} ${ARROW} ${MONITOR} Real-time dashboard"
    echo -e "  ${GREEN}11${NC} ${ARROW} ${SPEED} Performance analysis"
    echo -e "  ${GREEN}12${NC} ${ARROW} ${SHIELD} System alerts"
    echo -e "  ${GREEN}13${NC} ${ARROW} ${TOOLS} Export metrics"
    echo ""
    echo -e "${CYAN}Development & Testing:${NC}"
    echo -e "  ${GREEN}14${NC} ${ARROW} ${GEAR} Build project"
    echo -e "  ${GREEN}15${NC} ${ARROW} ${SPEED} Run tests"
    echo -e "  ${GREEN}16${NC} ${ARROW} ${BRAIN} Run demo"
    echo ""
    echo -e "${CYAN}System Maintenance:${NC}"
    echo -e "  ${GREEN}17${NC} ${ARROW} ${SHIELD} Clean system"
    echo -e "  ${GREEN}18${NC} ${ARROW} ${CLOCK} View logs"
    echo ""
    echo -e "${CYAN}Other:${NC}"
    echo -e "  ${GREEN}0${NC} ${ARROW} Exit"
    echo -e "  ${GREEN}h${NC} ${ARROW} Help"
    echo ""
}

# Function to handle service management
handle_service_management() {
    local choice=$1
    
    case $choice in
        1)
            print_status "loading" "Starting all GaussOS services..."
            "$START_SCRIPT" start
            ;;
        2)
            print_status "loading" "Stopping all GaussOS services..."
            "$START_SCRIPT" stop
            ;;
        3)
            print_status "loading" "Restarting all GaussOS services..."
            "$START_SCRIPT" restart
            ;;
        4)
            print_status "loading" "Checking system status..."
            "$START_SCRIPT" status
            ;;
        *)
            print_status "error" "Invalid choice"
            ;;
    esac
}

# Function to handle database operations
handle_database_operations() {
    local choice=$1
    
    case $choice in
        5)
            print_status "loading" "Checking database health..."
            "$DATABASE_SCRIPT" health
            ;;
        6)
            read -p "Enter backup name (optional): " backup_name
            print_status "loading" "Creating database backup..."
            "$DATABASE_SCRIPT" backup "$backup_name"
            ;;
        7)
            "$DATABASE_SCRIPT" list
            echo ""
            read -p "Enter backup filename: " backup_file
            print_status "loading" "Restoring from backup..."
            "$DATABASE_SCRIPT" restore "$backup_file"
            ;;
        8)
            print_status "loading" "Listing available backups..."
            "$DATABASE_SCRIPT" list
            ;;
        9)
            print_status "loading" "Optimizing database..."
            "$DATABASE_SCRIPT" optimize
            ;;
        *)
            print_status "error" "Invalid choice"
            ;;
    esac
}

# Function to handle monitoring operations
handle_monitoring_operations() {
    local choice=$1
    
    case $choice in
        10)
            read -p "Enter refresh rate in seconds (default: 2): " refresh_rate
            refresh_rate=${refresh_rate:-2}
            print_status "loading" "Starting real-time dashboard..."
            "$MONITOR_SCRIPT" dashboard "$refresh_rate"
            ;;
        11)
            print_status "loading" "Analyzing performance..."
            "$MONITOR_SCRIPT" performance
            ;;
        12)
            print_status "loading" "Checking system alerts..."
            "$MONITOR_SCRIPT" alerts
            ;;
        13)
            echo -e "${WHITE}Export Format Options:${NC}"
            echo -e "  ${GREEN}1${NC} ${ARROW} JSON"
            echo -e "  ${GREEN}2${NC} ${ARROW} CSV"
            read -p "Choose format (1-2): " format_choice
            
            case $format_choice in
                1) format="json" ;;
                2) format="csv" ;;
                *) format="json" ;;
            esac
            
            read -p "Enter output filename (optional): " output_file
            print_status "loading" "Exporting metrics..."
            "$MONITOR_SCRIPT" export "$format" "$output_file"
            ;;
        *)
            print_status "error" "Invalid choice"
            ;;
    esac
}

# Function to handle development operations
handle_development_operations() {
    local choice=$1
    
    case $choice in
        14)
            print_status "loading" "Building GaussOS project..."
            "$START_SCRIPT" build
            ;;
        15)
            print_status "loading" "Running tests and benchmarks..."
            "$START_SCRIPT" test
            ;;
        16)
            print_status "loading" "Running GaussOS demo..."
            cargo run --example simple_demo
            ;;
        *)
            print_status "error" "Invalid choice"
            ;;
    esac
}

# Function to handle system maintenance
handle_system_maintenance() {
    local choice=$1
    
    case $choice in
        17)
            print_status "loading" "Cleaning system..."
            "$START_SCRIPT" clean
            ;;
        18)
            echo -e "${WHITE}Log Options:${NC}"
            echo -e "  ${GREEN}1${NC} ${ARROW} Backend logs"
            echo -e "  ${GREEN}2${NC} ${ARROW} Frontend logs"
            read -p "Choose log type (1-2): " log_choice
            
            case $log_choice in
                1) "$START_SCRIPT" logs backend ;;
                2) "$START_SCRIPT" logs frontend ;;
                *) print_status "error" "Invalid choice" ;;
            esac
            ;;
        *)
            print_status "error" "Invalid choice"
            ;;
    esac
}

# Function to show help
show_help() {
    echo -e "${CYAN}GaussOS Management Suite - Help${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    echo ""
    echo -e "${WHITE}Overview:${NC}"
    echo "  This management suite provides a unified interface for all GaussOS operations."
    echo "  It includes service management, database operations, monitoring, and maintenance."
    echo ""
    echo -e "${WHITE}Quick Start:${NC}"
    echo "  1. Build the project: ${YELLOW}./scripts/manage.sh${NC} → Option 14"
    echo "  2. Start services: ${YELLOW}./scripts/manage.sh${NC} → Option 1"
    echo "  3. Check status: ${YELLOW}./scripts/manage.sh${NC} → Option 4"
    echo "  4. Monitor: ${YELLOW}./scripts/manage.sh${NC} → Option 10"
    echo ""
    echo -e "${WHITE}Individual Scripts:${NC}"
    echo "  ${YELLOW}./scripts/start.sh${NC}     - Service management"
    echo "  ${YELLOW}./scripts/database.sh${NC}  - Database operations"
    echo "  ${YELLOW}./scripts/monitor.sh${NC}   - Monitoring and performance"
    echo ""
    echo -e "${WHITE}For more information:${NC}"
    echo "  - README.md: Project documentation"
    echo "  - ARCHITECTURE.md: System architecture"
    echo "  - SPECS.md: Technical specifications"
    echo ""
}

# Function to run interactive menu
run_interactive_menu() {
    while true; do
        print_header
        show_main_menu
        
        echo -e "${WHITE}Enter your choice:${NC} "
        read -r choice
        
        case $choice in
            0)
                print_status "info" "Exiting GaussOS Management Suite"
                exit 0
                ;;
            h|H)
                show_help
                echo ""
                read -p "Press Enter to continue..."
                ;;
            1|2|3|4)
                handle_service_management "$choice"
                ;;
            5|6|7|8|9)
                handle_database_operations "$choice"
                ;;
            10|11|12|13)
                handle_monitoring_operations "$choice"
                ;;
            14|15|16)
                handle_development_operations "$choice"
                ;;
            17|18)
                handle_system_maintenance "$choice"
                ;;
            *)
                print_status "error" "Invalid choice: $choice"
                ;;
        esac
        
        echo ""
        read -p "Press Enter to continue..."
    done
}

# Function to handle direct command execution
handle_direct_command() {
    local command=$1
    shift
    
    case $command in
        "start")
            "$START_SCRIPT" start
            ;;
        "stop")
            "$START_SCRIPT" stop
            ;;
        "restart")
            "$START_SCRIPT" restart
            ;;
        "status")
            "$START_SCRIPT" status
            ;;
        "build")
            "$START_SCRIPT" build
            ;;
        "test")
            "$START_SCRIPT" test
            ;;
        "demo")
            cargo run --example simple_demo
            ;;
        "clean")
            "$START_SCRIPT" clean
            ;;
        "logs")
            "$START_SCRIPT" logs "$@"
            ;;
        "db-health")
            "$DATABASE_SCRIPT" health
            ;;
        "db-backup")
            "$DATABASE_SCRIPT" backup "$@"
            ;;
        "db-restore")
            "$DATABASE_SCRIPT" restore "$@"
            ;;
        "db-list")
            "$DATABASE_SCRIPT" list
            ;;
        "db-optimize")
            "$DATABASE_SCRIPT" optimize
            ;;
        "monitor")
            "$MONITOR_SCRIPT" dashboard "$@"
            ;;
        "performance")
            "$MONITOR_SCRIPT" performance
            ;;
        "alerts")
            "$MONITOR_SCRIPT" alerts
            ;;
        "export")
            "$MONITOR_SCRIPT" export "$@"
            ;;
        "help")
            show_help
            ;;
        *)
            print_status "error" "Unknown command: $command"
            echo ""
            echo -e "${WHITE}Available direct commands:${NC}"
            echo "  start, stop, restart, status, build, test, demo, clean"
            echo "  logs [backend|frontend], db-health, db-backup, db-restore"
            echo "  db-list, db-optimize, monitor, performance, alerts, export"
            echo ""
            echo "Use 'help' for more information or run without arguments for interactive menu."
            exit 1
            ;;
    esac
}

# Main script logic
main() {
    # Check if scripts are available
    if ! check_scripts; then
        print_status "error" "Please ensure all management scripts are present in the scripts directory"
        exit 1
    fi
    
    # If no arguments provided, run interactive menu
    if [ $# -eq 0 ]; then
        run_interactive_menu
    else
        # Handle direct command execution
        handle_direct_command "$@"
    fi
}

# Run main function with all arguments
main "$@"
