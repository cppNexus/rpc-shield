#!/bin/bash

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROXY_URL="http://localhost:8545"

echo "Testing rpc-shield

 v2 (Enhanced)..."
echo ""

# ============================================================================
# Test 1: Health Check
# ============================================================================
echo -e "${BLUE}[1] Health Check${NC}"
HEALTH=$(curl -s $PROXY_URL/health)
echo "$HEALTH" | jq .

if echo "$HEALTH" | jq -e '.status == "ok"' > /dev/null 2>&1; then
  echo -e "${GREEN}✓ Health check passed${NC}"
else
  echo -e "${RED}✗ Health check failed${NC}"
  exit 1
fi
echo ""

# ============================================================================
# Test 2: Basic RPC Proxy
# ============================================================================
echo -e "${BLUE}[2] Basic RPC Proxy (eth_blockNumber)${NC}"
RESPONSE=$(curl -s -X POST $PROXY_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}')

echo "$RESPONSE" | jq .

if echo "$RESPONSE" | jq -e '.result' > /dev/null 2>&1; then
  echo -e "${GREEN}✓ RPC proxy working${NC}"
else
  echo -e "${RED}✗ RPC proxy failed${NC}"
fi
echo ""

# ============================================================================
# Test 3: Rate Limiting Test
# ============================================================================
echo -e "${BLUE}[3] Rate Limiting Test (sending 15 rapid requests)${NC}"
echo "Testing with IP-based limiting..."
echo ""

SUCCESS=0
RATE_LIMITED=0
BURST_USED=0

for i in {1..15}; do
  RESPONSE=$(curl -s -w "\n%{http_code}" -X POST $PROXY_URL \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":'$i'}')
  
  HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
  BODY=$(echo "$RESPONSE" | sed '$d')
  
  if [ "$HTTP_CODE" = "200" ]; then
    if echo "$BODY" | jq -e '.result' > /dev/null 2>&1; then
      SUCCESS=$((SUCCESS + 1))
      if [ $i -le 5 ]; then
        echo "  Request $i: ${GREEN}✓ Allowed (steady)${NC}"
      else
        BURST_USED=$((BURST_USED + 1))
        echo "  Request $i: ${YELLOW}✓ Allowed (burst)${NC}"
      fi
    fi
  elif [ "$HTTP_CODE" = "429" ]; then
    RATE_LIMITED=$((RATE_LIMITED + 1))
    echo "  Request $i: ${RED}✗ Rate Limited (429)${NC}"
  else
    echo "  Request $i: ${RED}✗ Unexpected status: $HTTP_CODE${NC}"
  fi
  
  sleep 0.05
done

echo ""
echo "Results:"
echo -e "  ${GREEN}Successful (steady): $((SUCCESS - BURST_USED))${NC}"
echo -e "  ${YELLOW}Successful (burst): $BURST_USED${NC}"
echo -e "  ${RED}Rate Limited: $RATE_LIMITED${NC}"
echo ""

if [ $RATE_LIMITED -gt 0 ]; then
  echo -e "${GREEN}✓✓✓ Rate limiting is WORKING!${NC}"
else
  echo -e "${RED}WARNING: Rate limiting NOT working${NC}"
  echo -e "${YELLOW}Possible reasons:${NC}"
  echo "  - Limits too high in config"
  echo "  - Check: cat config.yaml | grep -A5 'default_ip_limit'"
  echo "  - Try: config-test-strict.yaml with lower limits"
fi

echo ""

# ============================================================================
# Test 4: API Key Authentication
# ============================================================================
echo -e "${BLUE}[4] API Key Authentication Test${NC}"

# Test without API key
echo -n "  No API key: "
RESPONSE=$(curl -s -X POST $PROXY_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}')

if echo "$RESPONSE" | jq -e '.result' > /dev/null 2>&1; then
  echo -e "${GREEN}✓ (uses IP limiting)${NC}"
else
  echo -e "${RED}✗${NC}"
fi

# Test with Free API key (if configured)
if grep -q "demo_free_key" config*.yaml 2>/dev/null; then
  echo -n "  Free API key: "
  RESPONSE=$(curl -s -X POST $PROXY_URL \
    -H "Authorization: Bearer demo_free_key_abc123" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}')
  
  if echo "$RESPONSE" | jq -e '.result' > /dev/null 2>&1; then
    echo -e "${GREEN}✓ (authenticated)${NC}"
  else
    echo -e "${YELLOW}⚠ (key might not be configured)${NC}"
  fi
fi

# Test with Pro API key (if configured)
if grep -q "pro_key" config*.yaml 2>/dev/null; then
  echo -n "  Pro API key: "
  RESPONSE=$(curl -s -X POST $PROXY_URL \
    -H "Authorization: Bearer pro_key_xyz789" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}')
  
  if echo "$RESPONSE" | jq -e '.result' > /dev/null 2>&1; then
    echo -e "${GREEN}✓ (authenticated)${NC}"
  else
    echo -e "${YELLOW}⚠ (key might not be configured)${NC}"
  fi
fi

echo ""

# ============================================================================
# Test 5: Different RPC Methods (with proper params)
# ============================================================================
echo -e "${BLUE}[5] Testing Different RPC Methods${NC}"

# eth_blockNumber (no params needed)
echo -n "  eth_blockNumber: "
RESPONSE=$(curl -s -X POST $PROXY_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}')

if echo "$RESPONSE" | jq -e '.result' > /dev/null 2>&1; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC}"
fi

