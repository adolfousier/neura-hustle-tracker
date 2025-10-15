.PHONY: run dev build setup db-up db-down clean help

# Detect OS for cross-platform support
ifeq ($(OS),Windows_NT)
    BINARY := target/release/neura_hustle_tracker.exe
    RM := del /Q
    RMDIR := rmdir /S /Q
else
    UNAME_S := $(shell uname -s)
    BINARY := target/release/neura_hustle_tracker
    RM := rm -f
    RMDIR := rm -rf
endif

# Default target
help:
	@echo "Neura Hustle Tracker - Make Commands"
	@echo "====================================="
	@echo ""
	@echo "Quick Start:"
	@echo "  make run        - Start DB + build (release) + run app (ONE COMMAND!)"
	@echo "  make dev        - Start DB + run app in dev mode (faster builds)"
	@echo ""
	@echo "Individual Steps:"
	@echo "  make db-up      - Start PostgreSQL in Docker"
	@echo "  make build      - Build release binary only"
	@echo "  make db-down    - Stop PostgreSQL"
	@echo "  make clean      - Clean all build artifacts and stop DB"
	@echo ""
	@echo "Note: Credentials are auto-generated on first run!"

# ONE COMMAND: Start DB + Build Release + Run
run: db-up build
	@echo "Starting Neura Hustle Tracker..."
	./$(BINARY)

# Quick dev mode: Start DB + Run in debug mode (faster)
dev: db-up
	@echo "Starting in development mode..."
	cargo run

# Just build release binary
build:
	@echo "Building release binary..."
	cargo build --release

# Start PostgreSQL
db-up:
	@echo "Starting PostgreSQL..."
	docker-compose up -d
	@echo "Waiting for database to be ready..."
	@sleep 5

# Stop PostgreSQL
db-down:
	@echo "Stopping PostgreSQL..."
	docker-compose down

# Clean everything
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "Stopping and removing database..."
	docker-compose down -v
	@echo "Clean complete!"

# Setup (alias for db-up for backward compatibility)
setup: db-up
