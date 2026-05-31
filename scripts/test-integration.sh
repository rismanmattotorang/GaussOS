#!/bin/bash

# GaussOS Integration Test Script
# Tests the integration between backend and frontend services

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BACKEND_URL="http://localhost:8080"
FRONTEND_URL="http://localhost:3000"
TIMEOUT=10

# Logging functions
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

# Test function
test_endpoint() {
    local url=$1
    local name=$2
    local expected_status=${3:-200}
    
    log "Testing $name endpoint: $url"
    
    if curl -s -o /dev/null -w "%{http_code}" --max-time $TIMEOUT "$url" | grep -q "$expected_status"; then
        log "✅ $name endpoint is working (HTTP $expected_status)"
        return 0
    else
        error "❌ $name endpoint failed or returned unexpected status"
        return 1
    fi
}

# Test API response
test_api_response() {
    local url=$1
    local name=$2
    local expected_field=$3
    
    log "Testing $name API response: $url"
    
    response=$(curl -s --max-time $TIMEOUT "$url" 2>/dev/null || echo "{}")
    
    if echo "$response" | jq -e ".$expected_field" >/dev/null 2>&1; then
        log "✅ $name API response is valid (contains $expected_field)"
        return 0
    else
        error "❌ $name API response is invalid or missing $expected_field"
        warn "Response: $response"
        return 1
    fi
}

# Check if services are running
check_services() {
    log "Checking if services are running..."
    
    # Check backend
    if curl -s --max-time 5 "$BACKEND_URL/health" >/dev/null 2>&1; then
        log "✅ Backend service is running on $BACKEND_URL"
    else
        error "❌ Backend service is not running on $BACKEND_URL"
        warn "Please start the backend with: cargo run --bin gaussos server"
        return 1
    fi
    
    # Check frontend
    if curl -s --max-time 5 "$FRONTEND_URL" >/dev/null 2>&1; then
        log "✅ Frontend service is running on $FRONTEND_URL"
    else
        error "❌ Frontend service is not running on $FRONTEND_URL"
        warn "Please start the frontend with: cd web-ui && ./start.sh"
        return 1
    fi
    
    return 0
}

# Run integration tests
run_tests() {
    log "=== Starting GaussOS Integration Tests ==="
    
    # Check if services are running
    if ! check_services; then
        error "Integration tests failed: Services not running"
        exit 1
    fi
    
    local tests_passed=0
    local tests_failed=0
    
    # Test backend endpoints
    log "Testing backend endpoints..."
    
    if test_endpoint "$BACKEND_URL/health" "Backend Health"; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    if test_endpoint "$BACKEND_URL/metrics" "Backend Metrics"; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    if test_api_response "$BACKEND_URL/health" "Backend Health" "status"; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    # Test frontend endpoints
    log "Testing frontend endpoints..."
    
    if test_endpoint "$FRONTEND_URL" "Frontend Main"; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    if test_endpoint "$FRONTEND_URL/api/status" "Frontend API Status"; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    if test_endpoint "$FRONTEND_URL/api/metrics" "Frontend API Metrics"; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    # Test frontend-backend integration
    log "Testing frontend-backend integration..."
    
    # Test if frontend can proxy backend requests
    if test_api_response "$FRONTEND_URL/api/status" "Frontend-Backend Integration" "status"; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    # Test CORS
    log "Testing CORS configuration..."
    cors_response=$(curl -s -H "Origin: $FRONTEND_URL" -H "Access-Control-Request-Method: GET" \
        -H "Access-Control-Request-Headers: Content-Type" \
        -X OPTIONS --max-time $TIMEOUT "$BACKEND_URL/health" 2>/dev/null || echo "")
    
    if echo "$cors_response" | grep -q "Access-Control-Allow-Origin"; then
        log "✅ CORS is properly configured"
        ((tests_passed++))
    else
        warn "⚠️  CORS configuration may need attention"
        ((tests_failed++))
    fi
    
    # Summary
    log "=== Integration Test Summary ==="
    log "Tests passed: $tests_passed"
    log "Tests failed: $tests_failed"
    log "Total tests: $((tests_passed + tests_failed))"
    
    if [ $tests_failed -eq 0 ]; then
        log "🎉 All integration tests passed!"
        exit 0
    else
        error "❌ Some integration tests failed"
        exit 1
    fi
}

# Main execution
main() {
    # Check dependencies
    if ! command -v curl >/dev/null 2>&1; then
        error "curl is required but not installed"
        exit 1
    fi
    
    if ! command -v jq >/dev/null 2>&1; then
        warn "jq is not installed. Some tests may be skipped."
    fi
    
    run_tests
}

# Handle script interruption
cleanup() {
    log "Integration test script interrupted"
    exit 1
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

# Run main function
main "$@"