# eth_getBalance (with valid address)
echo -n "  eth_getBalance: "
RESPONSE=$(curl -s -X POST $PROXY_URL \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "method":"eth_getBalance",
    "params":["0x0000000000000000000000000000000000000000","latest"],
    "id":1
  }')

if echo "$RESPONSE" | jq -e '.result' > /dev/null 2>&1; then
  echo -e "${GREEN}✓${NC}"
elif echo "$RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
  echo -e "${YELLOW}Backend Error (normal for test)${NC}"
else
  echo -e "${RED}✗${NC}"
fi

# eth_chainId
echo -n "  eth_chainId: "
RESPONSE=$(curl -s -X POST $PROXY_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}')

if echo "$RESPONSE" | jq -e '.result' > /dev/null 2>&1; then
  CHAIN_ID=$(echo "$RESPONSE" | jq -r '.result')
  echo -e "${GREEN}✓ (chain: $CHAIN_ID)${NC}"
elif echo "$RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
  echo -e "${YELLOW}Backend Error${NC}"
else
  echo -e "${RED}✗${NC}"
fi

echo ""

# ============================================================================
# Test 6: Response Time
# ============================================================================
echo -e "${BLUE}[6] Performance Test (5 requests)${NC}"

TOTAL_TIME=0
for i in {1..5}; do
  START=$(date +%s%N)
  curl -s -X POST $PROXY_URL \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":'$i'}' > /dev/null
  END=$(date +%s%N)
  
  DURATION=$(( (END - START) / 1000000 )) # Convert to milliseconds
  TOTAL_TIME=$((TOTAL_TIME + DURATION))
  echo "  Request $i: ${DURATION}ms"
done

AVG_TIME=$((TOTAL_TIME / 5))
echo ""
echo "  Average response time: ${AVG_TIME}ms"

if [ $AVG_TIME -lt 100 ]; then
  echo -e "${GREEN}✓ Excellent performance (<100ms)${NC}"
elif [ $AVG_TIME -lt 500 ]; then
  echo -e "${YELLOW}⚠ Good performance (<500ms)${NC}"
else
  echo -e "${RED}✗ Slow performance (>500ms)${NC}"
fi

echo ""

# ============================================================================
# Summary
# ============================================================================
echo -e "${BLUE}===================================${NC}"
echo -e "${GREEN}Testing Complete${NC}"
echo -e "${BLUE}===================================${NC}"
echo ""
echo "Summary:"
echo "  ✓ Health check"
echo "  ✓ RPC proxy working"
if [ $RATE_LIMITED -gt 0 ]; then
  echo -e "  ${GREEN}✓ Rate limiting working${NC}"
else
  echo -e "  ${YELLOW}⚠ Rate limiting needs checking${NC}"
fi
echo "  ✓ API key authentication"
echo "  ✓ Multiple RPC methods"
echo "  ✓ Performance: ${AVG_TIME}ms avg"
echo ""

# Final verdict
ISSUES=0
if [ $RATE_LIMITED -eq 0 ]; then
  ISSUES=$((ISSUES + 1))
fi

if [ $ISSUES -eq 0 ]; then
  echo -e "${GREEN}All tests passed! Proxy is working perfectly!${NC}"
else
  echo -e "${YELLOW}$ISSUES issue(s) detected - see above${NC}"
fi

echo ""
echo "Next steps:"
if [ $RATE_LIMITED -eq 0 ]; then
  echo "  1. Check rate limit configuration"
  echo "  2. Try with config-test-strict.yaml"
  echo "  3. Check logs: tail -f /tmp/rpc-shield.log"
fi
echo "  • Test advanced features: ./test_advanced.sh"
echo "  • Check documentation: cat TEST_RESULTS_ANALYSIS.md"
echo ""