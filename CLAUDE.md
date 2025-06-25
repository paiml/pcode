# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

pcode is a Rust-based AI code agent designed for production use with extreme performance and security requirements. The project is currently in the specification phase with no implementation yet.

## Key Architecture Decisions

### Performance Targets
- Binary size: <12MB (stripped, UPX compressed)
- Memory: 20MB RSS baseline
- Latency: <200ms first-token, 150ms p99
- Token estimation: Self-contained using 256KB perfect hash table

### Security Architecture
- Platform-specific sandboxing:
  - Linux: Landlock LSM
  - macOS: Sandbox profiles
  - Windows: AppContainer
- Minimal dependencies to reduce attack surface
- No network access except through MCP tools

### Technical Stack
- Language: Rust
- Async runtime: Tokio with custom schedulers
- IPC: MCP protocol over stdio using Cap'n Proto
- Build target: musl for static linking

## Development Commands

The project uses a Makefile for all build and development tasks:

```bash
# Build debug version
make build

# Build optimized release (requires musl target)
make release

# Run tests
make test

# Run linters (fmt + clippy)
make lint

# Run benchmarks
make bench

# Full CI pipeline
make ci

# Development cycle (format, test, lint)
make dev

# Clean build artifacts
make clean
```

To run the application:
```bash
./target/debug/pcode --help
./target/debug/pcode --debug
```

## Important Specifications

The complete technical specification is in `docs/v1-spec.md`. Key points:

1. **Unified Runtime**: All async operations through a single Tokio runtime with platform-specific tuning
2. **Tool System**: MCP-based tools for file operations, process execution, and LLM communication
3. **Memory Management**: Aggressive optimization with custom allocators and memory mapping
4. **Cross-Platform**: Must support Linux, macOS, and Windows with platform-specific optimizations

## Code Quality Standards

This project practices extreme quality using PMAT metrics:
- **Cyclomatic Complexity**: 10-20 maximum for any function
- **Test Dependency Graph (TDG)**: Under 1.0 for any function
- **Technical Debt**: Zero tolerance for Self-Admitted Technical Debt (SATD)
- **Algorithm Complexity**: Low Big O complexity required
- **Code Verifiability**: Highly provable code with formal reasoning
- **Test Coverage**: Maintain 80% test coverage at all times
- **Mock Code**: ZERO TOLERANCE for mocking - all code must be production-ready
- **Comments**: ZERO TOLERANCE for TODO, FIXME, HACK, or any SATD comments
- **Scripts**: NO BASH - Only Deno TypeScript scripts in scripts/*.ts
- **Script Quality**: All scripts must be tested, linted, type-checked, and formatted

## Environment Variables

- `AI_STUDIO_API_KEY`: Google AI Studio API key for LLM tool functionality
- `RUST_LOG`: Logging level (e.g., `debug`, `info`, `warn`)

## Project Status

The project has been implemented with all core modules:
- Runtime with Tokio async
- Security sandboxing (platform stubs)
- MCP protocol framework
- Tool system (file, process, llm)
- Token estimation
- CLI interface

The LLM tool will use the AI Studio API if `AI_STUDIO_API_KEY` is set, otherwise falls back to mock responses.