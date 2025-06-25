# pcode

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![Coverage](https://img.shields.io/badge/coverage-80.9%25-green.svg)](QUALITY.md)

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

### Tools & Capabilities (13 Built-in Tools)
- **File Operations**: Read/write with path restrictions
- **Process Execution**: Sandboxed command execution with timeout
- **Code Execution**: Sandboxed Python and JavaScript/TypeScript execution
- **LLM Integration**: Google AI Studio support with Gemini 2.0 Flash (API key required)
- **Token Estimation**: Fast and accurate token counting
- **Code Analysis**: PMAT integration for complexity, SATD, coverage, and TDG
- **Development Tools**: Bash, ripgrep, cargo, git integration
- **Code Quality**: Real coverage analysis with tarpaulin
- **AI Refactoring**: Intelligent code improvement suggestions
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

### Interactive Mode

pcode supports an interactive chat interface similar to Claude:

```bash
# Start interactive mode (default when no command given)
pcode

# Explicitly start interactive mode
pcode --interactive

# Interactive mode with custom working directory
pcode -i --workdir /path/to/project
```

### Basic Commands

```bash
# Show help
pcode --help

# Show version
pcode --version

# Run with debug logging
pcode --debug

# Execute a single command
pcode --command "/file_read src/main.rs"
pcode -c "/pmat complexity src/"

# Set memory limit
pcode --max-memory 1024

# Disable sandbox (not recommended)
pcode --no-sandbox
```

### Interactive Mode Commands

Once in interactive mode:

```
pcode> help                          # Show available commands
pcode> tools                         # List available tools
pcode> /file_read src/main.rs        # Read a file
pcode> /file_write test.txt Hello    # Write to a file
pcode> /process ls -la               # Execute a command
pcode> /llm Explain this code        # Query the LLM (requires API key)
pcode> /token_estimate text          # Estimate token count
pcode> /pmat complexity src/         # Analyze code complexity
pcode> /pmat satd .                  # Find technical debt
pcode> /bash find . -name "*.rs"     # Run bash commands
pcode> /dev_cli rg TODO              # Use ripgrep to find TODOs
pcode> /fix format src/main.rs       # Auto-format code
pcode> /coverage                     # Run code coverage analysis
pcode> /refactor src/complex.rs      # Get refactoring suggestions
pcode> /python print("Hello!")       # Run Python code
pcode> /javascript console.log("Hi") # Run JavaScript code
pcode> clear                         # Clear screen
pcode> exit                          # Exit pcode
```

### Available Tools (13)

| Tool | Description | Parameters |
|------|-------------|------------|
| `file_read` | Read file contents | `path`, `offset?`, `limit?` |
| `file_write` | Write content to file | `path`, `content`, `append?` |
| `process` | Execute system command | `command`, `args?`, `cwd?`, `timeout_ms?` |
| `llm` | Interact with language model | `prompt`, `max_tokens?`, `temperature?` |
| `token_estimate` | Estimate token count | `text`, `fast?` |
| `pmat` | Run code quality analysis | `command`, `path`, `language?` |
| `bash` | Execute bash commands | `command` |
| `dev_cli` | Run dev tools (rg, cargo, git) | `tool`, `args` |
| `fix` | Auto-fix code issues | `fix_type`, `path`, `dry_run?` |
| `coverage` | Real code coverage with tarpaulin | `path?`, `format?`, `exclude_files?` |
| `refactor` | AI-powered code refactoring | `path`, `auto_apply?`, `focus?` |
| `python` | Execute Python code securely | `code`, `timeout_ms?`, `stdin?`, `args?` |
| `javascript` | Execute JavaScript/TypeScript | `code`, `timeout_ms?`, `use_deno?`, `args?` |

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
| Test Coverage | ≥ 80% | ✅ 80.9% |
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
- Release build: ~5.2MB (✅ achieved <12MB target)

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

## 🤖 AI-Powered Refactoring

pcode now includes **AI-powered automatic refactoring** that combines PMAT analysis with intelligent suggestions!

### How It Works

1. **Analyzes code** with PMAT to identify complexity issues
2. **Generates suggestions** based on common refactoring patterns
3. **Enhances with AI** when API key is available (optional)
4. **Provides actionable fixes** with clear explanations

### Refactor Command Examples

```bash
# Analyze and suggest refactoring for a file
pcode> /refactor { "path": "src/complex.rs" }

# Focus on specific issues
pcode> /refactor { "path": "src/lib.rs", "focus": "complexity" }

# Future: Auto-apply refactoring (not yet implemented)
pcode> /refactor { "path": "src/main.rs", "auto_apply": true }
```

### Example Output

```json
{
  "status": "success",
  "suggestions_count": 2,
  "suggestions": [
    {
      "file": "complex.rs",
      "function": "process_data",
      "line": 45,
      "severity": "high",
      "issue": "Function has complexity of 35, exceeding threshold",
      "suggestion": "Refactoring suggestions:\n- Extract helper methods for nested conditionals\n- Split this function into multiple smaller functions\n- Consider extracting a class for this functionality"
    }
  ]
}
```

When AI is enabled (with `AI_STUDIO_API_KEY`), suggestions include:
- Refactored code examples
- Step-by-step transformation guides
- Estimated complexity reduction

## 🎨 Code Analysis with PMAT

pcode now includes **PMAT (Pragmatic Metrics for Agile Teams)** integration for code quality analysis! This is the first step towards full code execution capabilities.

### PMAT Commands

```bash
# Analyze code complexity (Python & Rust)
pcode> /pmat complexity src/
# Shows cyclomatic complexity for all functions
# Flags functions with complexity > 20

# Detect technical debt (SATD)
pcode> /pmat satd .
# Finds TODO, FIXME, HACK comments
# Identifies workarounds and temporary code

# Estimate test coverage
pcode> /pmat coverage tests/
# Estimates coverage based on test presence
# Shows uncovered lines and low coverage files

# Analyze test dependencies (TDG)
pcode> /pmat tdg tests/
# Calculates Test Dependency Graph score
# Identifies tests with dependencies or shared state
```

### Example Output

```bash
pcode> /pmat complexity test.py
🔧 Executing PMAT analysis...
✅ Success:
{
  "summary": {
    "max_complexity": 9,
    "average_complexity": 5.0,
    "total_functions": 4,
    "violations": 0
  },
  "details": [
    {"function": "simple_func", "complexity": 1},
    {"function": "complex_func", "complexity": 9}
  ]
}
```

### Security

PMAT runs Python code in a sandboxed environment with:
- No network access
- Limited file system access (read-only to source)
- 30-second timeout
- Memory limits

## 🏗️ Roadmap

### ✅ Completed: Code Execution (Phase 1)

We've successfully implemented the first phase of code execution:
- [x] PMAT integration with sandboxed Python execution
- [x] Complexity analysis for Python and Rust
- [x] Technical debt detection
- [x] Test coverage estimation
- [x] Test dependency graph (TDG) analysis
- [x] Secure code execution framework
- [x] Bash command execution tool
- [x] Development CLI tool integration (ripgrep, cargo, git, etc.)
- [x] Single command execution mode (--command flag)
- [x] Version flag support (--version/-V)

### 🎯 Next Milestone: Extended Code Execution

#### Phase 2: Extended PMAT Features
- [x] Add test coverage analysis
- [x] Implement test dependency graph (TDG) analysis
- [x] Support for JavaScript/TypeScript analysis
- [x] Support for Rust code analysis
- [x] Integration with AI for automatic refactoring
- [x] Real coverage integration with cargo-tarpaulin

#### Phase 3: General Code Execution
- [x] Implement sandboxed code execution for multiple languages:
  - [x] Python (via subprocess with security flags)
  - [x] JavaScript/TypeScript (via Deno/Node.js)
  - [ ] Rust (via cargo)
  - [ ] Shell scripts (carefully sandboxed)
- [ ] Add code compilation and build support
- [ ] Implement test runner integration
- [ ] Add debugging capabilities

#### Phase 3: Enhanced Development Tools
- [ ] Git integration for version control operations
- [ ] Code search and refactoring tools
- [ ] Dependency management (npm, cargo, pip)
- [ ] Linting and formatting integration
- [ ] Documentation generation

#### Phase 4: Platform Integration
- [ ] VSCode extension with full MCP support
- [ ] Neovim plugin
- [ ] GitHub Actions integration
- [ ] CI/CD pipeline support

### 🔧 Technical Requirements for Code Execution

1. **Security**: All code execution must be sandboxed using:
   - Linux: Landlock + namespaces + cgroups
   - macOS: Sandbox profiles + App Sandbox
   - Windows: AppContainer + Job Objects

2. **Resource Limits**:
   - Memory: Configurable limits (default 512MB)
   - CPU: Time limits for execution
   - Disk: Temporary workspace with quota
   - Network: Disabled by default

3. **Supported Operations**:
   - Run code analysis tools
   - Execute tests
   - Build projects
   - Run linters and formatters
   - Generate metrics and reports

### 📈 Success Metrics

- Code execution adds <100ms latency
- Binary size remains under 15MB
- 100% sandboxed execution (no escapes)
- Support for 80% of common development tasks

## 📝 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) 🦀
- Uses [Tokio](https://tokio.rs/) for async runtime
- Implements [MCP](https://github.com/modelcontextprotocol) for tool communication
- Designed for integration with [Claude](https://claude.ai/code)

---

**pcode** - Pragmatic AI Labs | [GitHub](https://github.com/paiml/pcode) | [Issues](https://github.com/paiml/pcode/issues)