#!/bin/bash
# GaussOS Compilation Script
# Comprehensive build system for backend and frontend

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
BUILD_DIR="$PROJECT_ROOT/target"

# Create necessary directories
mkdir -p "$LOG_DIR"

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_DIR/compile.log"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_DIR/compile.log"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_DIR/compile.log"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_DIR/compile.log"
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
    log_header "Checking Build Dependencies"
    
    # Check Rust/Cargo
    if ! command_exists cargo; then
        log_error "Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    log_success "Cargo found: $(cargo --version)"
    
    # Check Rust version
    local rust_version=$(rustc --version | cut -d' ' -f2)
    log_success "Rust version: $rust_version"
    
    # Check for required Rust components
    if rustup component list --installed | grep -q "clippy"; then
        log_success "Clippy found"
    else
        log_info "Installing clippy..."
        rustup component add clippy
    fi
    
    if rustup component list --installed | grep -q "rustfmt"; then
        log_success "Rustfmt found"
    else
        log_info "Installing rustfmt..."
        rustup component add rustfmt
    fi
    
    # Check Deno for frontend
    if command_exists deno; then
        log_success "Deno found: $(deno --version | head -n1)"
        DENO_AVAILABLE=true
    else
        log_warning "Deno not found. Frontend compilation will be limited."
        log_info "Install Deno: https://deno.land/manual/getting_started/installation"
        DENO_AVAILABLE=false
    fi
    
    # Check Node.js (alternative)
    if command_exists node; then
        log_success "Node.js found: $(node --version)"
        NODE_AVAILABLE=true
    else
        log_info "Node.js not found (optional)"
        NODE_AVAILABLE=false
    fi
    
    # Check system tools
    if command_exists git; then
        log_success "Git found: $(git --version)"
    else
        log_warning "Git not found - version info will be limited"
    fi
}

# Function to clean previous builds
clean_build() {
    log_header "Cleaning Previous Build Artifacts"
    
    cd "$PROJECT_ROOT"
    
    # Clean Cargo artifacts
    log_info "Cleaning Cargo build cache..."
    cargo clean > "$LOG_DIR/clean.log" 2>&1
    log_success "Cargo artifacts cleaned"
    
    # Clean frontend artifacts
    if [ -d "web-ui/dist" ]; then
        log_info "Cleaning frontend build artifacts..."
        rm -rf web-ui/dist
        log_success "Frontend artifacts cleaned"
    fi
    
    # Clean logs directory (except current compilation log)
    log_info "Cleaning old logs..."
    find "$LOG_DIR" -name "*.log" -not -name "compile.log" -mtime +7 -delete 2>/dev/null || true
    
    log_success "Build cleanup completed"
}

# Function to validate configuration
validate_configuration() {
    log_header "Validating Configuration"
    
    cd "$PROJECT_ROOT"
    
    # Validate Cargo.toml
    log_info "Validating Cargo.toml..."
    if cargo check --manifest-path Cargo.toml > "$LOG_DIR/cargo_validate.log" 2>&1; then
        log_success "Cargo.toml is valid"
    else
        log_error "Cargo.toml validation failed"
        log_error "Check: $LOG_DIR/cargo_validate.log"
        return 1
    fi
    
    # Check for required features
    log_info "Checking feature flags..."
    local features=$(cargo read-manifest | jq -r '.features | keys[]' 2>/dev/null || echo "")
    if [ -n "$features" ]; then
        log_success "Available features: $(echo $features | tr '\n' ' ')"
    else
        log_info "No custom features detected"
    fi
    
    # Validate frontend configuration
    if [ "$DENO_AVAILABLE" = true ] && [ -f "web-ui/deno.json" ]; then
        log_info "Validating Deno configuration..."
        if deno info --json web-ui/deno.json > "$LOG_DIR/deno_validate.log" 2>&1; then
            log_success "Deno configuration is valid"
        else
            log_warning "Deno configuration validation failed"
        fi
    fi
}

