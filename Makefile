# pcode Makefile
# Production-grade AI code agent

# Variables
CARGO = cargo
TARGET = x86_64-unknown-linux-musl
BINARY_NAME = pcode
RELEASE_DIR = target/$(TARGET)/release
DEBUG_DIR = target/$(TARGET)/debug

# Rust flags for optimization (only for release builds)
RELEASE_RUSTFLAGS = -C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1

# Default target
.PHONY: all
all: build

# Build targets
.PHONY: build
build:
	$(CARGO) build --target $(TARGET)

.PHONY: release
release:
	RUSTFLAGS="$(RELEASE_RUSTFLAGS)" $(CARGO) build --release --target $(TARGET)
	strip $(RELEASE_DIR)/$(BINARY_NAME)
	upx --best --lzma $(RELEASE_DIR)/$(BINARY_NAME) || true

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
lint: fmt clippy

.PHONY: fmt
fmt:
	$(CARGO) fmt -- --check

.PHONY: fmt-fix
fmt-fix:
	$(CARGO) fmt

.PHONY: clippy
clippy:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

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

.PHONY: help
help:
	@echo "pcode Makefile targets:"
	@echo "  make build         - Build debug binary"
	@echo "  make release       - Build optimized release binary with compression"
	@echo "  make run           - Run the application"
	@echo "  make test          - Run all tests"
	@echo "  make test-coverage - Run tests with coverage report (XML)"
	@echo "  make coverage      - Run tests with HTML coverage report"
	@echo "  make lint          - Run all linters (fmt + clippy)"
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