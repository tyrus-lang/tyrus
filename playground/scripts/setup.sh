#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
FRONTEND_DIR="$ROOT_DIR/frontend"
BACKEND_DIR="$ROOT_DIR/backend"

echo "=== Tyrus Playground Setup ==="
echo ""

echo "[1/3] Installing backend dependencies..."
cd "$BACKEND_DIR"
npm ci
echo ""

echo "[2/3] Installing frontend dependencies..."
cd "$FRONTEND_DIR"
npm ci
echo ""

if [ ! -f "$BACKEND_DIR/.env" ]; then
    echo "[3/3] Creating backend .env from template..."
    cp "$BACKEND_DIR/.env.example" "$BACKEND_DIR/.env"
    echo "  Edit $BACKEND_DIR/.env to configure the local backend."
else
    echo "[3/3] Backend .env already exists."
fi
