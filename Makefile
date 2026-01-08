.PHONY: help install test lint format build clean

help:
	@echo "Constellation Metagraph SDK"
	@echo ""
	@echo "Usage: make <target>"
	@echo ""
	@echo "Targets:"
	@echo "  install     Install dependencies for all packages"
	@echo "  test        Run tests for all packages"
	@echo "  lint        Run linters for all packages"
	@echo "  format      Format code in all packages"
	@echo "  build       Build all packages"
	@echo "  clean       Clean build artifacts"
	@echo ""
	@echo "Per-language targets:"
	@echo "  install-ts, test-ts, lint-ts, format-ts, build-ts"
	@echo "  install-py, test-py, lint-py, format-py"

# TypeScript targets
install-ts:
	cd packages/typescript && npm install

test-ts:
	cd packages/typescript && npm test

lint-ts:
	cd packages/typescript && npm run lint

format-ts:
	cd packages/typescript && npm run format

build-ts:
	cd packages/typescript && npm run build

clean-ts:
	rm -rf packages/typescript/dist packages/typescript/coverage packages/typescript/node_modules

# Python targets
install-py:
	cd packages/python && python3 -m pip install -e ".[dev]"

test-py:
	cd packages/python && python3 -m pytest

lint-py:
	cd packages/python && ruff check src tests && black --check src tests && isort --check-only src tests

format-py:
	cd packages/python && black src tests && isort src tests

clean-py:
	rm -rf packages/python/.pytest_cache packages/python/.mypy_cache packages/python/dist packages/python/*.egg-info packages/python/src/*.egg-info

# Combined targets
install: install-ts install-py

test: test-ts test-py

lint: lint-ts lint-py

format: format-ts format-py

build: build-ts

clean: clean-ts clean-py