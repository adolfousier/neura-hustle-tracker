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
run: check-wayland db-up build
	@echo "Starting Neura Hustle Tracker..."
	./$(BINARY)

# Quick dev mode: Start DB + Run in debug mode (faster)
dev: check-wayland db-up
	@echo "Starting in development mode..."
	cargo run

# Just build release binary
build:
	@echo "Building release binary..."
	cargo build --release

# Start PostgreSQL
db-up:
	@echo "Starting PostgreSQL..."
	docker compose up -d
	@echo "Waiting for database to be ready..."
	@sleep 5

# Check for Wayland and install extension if needed (Linux only)
check-wayland:
ifndef OS
	@if [ "$$XDG_SESSION_TYPE" = "wayland" ] || [ -n "$$WAYLAND_DISPLAY" ]; then \
		echo "Wayland session detected!"; \
		echo "Checking for Window Calls GNOME extension..."; \
		if ! gnome-extensions list 2>/dev/null | grep -q "window-calls"; then \
			echo ""; \
			echo "⚠️  WAYLAND SETUP REQUIRED ⚠️"; \
			echo ""; \
			echo "The 'Window Calls' GNOME extension is required for Wayland support."; \
			echo ""; \
			echo "Install it by visiting:"; \
			echo "  https://extensions.gnome.org/extension/4724/window-calls/"; \
			echo ""; \
			echo "Or install Extension Manager:"; \
			echo "  sudo apt install gnome-shell-extension-manager"; \
			echo ""; \
			echo "After installing, re-run 'make run'"; \
			echo ""; \
			exit 1; \
		else \
			echo "✓ Window Calls extension found!"; \
		fi; \
	else \
		echo "X11 session detected - no additional setup needed."; \
	fi
endif

# Stop PostgreSQL
db-down:
	@echo "Stopping PostgreSQL..."
	docker compose down

# Clean everything
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "Stopping and removing database..."
	docker compose down -v
	@echo "Clean complete!"

# Setup (alias for db-up for backward compatibility)
setup: db-up
