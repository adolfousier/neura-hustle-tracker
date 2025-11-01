.PHONY: run dev build setup db-up db-down clean help daemon-start daemon-stop daemon-status view build-daemon uninstall

# Detect OS for cross-platform support
ifeq ($(OS),Windows_NT)
    BINARY := target/release/neura_hustle_tracker.exe
    DAEMON_BINARY := target/release/neura_hustle_daemon.exe
    RM := del /Q
    RMDIR := rmdir /S /Q
    PID_FILE := daemon.pid
else
    UNAME_S := $(shell uname -s)
    BINARY := target/release/neura_hustle_tracker
    DAEMON_BINARY := target/release/neura_hustle_daemon
    RM := rm -f
    RMDIR := rm -rf
    PID_FILE := daemon.pid
endif

# Default target
help:
	@echo "Neura Hustle Tracker - Make Commands"
	@echo "====================================="
	@echo ""
	@echo "Quick Start (Linux - Unified Mode):"
	@echo "  make run           - Start DB + build + run app (ONE COMMAND!)"
	@echo "  make dev           - Start DB + run app in dev mode (faster builds)"
	@echo ""
	@echo "macOS/Windows (Daemon Mode - Recommended):"
	@echo "  make daemon-start  - Start background tracking daemon"
	@echo "  make daemon-stop   - Stop background tracking daemon"
	@echo "  make daemon-status - Check if daemon is running"
	@echo "  make view          - Open TUI to view stats (daemon must be running)"
	@echo ""
	@echo "Individual Steps:"
	@echo "  make db-up         - Start PostgreSQL in Docker"
	@echo "  make build         - Build TUI binary only"
	@echo "  make build-daemon  - Build daemon binary only"
	@echo "  make db-down       - Stop PostgreSQL"
	@echo "  make clean         - Clean all build artifacts and stop DB"
	@echo ""
	@echo "Cleanup & Removal:"
	@echo "  make uninstall     - Remove app, database volume, and local directory"
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

# Just build TUI release binary
build:
	@echo "Building TUI release binary..."
	cargo build --release --bin neura_hustle_tracker

# Just build daemon release binary
build-daemon:
	@echo "Building daemon release binary..."
	cargo build --release --bin neura_hustle_daemon

# Start PostgreSQL
db-up:
	@echo "Starting PostgreSQL..."
	docker compose up -d
	@echo "Waiting for database to be ready..."
	@$(SLEEP)

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

# Daemon control commands
daemon-start: db-up build-daemon
	@echo "Starting background daemon..."
	@if [ -f $(PID_FILE) ]; then \
		echo "Daemon already running (PID: $$(cat $(PID_FILE)))"; \
		echo "Run 'make daemon-stop' first to restart"; \
		exit 1; \
	fi
	@nohup $(DAEMON_BINARY) > daemon.log 2>&1 & echo $$! > $(PID_FILE)
	@echo "Daemon started (PID: $$(cat $(PID_FILE)))"
	@echo "Logs: daemon.log"
	@echo "To view stats: make view"

daemon-stop:
	@if [ ! -f $(PID_FILE) ]; then \
		echo "Daemon not running (no PID file found)"; \
		exit 1; \
	fi
	@echo "Stopping daemon (PID: $$(cat $(PID_FILE)))..."
	@kill $$(cat $(PID_FILE)) 2>/dev/null || echo "Process already stopped"
	@rm -f $(PID_FILE)
	@echo "Daemon stopped"

daemon-status:
	@if [ ! -f $(PID_FILE) ]; then \
		echo "Daemon is NOT running"; \
		exit 1; \
	fi
	@if ps -p $$(cat $(PID_FILE)) > /dev/null 2>&1; then \
		echo "Daemon is RUNNING (PID: $$(cat $(PID_FILE)))"; \
		echo "Logs: daemon.log"; \
	else \
		echo "Daemon is NOT running (stale PID file)"; \
		rm -f $(PID_FILE); \
		exit 1; \
	fi

view: build
	@if [ ! -f $(PID_FILE) ]; then \
		echo "⚠️  Warning: Daemon not running"; \
		echo "Start daemon first: make daemon-start"; \
		echo ""; \
		echo "Opening TUI in viewer mode anyway..."; \
	fi
	@echo "Opening TUI..."
	./$(BINARY)

# Uninstall everything: stop daemon, remove DB volume, and delete local directory
uninstall:
ifeq ($(OS),Windows_NT)
	@echo "Uninstalling Neura Hustle Tracker..."
	@if exist $(PID_FILE) (del /Q $(PID_FILE))
	@echo.
	@echo "This will:"
	@echo "  1. Stop the PostgreSQL database"
	@echo "  2. Remove the database volume (all tracked data)"
	@echo "  3. Delete the local installation directory"
	@echo.
	@set /p response="Do you want to proceed? (yes/no): "
	@if /I "!response!"=="yes" (
		@echo Stopping Docker Compose...
		@docker compose down -v
		@echo.
		@echo ⚠️  WARNING: This will delete the app directory from your computer!
		@set /p confirm="Type 'yes' to confirm deletion of all files: "
		@if /I "!confirm!"=="yes" (
			@echo Removing installation directory...
			@cd ..
			@rmdir /s /q "neura-hustle-tracker"
			@echo ✓ Uninstall complete!
		) else (
			@echo ✗ Cancelled. Directory kept.
		)
	) else (
		@echo ✗ Uninstall cancelled.
	)
else
	@echo "Uninstalling Neura Hustle Tracker..."
	@rm -f $(PID_FILE)
	@echo ""
	@echo "This will:"
	@echo "  1. Stop the PostgreSQL database"
	@echo "  2. Remove the database volume (all tracked data)"
	@echo "  3. Delete the local installation directory"
	@echo ""
	@read -p "Do you want to proceed? (yes/no): " response; \
	if [ "$$response" = "yes" ]; then \
		echo "Stopping Docker Compose..."; \
		docker compose down -v; \
		echo ""; \
		echo "⚠️  WARNING: This will delete the app directory from your computer!"; \
		read -p "Type 'yes' to confirm deletion of all files: " confirm; \
		if [ "$$confirm" = "yes" ]; then \
			echo "Removing installation directory..."; \
			cd ..; \
			rm -rf neura-hustle-tracker; \
			echo "✓ Uninstall complete!"; \
		else \
			echo "✗ Cancelled. Directory kept."; \
		fi; \
	else \
		echo "✗ Uninstall cancelled."; \
	fi
endif
