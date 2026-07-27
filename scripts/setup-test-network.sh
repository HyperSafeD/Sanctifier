#!/bin/bash
set -euo pipefail

echo "🚀 Setting up Soroban test network for E2E tests..."

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
  echo "❌ Docker is not running. Please start Docker and try again."
  exit 1
fi

# Stop existing container if running
if docker ps -a --format '{{.Names}}' | grep -q '^soroban-standalone$'; then
  echo "Stopping existing soroban-standalone container..."
  docker stop soroban-standalone || true
  docker rm soroban-standalone || true
fi

# Start Soroban standalone network
echo "Starting Soroban standalone network..."
docker run -d \
  --name soroban-standalone \
  -p 8000:8000 \
  stellar/quickstart:soroban-dev@sha256:latest \
  --standalone \
  --enable-soroban-rpc

# Wait for network to be ready
echo "Waiting for Soroban RPC to be ready..."
MAX_ATTEMPTS=30
ATTEMPT=0

until curl -s http://localhost:8000/health | grep -q "ready"; do
  ATTEMPT=$((ATTEMPT + 1))
  if [ $ATTEMPT -ge $MAX_ATTEMPTS ]; then
    echo "❌ Timeout waiting for Soroban RPC"
    docker logs soroban-standalone
    exit 1
  fi
  echo "Waiting... (attempt $ATTEMPT/$MAX_ATTEMPTS)"
  sleep 2
done

echo "✅ Soroban test network ready at http://localhost:8000"
echo ""
echo "To stop the network:"
echo "  docker stop soroban-standalone && docker rm soroban-standalone"
