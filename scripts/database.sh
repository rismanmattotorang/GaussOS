#!/bin/bash

# GaussOS Database Management Script
# Professional database administration and monitoring tools

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
DATABASE="💾"
BACKUP="📦"
RESTORE="🔄"
OPTIMIZE="⚡"
MONITOR="📊"
SCALE="📈"
SHIELD="🛡️"
GEAR="⚙️"
SEARCH="🔍"

# Configuration
GAUSSOS_BIN="./target/release/gaussos"
BACKUP_DIR="./backups"
LOG_DIR="./logs"
CONFIG_FILE="./config.toml"

# Create necessary directories
mkdir -p "$BACKUP_DIR" "$LOG_DIR"

# Function to print header
print_header() {
    clear
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════════════════════╗"
    echo "║                        ${DATABASE} GaussOS Database Management ${DATABASE}                        ║"
    echo "║                    Professional Database Administration Tools                    ║"
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

# Function to check database connectivity
check_database_health() {
    echo -e "${CYAN}${MONITOR} Database Health Check${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    print_status "loading" "Checking database connectivity..."
    
    # Check if backend is running
    if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
        print_status "error" "Backend service is not running"
        print_status "info" "Start the backend first: ./scripts/start.sh start"
        return 1
    fi
    
    print_status "success" "Backend service is responding"
    
    # Check database status via API
    print_status "loading" "Checking database status..."
    
    local response=$(curl -s http://localhost:8080/status 2>/dev/null || echo "{}")
    if echo "$response" | grep -q "error"; then
        print_status "warning" "Database status check failed"
    else
        print_status "success" "Database is healthy and responding"
    fi
    
    # Check memory usage
    print_status "loading" "Checking memory usage..."
    local memory_usage=$(ps aux | grep gaussos | grep -v grep | awk '{sum+=$6} END {print sum/1024 " MB"}' || echo "Unknown")
    print_status "info" "Current memory usage: $memory_usage"
}

# Function to create database backup
create_backup() {
    local backup_name=$1
    local timestamp=$(date +"%Y%m%d_%H%M%S")
    local backup_file="${backup_name:-gaussos_backup}_${timestamp}.tar.gz"
    
    echo -e "${CYAN}${BACKUP} Creating Database Backup${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    print_status "loading" "Preparing backup: $backup_file"
    
    # Check if backend is running
    if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
        print_status "error" "Backend service is not running"
        return 1
    fi
    
    # Create backup directory
    mkdir -p "$BACKUP_DIR"
    
    # Create backup using GaussOS CLI
    print_status "loading" "Creating backup via GaussOS API..."
    
    if curl -s -X POST http://localhost:8080/admin/backup > "$BACKUP_DIR/$backup_file" 2>/dev/null; then
        print_status "success" "Backup created successfully"
        print_status "info" "Backup file: $BACKUP_DIR/$backup_file"
        
        # Show backup size
        local size=$(du -h "$BACKUP_DIR/$backup_file" | cut -f1)
        print_status "info" "Backup size: $size"
    else
        print_status "error" "Failed to create backup"
        return 1
    fi
}

# Function to restore database from backup
restore_backup() {
    local backup_file=$1
    
    if [ -z "$backup_file" ]; then
        print_status "error" "Please specify a backup file"
        echo "Usage: ./scripts/database.sh restore <backup_file>"
        return 1
    fi
    
    echo -e "${CYAN}${RESTORE} Restoring Database from Backup${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    # Check if backup file exists
    if [ ! -f "$BACKUP_DIR/$backup_file" ]; then
        print_status "error" "Backup file not found: $BACKUP_DIR/$backup_file"
        return 1
    fi
    
    print_status "warning" "This will overwrite the current database!"
    read -p "Are you sure you want to continue? (y/N): " -n 1 -r
    echo
    
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_status "info" "Restore cancelled"
        return 0
    fi
    
    print_status "loading" "Restoring from backup: $backup_file"
    
    # Check if backend is running
    if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
        print_status "error" "Backend service is not running"
        return 1
    fi
    
    # Restore backup using GaussOS CLI
    print_status "loading" "Restoring backup via GaussOS API..."
    
    if curl -s -X POST -F "backup=@$BACKUP_DIR/$backup_file" http://localhost:8080/admin/restore > /dev/null 2>&1; then
        print_status "success" "Database restored successfully"
    else
        print_status "error" "Failed to restore database"
        return 1
    fi
}

# Function to list available backups
list_backups() {
    echo -e "${CYAN}${SEARCH} Available Backups${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    if [ ! -d "$BACKUP_DIR" ] || [ -z "$(ls -A $BACKUP_DIR 2>/dev/null)" ]; then
        print_status "info" "No backups found"
        return 0
    fi
    
    echo -e "${WHITE}Backup Files:${NC}"
    echo ""
    
    local total_size=0
    local count=0
    
    for backup in "$BACKUP_DIR"/*.tar.gz; do
        if [ -f "$backup" ]; then
            local filename=$(basename "$backup")
            local size=$(du -h "$backup" | cut -f1)
            local date=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M" "$backup" 2>/dev/null || echo "Unknown")
            
            echo -e "  ${GREEN}${filename}${NC}"
            echo -e "    ${BLUE}Size:${NC} $size"
            echo -e "    ${BLUE}Date:${NC} $date"
            echo ""
            
            total_size=$(($total_size + $(du -k "$backup" | cut -f1)))
            ((count++))
        fi
    done
    
    if [ $count -gt 0 ]; then
        local total_size_mb=$(echo "scale=1; $total_size/1024" | bc)
        print_status "info" "Total backups: $count"
        print_status "info" "Total size: ${total_size_mb}MB"
    fi
}

# Function to optimize database
optimize_database() {
    echo -e "${CYAN}${OPTIMIZE} Database Optimization${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    print_status "loading" "Starting database optimization..."
    
    # Check if backend is running
    if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
        print_status "error" "Backend service is not running"
        return 1
    fi
    
    # Run optimization via API
    print_status "loading" "Running database optimization..."
    
    if curl -s -X POST http://localhost:8080/admin/optimize > /dev/null 2>&1; then
        print_status "success" "Database optimization completed"
    else
        print_status "warning" "Optimization may have failed (check logs for details)"
    fi
    
    # Show optimization results
    print_status "loading" "Checking optimization results..."
    check_database_health
}

# Function to show database statistics
show_statistics() {
    echo -e "${CYAN}${SCALE} Database Statistics${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    print_status "loading" "Collecting database statistics..."
    
    # Check if backend is running
    if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
        print_status "error" "Backend service is not running"
        return 1
    fi
    
    # Get statistics via API
    local stats=$(curl -s http://localhost:8080/metrics 2>/dev/null || echo "{}")
    
    if [ "$stats" != "{}" ]; then
        echo -e "${WHITE}Database Metrics:${NC}"
        echo ""
        
        # Parse and display metrics (simplified)
        echo -e "  ${BLUE}Memory Operations:${NC}"
        echo -e "    ${ARROW} Total memories: $(echo "$stats" | grep -o '"total_memories":[0-9]*' | cut -d: -f2 || echo "Unknown")"
        echo -e "    ${ARROW} Cache hit rate: $(echo "$stats" | grep -o '"cache_hit_rate":[0-9.]*' | cut -d: -f2 || echo "Unknown")%"
        echo ""
        
        echo -e "  ${BLUE}Performance:${NC}"
        echo -e "    ${ARROW} Average response time: $(echo "$stats" | grep -o '"avg_response_time":[0-9.]*' | cut -d: -f2 || echo "Unknown")ms"
        echo -e "    ${ARROW} Requests per second: $(echo "$stats" | grep -o '"requests_per_second":[0-9.]*' | cut -d: -f2 || echo "Unknown")"
        echo ""
        
        echo -e "  ${BLUE}Storage:${NC}"
        echo -e "    ${ARROW} Storage size: $(echo "$stats" | grep -o '"storage_size":[0-9]*' | cut -d: -f2 || echo "Unknown") bytes"
        echo -e "    ${ARROW} Average memory size: $(echo "$stats" | grep -o '"avg_memory_size":[0-9.]*' | cut -d: -f2 || echo "Unknown") bytes"
    else
        print_status "warning" "Could not retrieve statistics"
    fi
}

# Function to clean old backups
clean_backups() {
    local days=${1:-30}
    
    echo -e "${CYAN}${SHIELD} Cleaning Old Backups${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    print_status "loading" "Finding backups older than $days days..."
    
    if [ ! -d "$BACKUP_DIR" ]; then
        print_status "info" "No backup directory found"
        return 0
    fi
    
    local old_backups=$(find "$BACKUP_DIR" -name "*.tar.gz" -mtime +$days 2>/dev/null)
    
    if [ -z "$old_backups" ]; then
        print_status "info" "No old backups found"
        return 0
    fi
    
    echo -e "${WHITE}Old backups to be removed:${NC}"
    echo ""
    
    local total_size=0
    local count=0
    
    for backup in $old_backups; do
        local filename=$(basename "$backup")
        local size=$(du -h "$backup" | cut -f1)
        local date=$(stat -f "%Sm" -t "%Y-%m-%d" "$backup" 2>/dev/null || echo "Unknown")
        
        echo -e "  ${YELLOW}${filename}${NC} (${date}) - $size"
        total_size=$(($total_size + $(du -k "$backup" | cut -f1)))
        ((count++))
    done
    
    echo ""
    local total_size_mb=$(echo "scale=1; $total_size/1024" | bc)
    print_status "info" "Total files to remove: $count"
    print_status "info" "Total space to free: ${total_size_mb}MB"
    
    read -p "Do you want to remove these backups? (y/N): " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_status "loading" "Removing old backups..."
        
        for backup in $old_backups; do
            rm "$backup"
            print_status "success" "Removed: $(basename "$backup")"
        done
        
        print_status "success" "Cleanup completed"
    else
        print_status "info" "Cleanup cancelled"
    fi
}

# Function to show main menu
show_menu() {
    echo -e "${WHITE}Available Commands:${NC}"
    echo ""
    echo -e "  ${GREEN}health${NC}     ${ARROW} Check database health and connectivity"
    echo -e "  ${GREEN}backup${NC}     ${ARROW} Create database backup"
    echo -e "  ${GREEN}restore${NC}    ${ARROW} Restore database from backup"
    echo -e "  ${GREEN}list${NC}       ${ARROW} List available backups"
    echo -e "  ${GREEN}optimize${NC}   ${ARROW} Optimize database performance"
    echo -e "  ${GREEN}stats${NC}      ${ARROW} Show database statistics"
    echo -e "  ${GREEN}clean${NC}      ${ARROW} Clean old backups"
    echo -e "  ${GREEN}help${NC}       ${ARROW} Show this help message"
    echo ""
    echo -e "Examples:"
    echo -e "  ${YELLOW}./scripts/database.sh health${NC}"
    echo -e "  ${YELLOW}./scripts/database.sh backup my_backup${NC}"
    echo -e "  ${YELLOW}./scripts/database.sh restore gaussos_backup_20231201_143022.tar.gz${NC}"
    echo -e "  ${YELLOW}./scripts/database.sh clean 7${NC}"
}

# Main script logic
main() {
    local command=$1
    local arg1=$2
    local arg2=$3
    
    print_header
    
    case $command in
        "health")
            check_database_health
            ;;
        "backup")
            create_backup "$arg1"
            ;;
        "restore")
            restore_backup "$arg1"
            ;;
        "list")
            list_backups
            ;;
        "optimize")
            optimize_database
            ;;
        "stats")
            show_statistics
            ;;
        "clean")
            clean_backups "$arg1"
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
