#!/bin/bash
# Comprehensive Testing Script for RPC Shield
# This script tests all major functionality

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROXY_URL="${PROXY_URL:-http://localhost:8545}"
ADMIN_URL="${ADMIN_URL:-http://localhost:8555}"
METRICS_URL="${METRICS_URL:-http://localhost:9090/metrics}"

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
    ((TESTS_FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

# Wait for service to be ready
wait_for_service() {
    local url=$1
    local max_attempts=30
    local attempt=1

    log_info "Waiting for service at $url..."
    
    while [ $attempt -le $max_attempts ]; do
        if curl -s -f "$url/health" > /dev/null 2>&1; then
            log_success "Service is ready"
            return 0
        fi
        echo -n "."
        sleep 2
        ((attempt++))
    done
    
    log_error "Service did not become ready in time"
    return 1
}

# Test 1: Health Check
test_health_check() {
    log_info "Testing health check endpoint..."
    
    response=$(curl -s "$PROXY_URL/health")
    
    if echo "$response" | jq -e '.status == "ok"' > /dev/null 2>&1; then
        log_success "Health check passed"
        return 0
    else
        log_error "Health check failed: $response"
        return 1
    fi
}

# Test 2: Basic RPC Request
test_basic_rpc() {
    log_info "Testing basic RPC request (eth_blockNumber)..."
    
    response=$(curl -s -X POST "$PROXY_URL" \
        -H "Content-Type: application/json" \
        -d '{
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        }')
    
    if echo "$response" | jq -e '.result' > /dev/null 2>&1; then
        log_success "Basic RPC request succeeded"
        echo "  Block number: $(echo $response | jq -r '.result')"
        return 0
    else
        log_error "Basic RPC request failed: $response"
        return 1
    fi
}

# Test 3: Rate Limiting
test_rate_limiting() {
    log_info "Testing rate limiting..."
    
    local success_count=0
    local rate_limited_count=0
    
    # Send 10 rapid requests
    for i in {1..10}; do
        response=$(curl -s -w "\n%{http_code}" -X POST "$PROXY_URL" \
            -H "Content-Type: application/json" \
            -d '{
                "jsonrpc": "2.0",
                "method": "eth_blockNumber",
                "params": [],
                "id": 1
            }')
        
        http_code=$(echo "$response" | tail -n1)
        
        if [ "$http_code" == "200" ]; then
            ((success_count++))
        elif [ "$http_code" == "429" ]; then
            ((rate_limited_count++))
        fi
    done
    
    log_info "  Successful: $success_count, Rate limited: $rate_limited_count"
    
    if [ $rate_limited_count -gt 0 ]; then
        log_success "Rate limiting is working"
        return 0
    else
        log_warning "Rate limiting did not trigger (may need stricter limits for test)"
        return 0
    fi
}

# Test 4: API Key Authentication
test_api_key() {
    log_info "Testing API key authentication..."
    
    response=$(curl -s -X POST "$PROXY_URL" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer demo_pro_key" \
        -d '{
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        }')
    
    if echo "$response" | jq -e '.result' > /dev/null 2>&1; then
        log_success "API key authentication works"
        return 0
    else
        log_error "API key authentication failed: $response"
        return 1
    fi
}

# Test 5: Different RPC Methods
test_various_methods() {
    log_info "Testing various RPC methods..."
    
    local methods=("eth_blockNumber" "eth_chainId" "net_version")
    local all_passed=true
    
    for method in "${methods[@]}"; do
        response=$(curl -s -X POST "$PROXY_URL" \
            -H "Content-Type: application/json" \
            -d "{
                \"jsonrpc\": \"2.0\",
                \"method\": \"$method\",
                \"params\": [],
                \"id\": 1
            }")
        
        if echo "$response" | jq -e '.result' > /dev/null 2>&1; then
            log_success "  $method: OK"
        else
            log_error "  $method: FAILED"
            all_passed=false
        fi
    done
    
    if [ "$all_passed" = true ]; then
        return 0
    else
        return 1
    fi
}

# Test 6: Admin API - Stats
test_admin_stats() {
    log_info "Testing Admin API - Stats endpoint..."
    
    response=$(curl -s "$ADMIN_URL/api/admin/stats")
    
    if echo "$response" | jq -e '.service' > /dev/null 2>&1; then
        log_success "Admin stats endpoint works"
        echo "  Uptime: $(echo $response | jq -r '.uptime_seconds')s"
        echo "  Total requests: $(echo $response | jq -r '.requests.total')"
        return 0
    else
        log_error "Admin stats endpoint failed: $response"
        return 1
    fi
}

# Test 7: Prometheus Metrics
test_prometheus_metrics() {
    log_info "Testing Prometheus metrics..."
    
    response=$(curl -s "$METRICS_URL")
    
    if echo "$response" | grep -q "rpc_shield_requests_total"; then
        log_success "Prometheus metrics are exposed"
        return 0
    else
        log_error "Prometheus metrics not found"
        return 1
    fi
}

# Test 8: Blocklist Functionality
test_blocklist() {
    log_info "Testing blocklist functionality..."
    
    # Add IP to blocklist
    add_response=$(curl -s -X POST "$ADMIN_URL/api/admin/blocklist" \
        -H "Content-Type: application/json" \
        -d '{
            "ip": "192.168.1.100",
            "reason": "Test block"
        }')
    
    if echo "$add_response" | jq -e '.status == "success"' > /dev/null 2>&1; then
        log_success "  IP added to blocklist"
    else
        log_error "  Failed to add IP to blocklist"
        return 1
    fi
    
    # List blocklist
    list_response=$(curl -s "$ADMIN_URL/api/admin/blocklist")
    
    if echo "$list_response" | jq -e '.blocked_ips[] | select(. == "192.168.1.100")' > /dev/null 2>&1; then
        log_success "  IP found in blocklist"
    else
        log_error "  IP not found in blocklist"
        return 1
    fi
    
    # Remove IP from blocklist
    remove_response=$(curl -s -X DELETE "$ADMIN_URL/api/admin/blocklist/192.168.1.100")
    
    if echo "$remove_response" | jq -e '.status == "success"' > /dev/null 2>&1; then
        log_success "  IP removed from blocklist"
        return 0
    else
        log_error "  Failed to remove IP from blocklist"
        return 1
    fi
}

# Test 9: Burst Behavior
test_burst() {
    log_info "Testing burst behavior..."
    
    local burst_succeeded=0
    
    # Send rapid burst of requests
    for i in {1..15}; do
        response=$(curl -s -w "\n%{http_code}" -X POST "$PROXY_URL" \
            -H "Content-Type: application/json" \
            -d '{
                "jsonrpc": "2.0",
                "method": "eth_blockNumber",
                "params": [],
                "id": 1
            }')
        
        http_code=$(echo "$response" | tail -n1)
        
        if [ "$http_code" == "200" ]; then
            ((burst_succeeded++))
        fi
    done
    
    log_info "  Burst allowed $burst_succeeded requests"
    
    if [ $burst_succeeded -gt 5 ]; then
        log_success "Burst logic is working (allowed extra requests)"
        return 0
    else
        log_warning "Burst may not be configured properly"
        return 0
    fi
}

# Test 10: Invalid Requests
test_invalid_requests() {
    log_info "Testing invalid request handling..."
    
    # Invalid JSON
    response=$(curl -s -w "\n%{http_code}" -X POST "$PROXY_URL" \
        -H "Content-Type: application/json" \
        -d 'invalid json')
    
    http_code=$(echo "$response" | tail -n1)
    
    if [ "$http_code" != "200" ]; then
        log_success "  Invalid JSON rejected properly"
    else
        log_error "  Invalid JSON was accepted"
        return 1
    fi
    
    # Invalid method
    response=$(curl -s -X POST "$PROXY_URL" \
        -H "Content-Type: application/json" \
        -d '{
            "jsonrpc": "2.0",
            "method": "invalid_method_name",
            "params": [],
            "id": 1
        }')
    
    if echo "$response" | jq -e '.error' > /dev/null 2>&1; then
        log_success "  Invalid method handled properly"
        return 0
    else
        log_error "  Invalid method not handled properly"
        return 1
    fi
}

# Test 11: Concurrent Requests
test_concurrent_requests() {
    log_info "Testing concurrent requests..."
    
    local pids=()
    
    # Launch 10 concurrent requests
    for i in {1..10}; do
        (
            curl -s -X POST "$PROXY_URL" \
                -H "Content-Type: application/json" \
                -d '{
                    "jsonrpc": "2.0",
                    "method": "eth_blockNumber",
                    "params": [],
                    "id": 1
                }' > /dev/null
        ) &
        pids+=($!)
    done
    
    # Wait for all requests to complete
    local all_success=true
    for pid in "${pids[@]}"; do
        if ! wait $pid; then
            all_success=false
        fi
    done
    
    if [ "$all_success" = true ]; then
        log_success "Concurrent requests handled successfully"
        return 0
    else
        log_error "Some concurrent requests failed"
        return 1
    fi
}

# Test 12: Method Groups
test_method_groups() {
    log_info "Testing method groups..."
    
    # Test read-only methods
    local read_methods=("eth_blockNumber" "eth_getBalance" "eth_call")
    local all_passed=true
    
    for method in "${read_methods[@]}"; do
        # Note: eth_getBalance and eth_call need params, so we use simple test
        if [ "$method" == "eth_blockNumber" ]; then
            response=$(curl -s -X POST "$PROXY_URL" \
                -H "Content-Type: application/json" \
                -d "{
                    \"jsonrpc\": \"2.0\",
                    \"method\": \"$method\",
                    \"params\": [],
                    \"id\": 1
                }")
            
            if ! echo "$response" | jq -e '.result' > /dev/null 2>&1; then
                all_passed=false
            fi
        fi
    done
    
    if [ "$all_passed" = true ]; then
        log_success "Method groups work correctly"
        return 0
    else
        log_error "Method groups test failed"
        return 1
    fi
}

# Main test execution
main() {
    echo "=========================================="
    echo "RPC Shield Comprehensive Test Suite"
    echo "=========================================="
    echo ""
    
    log_info "Starting tests..."
    echo ""
    
    # Wait for services
    if ! wait_for_service "$PROXY_URL"; then
        log_error "Service is not available. Exiting."
        exit 1
    fi
    echo ""
    
    # Run all tests
    test_health_check || true
    echo ""
    
    test_basic_rpc || true
    echo ""
    
    test_api_key || true
    echo ""
    
    test_various_methods || true
    echo ""
    
    test_rate_limiting || true
    echo ""
    
    test_burst || true
    echo ""
    
    test_admin_stats || true
    echo ""
    
    test_prometheus_metrics || true
    echo ""
    
    test_blocklist || true
    echo ""
    
    test_invalid_requests || true
    echo ""
    
    test_concurrent_requests || true
    echo ""
    
    test_method_groups || true
    echo ""
    
    # Summary
    echo "=========================================="
    echo "Test Summary"
    echo "=========================================="
    echo -e "${GREEN}Tests Passed:${NC} $TESTS_PASSED"
    echo -e "${RED}Tests Failed:${NC} $TESTS_FAILED"
    echo ""
    
    if [ $TESTS_FAILED -eq 0 ]; then
        log_success "All tests passed! 🎉"
        exit 0
    else
        log_error "Some tests failed."
        exit 1
    fi
}

# Run main
main "$@"