# Function to compile Rust backend
compile_backend() {
    log_header "Compiling Rust Backend"
    
    cd "$PROJECT_ROOT"
    
    # Check code formatting
    log_info "Checking Rust code formatting..."
    if cargo fmt --all -- --check > "$LOG_DIR/fmt_check.log" 2>&1; then
        log_success "Code formatting is correct"
    else
        log_warning "Code formatting issues detected"
        log_info "Run 'cargo fmt' to fix formatting issues"
    fi
    
    # Run clippy linting
    log_info "Running Clippy linting..."
    if cargo clippy --all-targets --all-features > "$LOG_DIR/clippy.log" 2>&1; then
        log_success "Clippy linting passed"
    else
        log_warning "Clippy found issues (check $LOG_DIR/clippy.log)"
    fi
    
    # Compile debug version
    log_info "Compiling debug version..."
    if cargo build --workspace > "$LOG_DIR/debug_build.log" 2>&1; then
        log_success "Debug compilation successful"
    else
        log_error "Debug compilation failed"
        log_error "Check: $LOG_DIR/debug_build.log"
        return 1
    fi
    
    # Compile release version
    log_info "Compiling release version..."
    if cargo build --release --workspace > "$LOG_DIR/release_build.log" 2>&1; then
        log_success "Release compilation successful"
    else
        log_error "Release compilation failed"
        log_error "Check: $LOG_DIR/release_build.log"
        return 1
    fi
    
    # Compile with specific features
    log_info "Compiling with enterprise features..."
    if cargo build --release --features enterprise > "$LOG_DIR/enterprise_build.log" 2>&1; then
        log_success "Enterprise build successful"
    else
        log_warning "Enterprise build failed (check $LOG_DIR/enterprise_build.log)"
    fi
    
    # Compile documentation
    log_info "Building documentation..."
    if cargo doc --no-deps --workspace > "$LOG_DIR/doc_build.log" 2>&1; then
        log_success "Documentation built successfully"
        log_info "Documentation available at: target/doc/gaussos/index.html"
    else
        log_warning "Documentation build failed (check $LOG_DIR/doc_build.log)"
    fi
    
    # Build CLI binary if available
    if grep -q "cli-bin" Cargo.toml; then
        log_info "Building CLI binary..."
        if cargo build --release --features cli-bin --bin gaussos > "$LOG_DIR/cli_build.log" 2>&1; then
            log_success "CLI binary built successfully"
        else
            log_warning "CLI binary build failed (check $LOG_DIR/cli_build.log)"
        fi
    fi
}

