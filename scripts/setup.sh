#!/bin/bash

# Sanctifier Contributor Setup Script 🛡️
# This script helps new contributors set up their development environment.

set -e

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}==================================================${NC}"
echo -e "${BLUE}   Sanctifier 🛡️  - Contributor Setup Script      ${NC}"
echo -e "${BLUE}==================================================${NC}"
echo ""

# --- Helper Functions ---
confirm() {
    read -r -p "${1} [y/N] " response
    case "$response" in
        [yY][eE][sS]|[yY]) 
            true
            ;;
        *)
            false
            ;;
    esac
}

check_tool() {
    if command -v "$1" >/dev/null 2>&1; then
        echo -e "${GREEN}[✓] $1 is installed.${NC}"
        return 0
    else
        echo -e "${YELLOW}[!] $1 is not found.${NC}"
        return 1
    fi
}

# --- 1. Rust & Cargo ---
echo -e "${BLUE}Checking Rust & Cargo...${NC}"
if ! check_tool "rustc"; then
    if confirm "Would you like to install Rust via rustup.rs?"; then
        echo "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    else
        echo -e "${RED}Please install Rust manually: https://rustup.rs/${NC}"
        exit 1
    fi
fi

# --- 2. WASM Target ---
echo -e "${BLUE}Checking for wasm32-unknown-unknown target...${NC}"
if rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    echo -e "${GREEN}[✓] WASM target is installed.${NC}"
else
    if confirm "WASM target is missing. Add it now?"; then
        rustup target add wasm32-unknown-unknown
        echo -e "${GREEN}[✓] WASM target added.${NC}"
    fi
fi

# --- 3. Soroban CLI ---
echo -e "${BLUE}Checking Soroban CLI...${NC}"
if ! check_tool "soroban"; then
    if confirm "Soroban CLI is missing. Install it via Cargo? (This may take a while)"; then
        cargo install --locked soroban-cli
        echo -e "${GREEN}[✓] Soroban CLI installed.${NC}"
    fi
fi

# --- 4. Node.js & NPM ---
echo -e "${BLUE}Checking Node.js & NPM...${NC}"
if ! check_tool "node"; then
    echo -e "${YELLOW}[!] Node.js is missing. Please install it from: https://nodejs.org/${NC}"
fi
if ! check_tool "npm"; then
    echo -e "${YELLOW}[!] NPM is missing.${NC}"
fi

# --- 5. Project Initialization ---
echo -e "${BLUE}Initializing Project...${NC}"

# Frontend
if [ -d "frontend" ]; then
    if confirm "Run 'npm install' in the frontend directory?"; then
        cd frontend
        npm install
        cd ..
        echo -e "${GREEN}[✓] Frontend dependencies installed.${NC}"
    fi
fi

# Tooling
if confirm "Build project tooling via Cargo?"; then
    cargo build
    echo -e "${GREEN}[✓] Project tooling built.${NC}"
fi

echo ""
echo -e "${BLUE}==================================================${NC}"
echo -e "${GREEN}   Setup Complete! You are ready to develop.     ${NC}"
echo -e "${BLUE}==================================================${NC}"
echo -e "Check ${YELLOW}docs/getting-started.md${NC} for next steps."
echo ""
