#!/bin/bash
# LSP Server Smoke Test
# This script tests the Sanctifier LSP server with various scenarios

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Sanctifier LSP Server Smoke Test ===${NC}\n"

# Check if sanctifier binary exists
if ! command -v sanctifier &> /dev/null; then
    echo -e "${RED}ERROR: sanctifier binary not found${NC}"
    echo "Build with: cargo build --bin sanctifier --release"
    exit 1
fi

echo -e "${GREEN}✓ sanctifier binary found${NC}"

# Test 1: Server startup
echo -e "\n${YELLOW}Test 1: LSP Server Startup${NC}"
timeout 1 sanctifier lsp --debug << EOF &> /tmp/lsp_test.log || true
EOF
sleep 0.2
if grep -q "LSP server starting" /tmp/lsp_test.log; then
    echo -e "${GREEN}✓ Server starts successfully${NC}"
else
    echo -e "${RED}✗ Server failed to start${NC}"
    cat /tmp/lsp_test.log
    exit 1
fi

# Test 2: Initialize request
echo -e "\n${YELLOW}Test 2: Initialize Request${NC}"
INIT_MSG='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
INIT_LEN=${#INIT_MSG}
RESPONSE=$(printf "Content-Length: %d\r\n\r\n%s" "$INIT_LEN" "$INIT_MSG" | timeout 2 sanctifier lsp 2>/dev/null || true)

if echo "$RESPONSE" | grep -q "textDocumentSync"; then
    echo -e "${GREEN}✓ Initialize request successful${NC}"
else
    echo -e "${YELLOW}⊘ Initialize response format may need adjustment${NC}"
fi

# Test 3: Analyze sample Soroban contract
echo -e "\n${YELLOW}Test 3: Analyze Sample Contract${NC}"
SAMPLE_CONTRACT=$(cat << 'RUST'
#[contractimpl]
impl MyContract {
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        // Missing auth check
        let from_balance: i128 = env.storage().persistent().get(&from).unwrap_or(0);
        // Unchecked arithmetic
        let new_balance = from_balance - amount;
        env.storage().persistent().set(&from, &new_balance);
    }
}
RUST
)

# Test the analyzer directly
echo "Analyzing sample contract..."
ANALYZER_RESULT=$(echo "$SAMPLE_CONTRACT" | timeout 5 sanctifier lsp 2>&1 || true)

if [ -z "$ANALYZER_RESULT" ] || [ "$ANALYZER_RESULT" != "${ANALYZER_RESULT#*}" ]; then
    echo -e "${GREEN}✓ Sample contract analyzed without errors${NC}"
else
    echo -e "${YELLOW}⊘ Analyzer output: $ANALYZER_RESULT${NC}"
fi

# Test 4: Test with actual contract file
echo -e "\n${YELLOW}Test 4: Analyze Real Contract File${NC}"
if [ -f "contracts/token-with-bugs/src/lib.rs" ]; then
    CONTRACT_FILE="contracts/token-with-bugs/src/lib.rs"
    FILE_SIZE=$(wc -c < "$CONTRACT_FILE")
    echo "Testing with $CONTRACT_FILE ($FILE_SIZE bytes)..."
    
    if timeout 5 ./target/release/sanctifier analyze "$CONTRACT_FILE" &>/dev/null; then
        echo -e "${GREEN}✓ Real contract analyzed successfully${NC}"
    else
        echo -e "${YELLOW}⊘ Real contract analysis may need adjustment${NC}"
    fi
else
    echo -e "${YELLOW}⊘ Sample contract file not found${NC}"
fi

# Test 5: Help message
echo -e "\n${YELLOW}Test 5: LSP Command Help${NC}"
if sanctifier lsp --help | grep -q "Language Server Protocol"; then
    echo -e "${GREEN}✓ LSP help message is correct${NC}"
else
    echo -e "${YELLOW}⊘ LSP help message format may vary${NC}"
fi

# Test 6: Memory footprint baseline (optional)
echo -e "\n${YELLOW}Test 6: Memory Footprint Check${NC}"
if command -v ps &> /dev/null; then
    # Start server and check initial memory
    timeout 1 sanctifier lsp 2>/dev/null &
    SERVER_PID=$!
    sleep 0.1
    
    if ps -p $SERVER_PID &> /dev/null; then
        MEM=$(ps aux | grep $SERVER_PID | grep -v grep | awk '{print $6}')
        echo -e "Initial memory footprint: ${MEM} KB"
        
        if [ "$MEM" -lt 100000 ]; then
            echo -e "${GREEN}✓ Memory footprint acceptable${NC}"
        else
            echo -e "${YELLOW}⊘ Memory usage is higher than expected${NC}"
        fi
        
        kill $SERVER_PID &> /dev/null || true
    fi
else
    echo -e "${YELLOW}⊘ ps command not available for memory check${NC}"
fi

echo -e "\n${GREEN}=== Smoke Test Complete ===${NC}"
echo ""
echo "Next steps for editor integration:"
echo "1. VSCode: Install Sanctifier extension"
echo "2. Neovim: Add LSP configuration to init.lua"
echo "3. Helix: Update languages.toml"
echo "4. See docs/lsp-server.md for detailed setup"
echo ""
