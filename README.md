# pcode

Production-grade AI code agent with extreme performance and security requirements.

## Features

- **Extreme Performance**: <200ms first-token latency, <12MB binary size
- **Security First**: Platform-specific sandboxing (Landlock, macOS Sandbox, AppContainer)
- **Self-Contained**: Token estimation without external dependencies
- **MCP Protocol**: Standard tool communication via Cap'n Proto
- **Cross-Platform**: Linux, macOS, and Windows support

## Building

```bash
# Install dependencies
make install-deps

# Build debug version
make build

# Build optimized release
make release

# Run tests
make test

# Run with coverage
make test-coverage
```

## Configuration

### AI Studio API Key

To use the LLM tool with Google AI Studio:

1. Get your API key from [Google AI Studio](https://aistudio.google.com/app/apikey)
2. Set the environment variable:
   ```bash
   export AI_STUDIO_API_KEY=your_api_key_here
   ```
   Or create a `.env` file (see `.env.example`)

## Usage

```bash
# Run with default settings
./target/x86_64-unknown-linux-musl/release/pcode

# Run with debug logging
./target/x86_64-unknown-linux-musl/release/pcode --debug

# Specify working directory
./target/x86_64-unknown-linux-musl/release/pcode --workdir /path/to/project

# Disable sandbox (not recommended)
./target/x86_64-unknown-linux-musl/release/pcode --no-sandbox

# With API key
AI_STUDIO_API_KEY=your_key ./target/x86_64-unknown-linux-musl/release/pcode
```

## Development

This project enforces extreme quality standards:
- Cyclomatic complexity: 10-20 max per function
- Test Dependency Graph: <1.0 per function
- Zero tolerance for technical debt
- 80% test coverage minimum
- Low algorithmic complexity

Run quality checks:
```bash
make lint
make complexity
make audit
```

## Architecture

See `docs/v1-spec.md` for detailed technical specification.

## License

MIT License - see LICENSE file for details.
