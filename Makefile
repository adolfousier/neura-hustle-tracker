.PHONY: build run setup install-extension clean

install-extension:
	./install-extension.sh

setup: install-extension
	docker-compose up -d postgres
	@echo "Waiting for database to be ready..."
	@sleep 10

build: setup
	cargo build --release

run: build
	./target/release/time_tracker

clean:
	docker-compose down -v
	cargo clean