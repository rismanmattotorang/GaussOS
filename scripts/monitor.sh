#!/bin/bash

# GaussOS Monitoring and Performance Script
# Real-time system monitoring with beautiful console UI

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
MONITOR="📊"
SPEED="⚡"
MEMORY="💾"
CPU="🖥️"
NETWORK="🌐"
ALERT="🚨"
CHART="📈"
GEAR="⚙️"
CLOCK="⏰"

# Configuration
GAUSSOS_BIN="./target/release/gaussos"
LOG_DIR="./logs"
PID_DIR="./pids"
METRICS_FILE="./logs/metrics.json"

# Create necessary directories
mkdir -p "$LOG_DIR" "$PID_DIR"

# Function to print header
print_header() {
    clear
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════════════════════╗"
    echo "║                        ${MONITOR} GaussOS Monitoring Suite ${MONITOR}                        ║"
    echo "║                    Real-time Performance and System Monitoring                    ║"
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

# Function to get CPU usage
get_cpu_usage() {
    top -l 1 | grep "CPU usage" | awk '{print $3}' | sed 's/%//'
}

# Function to get memory usage
get_memory_usage() {
    vm_stat | grep "Pages free" | awk '{print $3}' | sed 's/\.//'
}

# Function to get disk usage
get_disk_usage() {
    df -h . | tail -1 | awk '{print $5}' | sed 's/%//'
}

# Function to get network stats
get_network_stats() {
    netstat -i | grep en0 | awk '{print $4, $8}'
}

# Function to get GaussOS process info
get_gaussos_process_info() {
    local pid_file="$PID_DIR/gaussos-backend.pid"
    
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if ps -p "$pid" > /dev/null 2>&1; then
            local cpu=$(ps -p "$pid" -o %cpu= 2>/dev/null || echo "0")
            local mem=$(ps -p "$pid" -o %mem= 2>/dev/null || echo "0")
            local vsz=$(ps -p "$pid" -o vsz= 2>/dev/null || echo "0")
            echo "$pid $cpu $mem $vsz"
        else
            echo "0 0 0 0"
        fi
    else
        echo "0 0 0 0"
    fi
}

# Function to get API metrics
get_api_metrics() {
    local response=$(curl -s http://localhost:8080/metrics 2>/dev/null || echo "{}")
    echo "$response"
}

# Function to display real-time dashboard
show_dashboard() {
    local refresh_rate=${1:-2}
    
    print_header
    echo -e "${CYAN}${CLOCK} Real-time Dashboard (Refresh: ${refresh_rate}s)${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    echo ""
    
    # Trap Ctrl+C to exit gracefully
    trap 'echo ""; print_status "info" "Dashboard stopped"; exit 0' INT
    
    while true; do
        # Move cursor to top
        echo -en "\033[H"
        
        # Get system metrics
        local cpu_usage=$(get_cpu_usage)
        local memory_usage=$(get_memory_usage)
        local disk_usage=$(get_disk_usage)
        local gaussos_info=$(get_gaussos_process_info)
        local api_metrics=$(get_api_metrics)
        
        # Parse GaussOS process info
        read -r gaussos_pid gaussos_cpu gaussos_mem gaussos_vsz <<< "$gaussos_info"
        
        # Display system metrics
        echo -e "${WHITE}System Metrics:${NC}"
        echo -e "  ${CPU} CPU Usage:     ${BLUE}${cpu_usage}%${NC}"
        echo -e "  ${MEMORY} Memory Usage:   ${BLUE}${memory_usage}%${NC}"
        echo -e "  ${MEMORY} Disk Usage:     ${BLUE}${disk_usage}%${NC}"
        echo ""
        
        # Display GaussOS process metrics
        echo -e "${WHITE}GaussOS Process:${NC}"
        if [ "$gaussos_pid" != "0" ]; then
            echo -e "  ${GREEN}${CHECK_MARK}${NC} Process ID:     ${BLUE}${gaussos_pid}${NC}"
            echo -e "  ${CPU} CPU Usage:     ${BLUE}${gaussos_cpu}%${NC}"
            echo -e "  ${MEMORY} Memory Usage:   ${BLUE}${gaussos_mem}%${NC}"
            echo -e "  ${MEMORY} Virtual Memory: ${BLUE}$(echo "scale=1; $gaussos_vsz/1024" | bc) MB${NC}"
        else
            echo -e "  ${RED}${CROSS_MARK}${NC} Process:        ${RED}Not running${NC}"
        fi
        echo ""
        
        # Display API metrics if available
        if [ "$api_metrics" != "{}" ]; then
            echo -e "${WHITE}API Metrics:${NC}"
            local total_memories=$(echo "$api_metrics" | grep -o '"total_memories":[0-9]*' | cut -d: -f2 || echo "0")
            local cache_hit_rate=$(echo "$api_metrics" | grep -o '"cache_hit_rate":[0-9.]*' | cut -d: -f2 || echo "0")
            local avg_response_time=$(echo "$api_metrics" | grep -o '"avg_response_time":[0-9.]*' | cut -d: -f2 || echo "0")
            local requests_per_second=$(echo "$api_metrics" | grep -o '"requests_per_second":[0-9.]*' | cut -d: -f2 || echo "0")
            
            echo -e "  ${MEMORY} Total Memories: ${BLUE}${total_memories}${NC}"
            echo -e "  ${SPEED} Cache Hit Rate:  ${BLUE}${cache_hit_rate}%${NC}"
            echo -e "  ${CLOCK} Avg Response:    ${BLUE}${avg_response_time}ms${NC}"
            echo -e "  ${NETWORK} Requests/sec:    ${BLUE}${requests_per_second}${NC}"
        else
            echo -e "${WHITE}API Metrics:${NC}"
            echo -e "  ${YELLOW}⚠${NC} API:           ${YELLOW}Not available${NC}"
        fi
        echo ""
        
        # Display status indicators
        echo -e "${WHITE}Status:${NC}"
        if [ "$gaussos_pid" != "0" ]; then
            echo -e "  ${GREEN}${CHECK_MARK}${NC} Backend:       ${GREEN}Running${NC}"
        else
            echo -e "  ${RED}${CROSS_MARK}${NC} Backend:       ${RED}Stopped${NC}"
        fi
        
        if curl -s http://localhost:8080/health > /dev/null 2>&1; then
            echo -e "  ${GREEN}${CHECK_MARK}${NC} API Health:    ${GREEN}Healthy${NC}"
        else
            echo -e "  ${RED}${CROSS_MARK}${NC} API Health:    ${RED}Unhealthy${NC}"
        fi
        
        if [ -f "$PID_DIR/gaussos-frontend.pid" ]; then
            local frontend_pid=$(cat "$PID_DIR/gaussos-frontend.pid")
            if ps -p "$frontend_pid" > /dev/null 2>&1; then
                echo -e "  ${GREEN}${CHECK_MARK}${NC} Frontend:      ${GREEN}Running${NC}"
            else
                echo -e "  ${RED}${CROSS_MARK}${NC} Frontend:      ${RED}Stopped${NC}"
            fi
        else
            echo -e "  ${RED}${CROSS_MARK}${NC} Frontend:      ${RED}Not started${NC}"
        fi
        
        echo ""
        echo -e "${YELLOW}Press Ctrl+C to exit${NC}"
        
        # Save metrics to file
        cat > "$METRICS_FILE" << EOF
{
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "system": {
    "cpu_usage": "$cpu_usage",
    "memory_usage": "$memory_usage",
    "disk_usage": "$disk_usage"
  },
  "gaussos": {
    "pid": "$gaussos_pid",
    "cpu_usage": "$gaussos_cpu",
    "memory_usage": "$gaussos_mem",
    "virtual_memory_mb": "$(echo "scale=1; $gaussos_vsz/1024" | bc)"
  },
  "api": {
    "total_memories": "$total_memories",
    "cache_hit_rate": "$cache_hit_rate",
    "avg_response_time": "$avg_response_time",
    "requests_per_second": "$requests_per_second"
  }
}
EOF
        
        sleep "$refresh_rate"
    done
}

# Function to show performance analysis
show_performance_analysis() {
    echo -e "${CYAN}${CHART} Performance Analysis${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    print_status "loading" "Collecting performance data..."
    
    # Check if backend is running
    if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
        print_status "error" "Backend service is not running"
        return 1
    fi
    
    # Get current metrics
    local api_metrics=$(get_api_metrics)
    
    if [ "$api_metrics" != "{}" ]; then
        echo -e "${WHITE}Performance Metrics:${NC}"
        echo ""
        
        # Parse metrics
        local total_memories=$(echo "$api_metrics" | grep -o '"total_memories":[0-9]*' | cut -d: -f2 || echo "0")
        local cache_hit_rate=$(echo "$api_metrics" | grep -o '"cache_hit_rate":[0-9.]*' | cut -d: -f2 || echo "0")
        local avg_response_time=$(echo "$api_metrics" | grep -o '"avg_response_time":[0-9.]*' | cut -d: -f2 || echo "0")
        local requests_per_second=$(echo "$api_metrics" | grep -o '"requests_per_second":[0-9.]*' | cut -d: -f2 || echo "0")
        
        # Display metrics with performance indicators
        echo -e "  ${MEMORY} Memory Operations:"
        echo -e "    ${ARROW} Total memories: ${BLUE}${total_memories}${NC}"
        
        echo -e "  ${SPEED} Cache Performance:"
        if (( $(echo "$cache_hit_rate >= 90" | bc -l) )); then
            echo -e "    ${ARROW} Hit rate: ${GREEN}${cache_hit_rate}% (Excellent)${NC}"
        elif (( $(echo "$cache_hit_rate >= 70" | bc -l) )); then
            echo -e "    ${ARROW} Hit rate: ${YELLOW}${cache_hit_rate}% (Good)${NC}"
        else
            echo -e "    ${ARROW} Hit rate: ${RED}${cache_hit_rate}% (Needs improvement)${NC}"
        fi
        
        echo -e "  ${CLOCK} Response Time:"
        if (( $(echo "$avg_response_time <= 10" | bc -l) )); then
            echo -e "    ${ARROW} Average: ${GREEN}${avg_response_time}ms (Excellent)${NC}"
        elif (( $(echo "$avg_response_time <= 50" | bc -l) )); then
            echo -e "    ${ARROW} Average: ${YELLOW}${avg_response_time}ms (Good)${NC}"
        else
            echo -e "    ${ARROW} Average: ${RED}${avg_response_time}ms (Slow)${NC}"
        fi
        
        echo -e "  ${NETWORK} Throughput:"
        if (( $(echo "$requests_per_second >= 1000" | bc -l) )); then
            echo -e "    ${ARROW} Requests/sec: ${GREEN}${requests_per_second} (High)${NC}"
        elif (( $(echo "$requests_per_second >= 100" | bc -l) )); then
            echo -e "    ${ARROW} Requests/sec: ${YELLOW}${requests_per_second} (Medium)${NC}"
        else
            echo -e "    ${ARROW} Requests/sec: ${RED}${requests_per_second} (Low)${NC}"
        fi
        
        echo ""
        
        # Performance recommendations
        echo -e "${WHITE}Performance Recommendations:${NC}"
        echo ""
        
        if (( $(echo "$cache_hit_rate < 70" | bc -l) )); then
            print_status "warning" "Consider increasing cache size or optimizing cache strategy"
        fi
        
        if (( $(echo "$avg_response_time > 50" | bc -l) )); then
            print_status "warning" "Response time is high. Consider database optimization"
        fi
        
        if (( $(echo "$requests_per_second < 100" | bc -l) )); then
            print_status "warning" "Throughput is low. Check for bottlenecks"
        fi
        
        if (( $(echo "$cache_hit_rate >= 90" | bc -l) )) && (( $(echo "$avg_response_time <= 10" | bc -l) )) && (( $(echo "$requests_per_second >= 1000" | bc -l) )); then
            print_status "success" "Performance is excellent across all metrics!"
        fi
    else
        print_status "error" "Could not retrieve performance metrics"
    fi
}

# Function to show system alerts
show_alerts() {
    echo -e "${CYAN}${ALERT} System Alerts${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    local alert_count=0
    
    # Check CPU usage
    local cpu_usage=$(get_cpu_usage)
    if (( $(echo "$cpu_usage > 80" | bc -l) )); then
        print_status "error" "High CPU usage: ${cpu_usage}%"
        ((alert_count++))
    fi
    
    # Check memory usage
    local memory_usage=$(get_memory_usage)
    if (( $(echo "$memory_usage > 90" | bc -l) )); then
        print_status "error" "High memory usage: ${memory_usage}%"
        ((alert_count++))
    fi
    
    # Check disk usage
    local disk_usage=$(get_disk_usage)
    if (( $(echo "$disk_usage > 85" | bc -l) )); then
        print_status "error" "High disk usage: ${disk_usage}%"
        ((alert_count++))
    fi
    
    # Check if GaussOS is running
    local gaussos_info=$(get_gaussos_process_info)
    read -r gaussos_pid gaussos_cpu gaussos_mem gaussos_vsz <<< "$gaussos_info"
    
    if [ "$gaussos_pid" = "0" ]; then
        print_status "error" "GaussOS backend is not running"
        ((alert_count++))
    fi
    
    # Check API health
    if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
        print_status "error" "API health check failed"
        ((alert_count++))
    fi
    
    # Check GaussOS memory usage
    if [ "$gaussos_pid" != "0" ] && (( $(echo "$gaussos_mem > 50" | bc -l) )); then
        print_status "warning" "GaussOS using high memory: ${gaussos_mem}%"
        ((alert_count++))
    fi
    
    echo ""
    if [ $alert_count -eq 0 ]; then
        print_status "success" "No alerts detected - system is healthy"
    else
        print_status "warning" "Found $alert_count alert(s)"
    fi
}

# Function to export metrics
export_metrics() {
    local format=${1:-json}
    local output_file=${2:-"./logs/metrics_$(date +%Y%m%d_%H%M%S).$format"}
    
    echo -e "${CYAN}${CHART} Exporting Metrics${NC}"
    echo "══════════════════════════════════════════════════════════════════════════════"
    
    print_status "loading" "Collecting metrics..."
    
    # Get current metrics
    local api_metrics=$(get_api_metrics)
    local gaussos_info=$(get_gaussos_process_info)
    read -r gaussos_pid gaussos_cpu gaussos_mem gaussos_vsz <<< "$gaussos_info"
    
    case $format in
        "json")
            cat > "$output_file" << EOF
{
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "system": {
    "cpu_usage": "$(get_cpu_usage)",
    "memory_usage": "$(get_memory_usage)",
    "disk_usage": "$(get_disk_usage)"
  },
  "gaussos": {
    "pid": "$gaussos_pid",
    "cpu_usage": "$gaussos_cpu",
    "memory_usage": "$gaussos_mem",
    "virtual_memory_mb": "$(echo "scale=1; $gaussos_vsz/1024" | bc)"
  },
  "api_metrics": $api_metrics
}
EOF
            ;;
        "csv")
            cat > "$output_file" << EOF
timestamp,cpu_usage,memory_usage,disk_usage,gaussos_pid,gaussos_cpu,gaussos_mem,gaussos_vsz_mb
$(date -u +"%Y-%m-%dT%H:%M:%SZ"),$(get_cpu_usage),$(get_memory_usage),$(get_disk_usage),$gaussos_pid,$gaussos_cpu,$gaussos_mem,$(echo "scale=1; $gaussos_vsz/1024" | bc)
EOF
            ;;
        *)
            print_status "error" "Unsupported format: $format"
            return 1
            ;;
    esac
    
    print_status "success" "Metrics exported to: $output_file"
}

