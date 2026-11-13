#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
FRONTEND_DIR="$ROOT_DIR/frontend"
BACKEND_DIR="$ROOT_DIR/backend"

echo "=== Tyrus Playground Dev ==="
echo ""

# Check if MongoDB is running
if ! mongosh --eval "db.runCommand({ ping: 1 })" --quiet > /dev/null 2>&1; then
    echo "Warning: MongoDB does not appear to be running."
    echo "Start it with: mongod or docker run -d -p 27017:27017 mongo"
    echo ""
fi

echo "Starting servers..."
echo ""

# Start backend in background
echo "Starting backend on http://localhost:3001 ..."
cd "$BACKEND_DIR"
npm run dev &
BACKEND_PID=$!

# Start frontend, Vite's dev server (vite.config.ts proxies /api to the
# backend above, the same way nginx does in production), not a plain static
# file server: the frontend is real ES modules now, not vendored globals.
echo "Starting frontend on http://localhost:3000 ..."
cd "$FRONTEND_DIR"
npm run dev &
FRONTEND_PID=$!

echo ""
echo "=== Dev servers started ==="
echo "Frontend: http://localhost:3000"
echo "Backend:  http://localhost:3001"
echo ""
echo "Press Ctrl+C to stop both servers."

trap "kill $BACKEND_PID $FRONTEND_PID 2>/dev/null; exit" INT TERM
wait
