#!/bin/bash
# GaussOS Test Runner Script
# Comprehensive testing for backend and frontend components

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
LOG_DIR="$PROJECT_ROOT/logs"
TEST_RESULTS_DIR="$PROJECT_ROOT/test-results"

# Create necessary directories
mkdir -p "$LOG_DIR" "$TEST_RESULTS_DIR"

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_DIR/test.log"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_DIR/test.log"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_DIR/test.log"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_DIR/test.log"
}

log_header() {
    echo -e "\n${WHITE}========================================${NC}"
    echo -e "${WHITE} $1${NC}"
    echo -e "${WHITE}========================================${NC}\n"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check dependencies
check_dependencies() {
    log_header "Checking Dependencies"
    
    # Check Rust/Cargo
    if ! command_exists cargo; then
        log_error "Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    log_success "Cargo found: $(cargo --version)"
    
    # Check Deno for frontend
    if ! command_exists deno; then
        log_warning "Deno not found. Frontend tests will be skipped."
        log_info "Install Deno: https://deno.land/manual/getting_started/installation"
        DENO_AVAILABLE=false
    else
        log_success "Deno found: $(deno --version | head -n1)"
        DENO_AVAILABLE=true
    fi
    
    # Check Node.js (alternative for frontend)
    if ! command_exists node; then
        log_warning "Node.js not found. Some frontend tests may be limited."
    else
        log_success "Node.js found: $(node --version)"
    fi
}

# Function to run Rust backend tests
run_backend_tests() {
    log_header "Running Backend Tests"
    
    cd "$PROJECT_ROOT"
    
    # Lint check
    log_info "Running Rust linting..."
    if cargo clippy --all-targets --all-features -- -D warnings > "$TEST_RESULTS_DIR/clippy.log" 2>&1; then
        log_success "Clippy linting passed"
    else
        log_warning "Clippy found issues (see test-results/clippy.log)"
    fi
    
    # Format check
    log_info "Checking Rust formatting..."
    if cargo fmt --all -- --check > "$TEST_RESULTS_DIR/fmt.log" 2>&1; then
        log_success "Rust formatting is correct"
    else
        log_warning "Rust formatting issues found (see test-results/fmt.log)"
        log_info "Run 'cargo fmt' to fix formatting"
    fi
    
    # Unit tests
    log_info "Running unit tests..."
    if cargo test --lib --bins > "$TEST_RESULTS_DIR/unit_tests.log" 2>&1; then
        log_success "Unit tests passed"
    else
        log_error "Unit tests failed (see test-results/unit_tests.log)"
        return 1
    fi
    
    # Integration tests
    log_info "Running integration tests..."
    if cargo test --test comprehensive_integration_tests > "$TEST_RESULTS_DIR/integration_tests.log" 2>&1; then
        log_success "Integration tests passed"
    else
        log_error "Integration tests failed (see test-results/integration_tests.log)"
        return 1
    fi
    
    # Frontend-backend integration tests
    log_info "Running frontend-backend integration tests..."
    if cargo test --test frontend_backend_integration_tests > "$TEST_RESULTS_DIR/frontend_backend_tests.log" 2>&1; then
        log_success "Frontend-backend integration tests passed"
    else
        log_error "Frontend-backend integration tests failed (see test-results/frontend_backend_tests.log)"
        return 1
    fi
    
    # Doc tests
    log_info "Running documentation tests..."
    if cargo test --doc > "$TEST_RESULTS_DIR/doc_tests.log" 2>&1; then
        log_success "Documentation tests passed"
    else
        log_warning "Documentation tests failed (see test-results/doc_tests.log)"
    fi
    
    # Test coverage (if available)
    if command_exists cargo-tarpaulin; then
        log_info "Running test coverage analysis..."
        cargo tarpaulin --out Html --output-dir "$TEST_RESULTS_DIR" > "$TEST_RESULTS_DIR/coverage.log" 2>&1 || true
        log_success "Coverage report generated in test-results/"
    else
        log_info "Install cargo-tarpaulin for coverage: cargo install cargo-tarpaulin"
    fi
}

# Function to run frontend tests
run_frontend_tests() {
    log_header "Running Frontend Tests"
    
    cd "$PROJECT_ROOT/web-ui"
    
    if [ "$DENO_AVAILABLE" = true ]; then
        # Deno-based tests
        log_info "Running Deno tests..."
        if deno test --allow-net --allow-read > "$TEST_RESULTS_DIR/deno_tests.log" 2>&1; then
            log_success "Deno tests passed"
        else
            log_warning "Deno tests failed or not found (see test-results/deno_tests.log)"
        fi
        
        # Deno linting
        log_info "Running Deno linting..."
        if deno lint > "$TEST_RESULTS_DIR/deno_lint.log" 2>&1; then
            log_success "Deno linting passed"
        else
            log_warning "Deno linting issues found (see test-results/deno_lint.log)"
        fi
        
        # Deno formatting check
        log_info "Checking Deno formatting..."
        if deno fmt --check > "$TEST_RESULTS_DIR/deno_fmt.log" 2>&1; then
            log_success "Deno formatting is correct"
        else
            log_warning "Deno formatting issues found (see test-results/deno_fmt.log)"
        fi
        
        # TypeScript type checking
        log_info "Running TypeScript type checking..."
        if deno check static/*.ts > "$TEST_RESULTS_DIR/typescript_check.log" 2>&1; then
            log_success "TypeScript type checking passed"
        else
            log_warning "TypeScript type checking failed (see test-results/typescript_check.log)"
        fi
    else
        log_warning "Skipping Deno-based frontend tests (Deno not available)"
    fi
    
    # Manual frontend validation
    log_info "Validating frontend files..."
    local frontend_valid=true
    
    # Check main TypeScript files exist and are valid
    for file in "main.ts" "static/app.ts" "static/enhanced-dashboard.ts" "static/api-client.ts"; do
        if [ -f "$file" ]; then
            log_success "Found: $file"
        else
            log_error "Missing: $file"
            frontend_valid=false
        fi
    done
    
    # Check CSS files
    for file in "static/styles.css" "static/themes.css" "static/components.css"; do
        if [ -f "$file" ]; then
            log_success "Found: $file"
        else
            log_warning "Missing CSS file: $file"
        fi
    done
    
    if [ "$frontend_valid" = true ]; then
        log_success "Frontend file validation passed"
    else
        log_error "Frontend file validation failed"
        return 1
    fi
}

# Function to run performance tests
run_performance_tests() {
    log_header "Running Performance Tests"
    
    cd "$PROJECT_ROOT"
    
    # Run benchmarks
    log_info "Running benchmarks..."
    if cargo bench > "$TEST_RESULTS_DIR/benchmarks.log" 2>&1; then
        log_success "Benchmarks completed"
    else
        log_warning "Benchmarks failed or incomplete (see test-results/benchmarks.log)"
    fi
    
    # Performance regression tests
    log_info "Running performance regression tests..."
    if cargo test --release performance > "$TEST_RESULTS_DIR/performance_tests.log" 2>&1; then
        log_success "Performance tests passed"
    else
        log_warning "Performance tests failed (see test-results/performance_tests.log)"
    fi
}

# Function to validate configuration
validate_configuration() {
    log_header "Validating Configuration"
    
    cd "$PROJECT_ROOT"
    
    # Check Cargo.toml
    if cargo check --manifest-path Cargo.toml > "$TEST_RESULTS_DIR/cargo_check.log" 2>&1; then
        log_success "Cargo.toml is valid"
    else
        log_error "Cargo.toml validation failed (see test-results/cargo_check.log)"
        return 1
    fi
    
    # Check config file
    if [ -f "config.toml" ]; then
        log_success "Configuration file found: config.toml"
    else
        log_warning "Configuration file not found: config.toml"
    fi
    
    # Check frontend configuration
    if [ -f "web-ui/deno.json" ]; then
        log_success "Frontend configuration found: web-ui/deno.json"
    else
        log_warning "Frontend configuration not found: web-ui/deno.json"
    fi
}

# Function to generate test report
generate_test_report() {
    log_header "Generating Test Report"
    
    local report_file="$TEST_RESULTS_DIR/test_report.md"
    
    cat > "$report_file" << EOF
# GaussOS Test Report

Generated on: $(date)

## Test Summary

### Backend Tests
- **Unit Tests**: $(grep -c "test result:" "$TEST_RESULTS_DIR/unit_tests.log" 2>/dev/null || echo "N/A")
- **Integration Tests**: $(grep -c "test result:" "$TEST_RESULTS_DIR/integration_tests.log" 2>/dev/null || echo "N/A")
- **Frontend-Backend Tests**: $(grep -c "test result:" "$TEST_RESULTS_DIR/frontend_backend_tests.log" 2>/dev/null || echo "N/A")

### Frontend Tests
- **Deno Tests**: $([ -f "$TEST_RESULTS_DIR/deno_tests.log" ] && echo "Completed" || echo "Skipped")
- **TypeScript Check**: $([ -f "$TEST_RESULTS_DIR/typescript_check.log" ] && echo "Completed" || echo "Skipped")

### Performance Tests
- **Benchmarks**: $([ -f "$TEST_RESULTS_DIR/benchmarks.log" ] && echo "Completed" || echo "Skipped")

### Code Quality
- **Clippy Linting**: $([ -f "$TEST_RESULTS_DIR/clippy.log" ] && echo "Completed" || echo "Skipped")
- **Formatting**: $([ -f "$TEST_RESULTS_DIR/fmt.log" ] && echo "Checked" || echo "Skipped")

## Detailed Logs

All detailed logs are available in the \`test-results/\` directory:

EOF

    # Add log file references
    for log_file in "$TEST_RESULTS_DIR"/*.log; do
        if [ -f "$log_file" ]; then
            local filename=$(basename "$log_file")
            echo "- [\`$filename\`](./$filename)" >> "$report_file"
        fi
    done
    
    log_success "Test report generated: $report_file"
}

# Function to clean up test artifacts
cleanup() {
    log_info "Cleaning up test artifacts..."
    
    cd "$PROJECT_ROOT"
    
    # Clean Cargo build artifacts
    cargo clean > /dev/null 2>&1 || true
    
    # Clean frontend build artifacts (if any)
    if [ -d "web-ui/dist" ]; then
        rm -rf web-ui/dist
    fi
    
    log_success "Cleanup completed"
}

# Main execution
main() {
    # Parse command line arguments
    SKIP_FRONTEND=false
    SKIP_BACKEND=false
    SKIP_PERFORMANCE=false
    CLEANUP_AFTER=false
    VERBOSE=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-frontend)
                SKIP_FRONTEND=true
                shift
                ;;
            --skip-backend)
                SKIP_BACKEND=true
                shift
                ;;
            --skip-performance)
                SKIP_PERFORMANCE=true
                shift
                ;;
            --cleanup)
                CLEANUP_AFTER=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            -h|--help)
                echo "GaussOS Test Runner"
                echo ""
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --skip-frontend     Skip frontend tests"
                echo "  --skip-backend      Skip backend tests"
                echo "  --skip-performance  Skip performance tests"
                echo "  --cleanup           Clean up after tests"
                echo "  --verbose           Verbose output"
                echo "  -h, --help          Show this help"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Initialize logging
    echo "GaussOS Test Runner Started at $(date)" > "$LOG_DIR/test.log"
    
    log_header "GaussOS Comprehensive Test Suite"
    log_info "Project root: $PROJECT_ROOT"
    log_info "Logs directory: $LOG_DIR"
    log_info "Results directory: $TEST_RESULTS_DIR"
    
    # Check dependencies
    check_dependencies
    
    # Validate configuration
    validate_configuration
    
    # Run tests based on options
    local exit_code=0
    
    if [ "$SKIP_BACKEND" = false ]; then
        if ! run_backend_tests; then
            exit_code=1
        fi
    else
        log_info "Skipping backend tests"
    fi
    
    if [ "$SKIP_FRONTEND" = false ]; then
        if ! run_frontend_tests; then
            exit_code=1
        fi
    else
        log_info "Skipping frontend tests"
    fi
    
    if [ "$SKIP_PERFORMANCE" = false ]; then
        run_performance_tests
    else
        log_info "Skipping performance tests"
    fi
    
    # Generate report
    generate_test_report
    
    # Cleanup if requested
    if [ "$CLEANUP_AFTER" = true ]; then
        cleanup
    fi
    
    # Final status
    if [ $exit_code -eq 0 ]; then
        log_header "All Tests Completed Successfully"
        log_success "Test results available in: $TEST_RESULTS_DIR"
    else
        log_header "Some Tests Failed"
        log_error "Check test results in: $TEST_RESULTS_DIR"
    fi
    
    exit $exit_code
}

# Run main function with all arguments
main "$@"
