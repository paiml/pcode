# pcode Makefile
# Production-grade AI code agent

# Variables
CARGO = cargo
BINARY_NAME = pcode
# Check if musl target is available, otherwise use default
MUSL_AVAILABLE := $(shell rustup target list --installed | grep -q x86_64-unknown-linux-musl && echo "yes" || echo "no")
ifeq ($(MUSL_AVAILABLE),yes)
    TARGET = x86_64-unknown-linux-musl
else
    TARGET = 
endif

# Set directories based on target
ifeq ($(TARGET),)
    RELEASE_DIR = target/release
    DEBUG_DIR = target/debug
else
    RELEASE_DIR = target/$(TARGET)/release
    DEBUG_DIR = target/$(TARGET)/debug
endif

# Rust flags for optimization (only for release builds)
# Note: Using cargo profile settings instead of RUSTFLAGS to avoid conflicts
RELEASE_RUSTFLAGS =

# Default target
.PHONY: all
all: build

# Build targets
.PHONY: build
build:
ifeq ($(TARGET),)
	$(CARGO) build
else
	$(CARGO) build --target $(TARGET)
endif

.PHONY: release
release:
ifeq ($(TARGET),)
	$(CARGO) build --release
else
	$(CARGO) build --release --target $(TARGET)
endif
	@echo "Stripping binary..."
	@cp $(RELEASE_DIR)/$(BINARY_NAME) $(RELEASE_DIR)/$(BINARY_NAME).tmp
	@strip $(RELEASE_DIR)/$(BINARY_NAME).tmp && mv $(RELEASE_DIR)/$(BINARY_NAME).tmp $(RELEASE_DIR)/$(BINARY_NAME) || echo "Strip failed, skipping"
	@echo "Checking for UPX..."
	@command -v upx >/dev/null 2>&1 && upx --best --lzma $(RELEASE_DIR)/$(BINARY_NAME) || echo "UPX not found, skipping compression"
	@echo "Release build complete: $(RELEASE_DIR)/$(BINARY_NAME)"
	@ls -lh $(RELEASE_DIR)/$(BINARY_NAME)

# Development commands
.PHONY: run
run:
	$(CARGO) run

.PHONY: check
check:
	$(CARGO) check --all-targets

# Testing targets
.PHONY: test
test:
	$(CARGO) test --all-features

.PHONY: test-coverage
test-coverage:
	$(CARGO) tarpaulin --out Xml --all-features --workspace --timeout 120 --exclude-files target/*

.PHONY: coverage
coverage:
	$(CARGO) tarpaulin --out Html --all-features --workspace --timeout 120 --exclude-files target/* --output-dir coverage
	@echo "Coverage report generated in coverage/tarpaulin-report.html"
	@$(CARGO) tarpaulin --print-summary --all-features --workspace --timeout 120 --exclude-files target/* 2>/dev/null | grep "Coverage" || true

# Code quality targets
.PHONY: lint
lint: fmt clippy deno-check

.PHONY: fmt
fmt:
	$(CARGO) fmt -- --check
	deno fmt --check scripts/

.PHONY: fmt-fix
fmt-fix:
	$(CARGO) fmt
	deno fmt scripts/

.PHONY: clippy
clippy:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: deno-check
deno-check:
	deno check scripts/*.ts
	deno lint scripts/
	deno test --allow-read --allow-env --allow-run scripts/

# Benchmarking
.PHONY: bench
bench:
	$(CARGO) bench --target $(TARGET)

# Documentation
.PHONY: doc
doc:
	$(CARGO) doc --no-deps --open

# Clean
.PHONY: clean
clean:
	$(CARGO) clean

# Security audit
.PHONY: audit
audit:
	$(CARGO) audit

# Quality and complexity analysis
.PHONY: quality
quality: complexity metrics

.PHONY: complexity
complexity:
	@echo "=== Cyclomatic Complexity Analysis ==="
	$(CARGO) install cargo-complexity || true
	find src -name "*.rs" -exec echo "File: {}" \; -exec grep -E "^(pub |)fn " {} \; | head -20
	@echo "Note: All functions should have complexity <= 20"

.PHONY: metrics
metrics:
	@echo "=== Code Metrics ==="
	@echo -n "Total lines of code: "
	@find src -name "*.rs" -exec cat {} \; | wc -l
	@echo -n "Number of functions: "
	@find src -name "*.rs" -exec grep -E "^[[:space:]]*(pub |)fn " {} \; | wc -l
	@echo -n "Number of tests: "
	@find . -name "*.rs" -exec grep -E "#\[test\]|#\[tokio::test\]" {} \; | wc -l
	@echo ""

# Install dependencies
.PHONY: install-deps
install-deps:
	rustup target add $(TARGET)
	$(CARGO) install cargo-tarpaulin
	$(CARGO) install cargo-audit
	$(CARGO) install cargo-geiger
	command -v upx >/dev/null 2>&1 || echo "Please install UPX for binary compression"

# Quick development cycle
.PHONY: dev
dev: fmt-fix test clippy

# CI/CD pipeline simulation
.PHONY: ci
ci: check fmt clippy test audit

# Binary size analysis
.PHONY: size
size: release
	@echo "Binary size analysis:"
	@ls -lh $(RELEASE_DIR)/$(BINARY_NAME)
	@size $(RELEASE_DIR)/$(BINARY_NAME) || true

# Chat testing targets
.PHONY: test-chat
test-chat:
	@echo "=== Testing pcode chat functionality ==="
	@test -f target/release/pcode || (echo "Error: target/release/pcode not found. Run 'make release' first." && exit 1)
	deno run --allow-run --allow-env scripts/test_interactive.ts
	deno run --allow-run --allow-env scripts/test_chat_responses.ts

.PHONY: demo-chat
demo-chat:
	@echo "=== pcode Interactive Mode Demo ==="
	deno run scripts/demo_interactive.ts

.PHONY: verify-chat
verify-chat: test-chat demo-chat
	@echo "✅ Chat verification complete"

.PHONY: help
help:
	@echo "pcode Makefile targets:"
	@echo "  make build         - Build debug binary"
	@echo "  make release       - Build optimized release binary with compression"
	@echo "  make run           - Run the application"
	@echo "  make test          - Run all tests"
	@echo "  make test-coverage - Run tests with coverage report (XML)"
	@echo "  make coverage      - Run tests with HTML coverage report"
	@echo "  make lint          - Run all linters (fmt + clippy + deno)"
	@echo "  make quality       - Run code quality analysis"
	@echo "  make bench         - Run benchmarks"
	@echo "  make doc           - Generate and open documentation"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make audit         - Run security audit"
	@echo "  make complexity    - Analyze code complexity"
	@echo "  make install-deps  - Install required dependencies"
	@echo "  make dev           - Quick development cycle (format, test, lint)"
	@echo "  make ci            - Run full CI pipeline"
	@echo "  make size          - Analyze binary size"
	@echo "  make test-chat     - Test interactive chat functionality"
	@echo "  make demo-chat     - Run interactive mode demo"
	@echo "  make verify-chat   - Full chat verification (build + test + demo)"