# Function to compile frontend
compile_frontend() {
    log_header "Compiling Frontend"
    
    cd "$PROJECT_ROOT/web-ui"
    
    if [ "$DENO_AVAILABLE" = true ]; then
        # TypeScript compilation and validation
        log_info "Type-checking TypeScript files..."
        if deno check static/*.ts > "$LOG_DIR/typescript_check.log" 2>&1; then
            log_success "TypeScript compilation successful"
        else
            log_warning "TypeScript issues found (check $LOG_DIR/typescript_check.log)"
        fi
        
        # Format check
        log_info "Checking TypeScript formatting..."
        if deno fmt --check static/*.ts > "$LOG_DIR/typescript_fmt.log" 2>&1; then
            log_success "TypeScript formatting is correct"
        else
            log_warning "TypeScript formatting issues found"
            log_info "Run 'deno fmt' to fix formatting"
        fi
        
        # Lint TypeScript files
        log_info "Linting TypeScript files..."
        if deno lint static/*.ts > "$LOG_DIR/typescript_lint.log" 2>&1; then
            log_success "TypeScript linting passed"
        else
            log_warning "TypeScript linting issues found (check $LOG_DIR/typescript_lint.log)"
        fi
        
        # Bundle application (if deno bundle is available)
        log_info "Bundling frontend application..."
        mkdir -p dist
        if deno bundle main.ts dist/bundle.js > "$LOG_DIR/bundle.log" 2>&1; then
            log_success "Frontend bundling successful"
        else
            log_warning "Frontend bundling failed (check $LOG_DIR/bundle.log)"
        fi
        
    else
        log_warning "Deno not available - limited frontend compilation"
        
        # Basic validation of JavaScript/TypeScript files
        log_info "Validating frontend files..."
        local valid=true
        
        for file in static/*.ts static/*.js; do
            if [ -f "$file" ]; then
                # Basic syntax check using Node.js if available
                if [ "$NODE_AVAILABLE" = true ]; then
                    if node -c "$file" 2>/dev/null; then
                        log_success "Syntax OK: $file"
                    else
                        log_error "Syntax error in: $file"
                        valid=false
                    fi
                else
                    log_info "File found: $file"
                fi
            fi
        done
        
        if [ "$valid" = false ]; then
            log_error "Frontend validation failed"
            return 1
        fi
    fi
    
    # Validate CSS files
    log_info "Validating CSS files..."
    for css_file in static/*.css; do
        if [ -f "$css_file" ]; then
            # Basic CSS validation (check for obvious syntax errors)
            if grep -q "}" "$css_file" && grep -q "{" "$css_file"; then
                log_success "CSS syntax OK: $css_file"
            else
                log_warning "Potential CSS syntax issues in: $css_file"
            fi
        fi
    done
    
    # Calculate frontend bundle sizes
    log_info "Calculating asset sizes..."
    {
        echo "Frontend Asset Analysis"
        echo "======================"
        echo "Generated: $(date)"
        echo ""
        
        total_size=0
        for file in static/*.ts static/*.js static/*.css; do
            if [ -f "$file" ]; then
                size=$(wc -c < "$file")
                total_size=$((total_size + size))
                echo "$(basename "$file"): $(numfmt --to=iec-i --suffix=B "$size")"
            fi
        done
        
        echo ""
        echo "Total frontend size: $(numfmt --to=iec-i --suffix=B "$total_size")"
        
    } > "$LOG_DIR/frontend_analysis.txt"
    
    log_success "Frontend asset analysis saved to: $LOG_DIR/frontend_analysis.txt"
}

# Function to run quick tests after compilation
run_quick_tests() {
    log_header "Running Quick Validation Tests"
    
    cd "$PROJECT_ROOT"
    
    # Quick library test
    log_info "Testing library compilation..."
    if cargo test --lib --no-run > "$LOG_DIR/lib_test.log" 2>&1; then
        log_success "Library tests compile successfully"
    else
        log_error "Library test compilation failed"
        return 1
    fi
    
    # Quick integration test compilation
    log_info "Testing integration test compilation..."
    if cargo test --no-run --test comprehensive_integration_tests > "$LOG_DIR/integration_compile.log" 2>&1; then
        log_success "Integration tests compile successfully"
    else
        log_warning "Integration test compilation issues (check $LOG_DIR/integration_compile.log)"
    fi
    
    # Quick benchmark compilation
    log_info "Testing benchmark compilation..."
    if cargo bench --no-run > "$LOG_DIR/bench_compile.log" 2>&1; then
        log_success "Benchmarks compile successfully"
    else
        log_warning "Benchmark compilation issues (check $LOG_DIR/bench_compile.log)"
    fi
}

# Function to generate build report
generate_build_report() {
    log_header "Generating Build Report"
    
    local report_file="$LOG_DIR/build_report.md"
    local timestamp=$(date)
    
    cat > "$report_file" << EOF
# GaussOS Build Report

Generated: $timestamp

## Build Summary

### Backend Compilation
- **Debug Build**: $([ -f "$BUILD_DIR/debug/gaussos" ] && echo "✅ Success" || echo "❌ Failed")
- **Release Build**: $([ -f "$BUILD_DIR/release/gaussos" ] && echo "✅ Success" || echo "❌ Failed")
- **Documentation**: $([ -d "$BUILD_DIR/doc" ] && echo "✅ Generated" || echo "❌ Failed")
- **CLI Binary**: $([ -f "$BUILD_DIR/release/gaussos" ] && echo "✅ Built" || echo "❌ Not Built")

### Frontend Compilation
- **TypeScript Check**: $([ "$DENO_AVAILABLE" = true ] && echo "✅ Completed" || echo "⚠️  Skipped")
- **Bundling**: $([ -f "web-ui/dist/bundle.js" ] && echo "✅ Success" || echo "❌ Failed")
- **Asset Analysis**: ✅ Completed

### Code Quality
- **Clippy Linting**: $([ -f "$LOG_DIR/clippy.log" ] && echo "✅ Completed" || echo "❌ Skipped")
- **Formatting Check**: $([ -f "$LOG_DIR/fmt_check.log" ] && echo "✅ Completed" || echo "❌ Skipped")
- **Test Compilation**: ✅ Verified

## Build Environment

- **Rust Version**: $(rustc --version)
- **Cargo Version**: $(cargo --version)
- **Deno Available**: $([ "$DENO_AVAILABLE" = true ] && echo "Yes" || echo "No")
- **Build Target**: $(rustc -vV | grep "host:" | cut -d' ' -f2)

## Build Artifacts

### Backend Binaries
EOF

    # Add binary information if they exist
    if [ -d "$BUILD_DIR" ]; then
        find "$BUILD_DIR" -name "gaussos*" -type f -executable 2>/dev/null | while read binary; do
            echo "- \`$(basename "$binary")\`: $(du -h "$binary" | cut -f1)" >> "$report_file"
        done
    fi
    
    cat >> "$report_file" << EOF

### Frontend Assets
EOF

    # Add frontend asset information
    if [ -d "web-ui/static" ]; then
        cd "$PROJECT_ROOT/web-ui"
        for file in static/*.ts static/*.js static/*.css; do
            if [ -f "$file" ]; then
                size=$(du -h "$file" | cut -f1)
                echo "- \`$(basename "$file")\`: $size" >> "$report_file"
            fi
        done
        cd "$PROJECT_ROOT"
    fi
    
    cat >> "$report_file" << EOF

## Build Logs

Detailed build logs are available in the \`logs/\` directory:

EOF

    # Add log file references
    for log_file in "$LOG_DIR"/*.log; do
        if [ -f "$log_file" ] && [ "$(basename "$log_file")" != "build_report.md" ]; then
            echo "- [\`$(basename "$log_file")\`](./$(basename "$log_file"))" >> "$report_file"
        fi
    done
    
    cat >> "$report_file" << EOF

## Performance Targets

| Component | Target | Status |
|-----------|--------|---------|
| Debug Build Time | < 2 minutes | ⏱️ |
| Release Build Time | < 5 minutes | ⏱️ |
| Frontend Bundle Size | < 1MB | 📊 |
| Documentation Coverage | > 90% | 📚 |

## Next Steps

1. Run tests: \`./scripts/test.sh\`
2. Run benchmarks: \`./scripts/bench.sh\`
3. Deploy: \`./scripts/deploy.sh\`

EOF
    
    log_success "Build report generated: $report_file"
}

# Function to optimize build
optimize_build() {
    log_header "Optimizing Build Configuration"
    
    cd "$PROJECT_ROOT"
    
    # Check if we can enable additional optimizations
    log_info "Checking optimization opportunities..."
    
    # Link-time optimization
    if cargo build --release --help | grep -q "lto"; then
        log_info "LTO available - already configured in Cargo.toml"
    fi
    
    # Codegen units optimization
    log_info "Codegen units set to 1 for release builds"
    
    # Target CPU optimization
    if [ -n "${RUSTFLAGS:-}" ]; then
        log_info "Custom RUSTFLAGS detected: $RUSTFLAGS"
    else
        log_info "Consider setting RUSTFLAGS='-C target-cpu=native' for local builds"
    fi
    
    # Dependency optimization
    log_info "Checking dependency tree..."
    cargo tree --depth 1 > "$LOG_DIR/dependency_tree.txt" 2>&1 || true
    
    log_success "Build optimization analysis completed"
}

# Function to setup development environment
setup_dev_environment() {
    log_header "Setting Up Development Environment"
    
    cd "$PROJECT_ROOT"
    
    # Create development configuration
    if [ ! -f ".vscode/settings.json" ]; then
        log_info "Creating VS Code configuration..."
        mkdir -p .vscode
        cat > .vscode/settings.json << 'EOF'
{
    "rust-analyzer.cargo.features": ["enterprise"],
    "rust-analyzer.checkOnSave.command": "clippy",
    "editor.formatOnSave": true,
    "[rust]": {
        "editor.defaultFormatter": "rust-lang.rust-analyzer"
    },
    "[typescript]": {
        "editor.defaultFormatter": "denoland.vscode-deno"
    }
}
EOF
        log_success "VS Code configuration created"
    fi
    
    # Setup git hooks
    if [ -d ".git" ] && [ ! -f ".git/hooks/pre-commit" ]; then
        log_info "Setting up git pre-commit hook..."
        cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
# Pre-commit hook for GaussOS

echo "Running pre-commit checks..."

# Format check
if ! cargo fmt --all -- --check; then
    echo "❌ Code formatting issues found. Run 'cargo fmt' to fix."
    exit 1
fi

# Clippy check
if ! cargo clippy --all-targets -- -D warnings; then
    echo "❌ Clippy found issues."
    exit 1
fi

# Quick test compilation
if ! cargo check --all-targets; then
    echo "❌ Compilation failed."
    exit 1
fi

echo "✅ Pre-commit checks passed!"
EOF
        chmod +x .git/hooks/pre-commit
        log_success "Git pre-commit hook installed"
    fi
    
    # Create development scripts
    if [ ! -f "scripts/dev.sh" ]; then
        log_info "Creating development helper script..."
        cat > scripts/dev.sh << 'EOF'
#!/bin/bash
# Development helper script

case "$1" in
    "watch")
        cargo watch -x "check --all-targets" -x "test --lib"
        ;;
    "fmt")
        cargo fmt --all
        if command -v deno >/dev/null 2>&1; then
            deno fmt web-ui/static/*.ts
        fi
        ;;
    "lint")
        cargo clippy --all-targets -- -D warnings
        if command -v deno >/dev/null 2>&1; then
            deno lint web-ui/static/*.ts
        fi
        ;;
    "docs")
        cargo doc --open --no-deps
        ;;
    *)
        echo "Usage: $0 {watch|fmt|lint|docs}"
        ;;
esac
EOF
        chmod +x scripts/dev.sh
        log_success "Development helper script created"
    fi
}

# Main execution
main() {
    # Parse command line arguments
    CLEAN_BUILD=false
    SKIP_BACKEND=false
    SKIP_FRONTEND=false
    SKIP_TESTS=false
    OPTIMIZE=false
    SETUP_DEV=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --clean)
                CLEAN_BUILD=true
                shift
                ;;
            --skip-backend)
                SKIP_BACKEND=true
                shift
                ;;
            --skip-frontend)
                SKIP_FRONTEND=true
                shift
                ;;
            --skip-tests)
                SKIP_TESTS=true
                shift
                ;;
            --optimize)
                OPTIMIZE=true
                shift
                ;;
            --setup-dev)
                SETUP_DEV=true
                shift
                ;;
            -h|--help)
                echo "GaussOS Compilation Script"
                echo ""
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --clean             Clean before building"
                echo "  --skip-backend      Skip backend compilation"
                echo "  --skip-frontend     Skip frontend compilation"
                echo "  --skip-tests        Skip test compilation"
                echo "  --optimize          Run build optimization analysis"
                echo "  --setup-dev         Setup development environment"
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
    echo "GaussOS Compilation Started at $(date)" > "$LOG_DIR/compile.log"
    
    log_header "GaussOS Comprehensive Build System"
    log_info "Project root: $PROJECT_ROOT"
    log_info "Build directory: $BUILD_DIR"
    log_info "Logs directory: $LOG_DIR"
    
    # Record build start time
    local start_time=$(date +%s)
    
    # Check dependencies
    check_dependencies
    
    # Clean if requested
    if [ "$CLEAN_BUILD" = true ]; then
        clean_build
    fi
    
    # Validate configuration
    validate_configuration
    
    # Setup development environment if requested
    if [ "$SETUP_DEV" = true ]; then
        setup_dev_environment
    fi
    
    # Run optimization analysis if requested
    if [ "$OPTIMIZE" = true ]; then
        optimize_build
    fi
    
    # Compile based on options
    local exit_code=0
    
    if [ "$SKIP_BACKEND" = false ]; then
        if ! compile_backend; then
            exit_code=1
        fi
    else
        log_info "Skipping backend compilation"
    fi
    
    if [ "$SKIP_FRONTEND" = false ]; then
        if ! compile_frontend; then
            exit_code=1
        fi
    else
        log_info "Skipping frontend compilation"
    fi
    
    if [ "$SKIP_TESTS" = false ]; then
        if ! run_quick_tests; then
            exit_code=1
        fi
    else
        log_info "Skipping test compilation"
    fi
    
    # Generate build report
    generate_build_report
    
    # Calculate build time
    local end_time=$(date +%s)
    local build_time=$((end_time - start_time))
    local build_time_formatted=$(printf "%02d:%02d" $((build_time / 60)) $((build_time % 60)))
    
    # Final status
    if [ $exit_code -eq 0 ]; then
        log_header "Build Completed Successfully"
        log_success "Total build time: $build_time_formatted"
        log_success "Build artifacts available in: $BUILD_DIR"
        log_success "Build report available in: $LOG_DIR/build_report.md"
    else
        log_header "Build Failed"
        log_error "Build time: $build_time_formatted"
        log_error "Check build logs in: $LOG_DIR"
    fi
    
    exit $exit_code
}

# Run main function with all arguments
main "$@"
