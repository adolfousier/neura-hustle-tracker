#!/bin/bash

# Neura Hustle Tracker - Uninstall Script
# This script removes the application, database volume, and local directory

set -e

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

echo "====================================="
echo "Neura Hustle Tracker - Uninstall"
echo "====================================="
echo ""
echo "This will:"
echo "  1. Stop the PostgreSQL database"
echo "  2. Remove the database volume (all tracked data)"
echo "  3. Delete the local installation directory"
echo ""
echo -e "${YELLOW}⚠️  WARNING: This cannot be undone!${NC}"
echo ""

# First confirmation
read -p "Do you want to proceed? (yes/no): " response
if [ "$response" != "yes" ]; then
    echo "✗ Uninstall cancelled."
    exit 0
fi

echo ""
echo "Stopping Docker Compose and removing database..."
cd "$PROJECT_ROOT"
docker compose down -v 2>/dev/null || echo "Docker Compose already stopped or not running"

echo ""
echo -e "${RED}✗ FINAL WARNING: This will delete ALL files in:${NC}"
echo "  $PROJECT_ROOT"
echo ""
read -p "Type 'yes' to confirm complete deletion: " confirm

if [ "$confirm" != "yes" ]; then
    echo "✗ Cancelled. Directory kept."
    exit 0
fi

echo ""
echo "Removing installation directory..."
cd ..
rm -rf "$(basename "$PROJECT_ROOT")"

echo ""
echo -e "${GREEN}✓ Uninstall complete!${NC}"
echo ""
echo "Neura Hustle Tracker has been completely removed."
echo "All tracked data has been deleted."
