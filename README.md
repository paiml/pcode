# pcode

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![Coverage](https://img.shields.io/badge/coverage-75%25-yellow.svg)](QUALITY.md)

Production-grade AI code agent with extreme performance and security requirements.

## 🚀 Features

### Performance & Efficiency
- **Extreme Performance**: <200ms first-token latency target
- **Minimal Footprint**: <12MB binary size with musl + UPX compression
- **Efficient Token Estimation**: Self-contained using 256KB perfect hash table
- **Optimized Runtime**: Custom-tuned Tokio async runtime with 2 worker threads

### Security & Sandboxing
- **Platform-Specific Sandboxing**:
  - 🐧 Linux: Landlock LSM (kernel 5.13+)
  - 🍎 macOS: Sandbox profiles
  - 🪟 Windows: AppContainer
- **Capability-Based Security**: Granular control over file, network, and process access
- **Zero Network Access**: Except through MCP tools

### Tools & Capabilities
- **File Operations**: Read/write with path restrictions
- **Process Execution**: Sandboxed command execution with timeout
- **LLM Integration**: Google AI Studio support (API key required)
- **Token Estimation**: Fast and accurate token counting
- **MCP Protocol**: Extensible tool system via Cap'n Proto

## 📦 Installation

### Prerequisites
- Rust 1.70+ (2021 edition)
- For optimal binary size: `rustup target add x86_64-unknown-linux-musl`
- Optional: UPX for binary compression

### Building from Source

```bash
# Clone the repository
git clone https://github.com/paiml/pcode.git
cd pcode

# Install build dependencies
make install-deps

# Build debug version (fast compilation)
make build

# Build optimized release (with compression)
make release
```

## 🔧 Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `AI_STUDIO_API_KEY` | Google AI Studio API key for LLM features | None |
| `RUST_LOG` | Logging level (`debug`, `info`, `warn`, `error`) | `info` |

### AI Studio Setup

1. Get your API key from [Google AI Studio](https://aistudio.google.com/app/apikey)
2. Configure the key:
   ```bash
   # Option 1: Export in shell
   export AI_STUDIO_API_KEY="your_api_key_here"
   
   # Option 2: Add to ~/.zshrc or ~/.bashrc
   echo 'export AI_STUDIO_API_KEY="your_api_key_here"' >> ~/.zshrc
   
   # Option 3: Create .env file
   cp .env.example .env
   # Edit .env with your key
   ```

## 📖 Usage

### Basic Commands

```bash
# Show help
pcode --help

# Run with default settings (current directory)
pcode

# Run with debug logging
pcode --debug

# Specify working directory
pcode --workdir /path/to/project

# Set memory limit
pcode --max-memory 1024

# Disable sandbox (not recommended)
pcode --no-sandbox
```

### Available Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `file_read` | Read file contents | `path`, `offset?`, `limit?` |
| `file_write` | Write content to file | `path`, `content`, `append?` |
| `process` | Execute system command | `command`, `args?`, `cwd?`, `timeout_ms?` |
| `llm` | Interact with language model | `prompt`, `max_tokens?`, `temperature?` |
| `token_estimate` | Estimate token count | `text`, `fast?` |

### Example: Dogfooding

pcode can analyze and improve itself! See our dogfooding examples:

```bash
# Analyze pcode's own code coverage
cargo run --example dogfood

# Generate tests for uncovered code
cargo run --example generate_tests

# Check API key configuration
cargo run --example test_api_key
```

## 🧪 Development

### Code Quality Standards

This project follows strict PMAT (Pragmatic Metrics for Agile Teams) standards:

| Metric | Target | Current Status |
|--------|--------|----------------|
| Cyclomatic Complexity | ≤ 20 per function | ✅ All pass |
| Test Coverage | ≥ 80% | 📊 75% |
| Technical Debt (SATD) | 0 | ✅ Zero |
| Test Dependency Graph | < 1.0 | ✅ All independent |
| Big O Complexity | ≤ O(n) | ✅ All linear or better |

### Development Commands

```bash
# Run full test suite
make test

# Generate coverage report (HTML)
make coverage

# Check code quality
make quality

# Run linters (fmt + clippy)
make lint

# Security audit
make audit

# View code metrics
make metrics

# Quick dev cycle (format, test, lint)
make dev

# Full CI pipeline
make ci
```

### Project Structure

```
pcode/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library root
│   ├── config.rs        # Configuration management
│   ├── runtime/         # Async runtime (Tokio)
│   ├── security/        # Platform sandboxing
│   ├── mcp/             # MCP protocol implementation
│   ├── tools/           # Tool implementations
│   └── token_estimation/ # Token counting
├── benches/             # Performance benchmarks
├── tests/               # Integration tests
├── examples/            # Usage examples
└── docs/                # Documentation
```

## 📊 Performance

### Benchmarks

Run benchmarks with:
```bash
make bench
```

| Benchmark | Performance |
|-----------|-------------|
| Token estimation (short) | ~500 ns/op |
| Token estimation (long) | ~50 μs/op |
| Runtime creation | ~100 μs |
| Task spawning | ~1 μs |

### Binary Size

With musl target and UPX compression:
- Debug build: ~50MB
- Release build: ~8-10MB (approaching <12MB target)

## 🔒 Security

### Sandboxing Details

pcode implements defense-in-depth with platform-specific sandboxing:

1. **File System**: Only allowed paths are accessible (default: working directory)
2. **Network**: Disabled by default, no direct network access
3. **Process**: Controlled process spawning with resource limits
4. **Memory**: Configurable memory limits (default: 512MB)

### Security Policy

See [SECURITY.md](SECURITY.md) for vulnerability reporting.

## 🤝 Contributing

Contributions are welcome! Please ensure:

1. All tests pass: `make test`
2. Code is formatted: `make fmt-fix`
3. No clippy warnings: `make clippy`
4. Coverage doesn't decrease: `make coverage`
5. Complexity limits are maintained: `make quality`

## 📚 Documentation

- [Technical Specification](docs/v1-spec.md) - Detailed architecture and design
- [Code Quality Report](QUALITY.md) - Coverage and complexity metrics
- [CLAUDE.md](CLAUDE.md) - AI assistant integration guide

## 🏗️ Roadmap

- [ ] Reach 80% test coverage
- [ ] Implement full AI Studio API integration
- [ ] Add more MCP tools (git, search, etc.)
- [ ] Create VSCode/Neovim extensions
- [ ] Build GitHub Actions integration
- [ ] Optimize binary size to <10MB

## 📝 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) 🦀
- Uses [Tokio](https://tokio.rs/) for async runtime
- Implements [MCP](https://github.com/modelcontextprotocol) for tool communication
- Designed for integration with [Claude](https://claude.ai/code)

---

**pcode** - Pragmatic AI Labs | [GitHub](https://github.com/paiml/pcode) | [Issues](https://github.com/paiml/pcode/issues)