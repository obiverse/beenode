#!/bin/bash
# Beenode Dev Mode
#
# Runs backend in Docker, web locally with hot reload
#
# Usage: ./scripts/dev.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "Starting beenode backend in Docker..."
docker compose -f docker-compose.dev.yml up -d

echo ""
echo "Waiting for backend health check..."
until curl -sf http://localhost:8080/health > /dev/null 2>&1; do
    sleep 1
    echo -n "."
done
echo " Ready!"

echo ""
echo "Starting web dev server with hot reload..."
echo "  Backend: http://localhost:8080"
echo "  Frontend: http://localhost:5173"
echo ""

cd web
npm run dev