# Function to show main menu
show_menu() {
    echo -e "${WHITE}Available Commands:${NC}"
    echo ""
    echo -e "  ${GREEN}dashboard${NC}  ${ARROW} Show real-time monitoring dashboard"
    echo -e "  ${GREEN}performance${NC} ${ARROW} Show performance analysis"
    echo -e "  ${GREEN}alerts${NC}     ${ARROW} Show system alerts"
    echo -e "  ${GREEN}export${NC}     ${ARROW} Export metrics (json|csv)"
    echo -e "  ${GREEN}help${NC}       ${ARROW} Show this help message"
    echo ""
    echo -e "Examples:"
    echo -e "  ${YELLOW}./scripts/monitor.sh dashboard${NC}"
    echo -e "  ${YELLOW}./scripts/monitor.sh dashboard 5${NC}"
    echo -e "  ${YELLOW}./scripts/monitor.sh performance${NC}"
    echo -e "  ${YELLOW}./scripts/monitor.sh export json my_metrics.json${NC}"
}

# Main script logic
main() {
    local command=$1
    local arg1=$2
    local arg2=$3
    
    print_header
    
    case $command in
        "dashboard")
            show_dashboard "$arg1"
            ;;
        "performance")
            show_performance_analysis
            ;;
        "alerts")
            show_alerts
            ;;
        "export")
            export_metrics "$arg1" "$arg2"
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
