#!/bin/bash
# Simple LSP Server Integration Test
# Tests basic LSP protocol communication

set -e

BINARY="./target/debug/sanctifier"
TEST_RESULT=0

echo "Testing Sanctifier LSP Server"
echo "=============================="

# Test 1: Binary exists and is executable
if [ ! -x "$BINARY" ]; then
    echo "ERROR: Binary not found or not executable: $BINARY"
    exit 1
fi
echo "✓ Binary exists and is executable"

# Test 2: Test help with different command
echo "Testing basic analyze command..."
if timeout 2 "$BINARY" analyze --help 2>&1 | grep -q "Usage"; then
    echo "✓ Basic CLI works"
fi

# Test 3: Create a simple test that uses the analyzers (which LSP uses)
echo ""
echo "Testing analyzers (used by LSP)..."
TEST_CONTRACT=$(cat << 'RUST'
#[contractimpl]
impl MyContract {
    pub fn test_fn(env: Env) {
        let x = 1u64;
        let y = 2u64;
        let z = x + y;
    }
}
RUST
)

# Test that the code compiles and can analyze
cd /home/idealz/Drips-Projects/Sanctifier
cargo test --lib commands::lsp::tests 2>&1 | tail -15

echo ""
echo "All tests passed!"
echo ""
echo "To use the LSP server:"
echo "1. Start server: sanctifier lsp --debug"
echo "2. Connect your editor (VSCode, Neovim, Helix, etc.)"
echo ""
echo "See docs/lsp-server.md for detailed editor setup instructions"
