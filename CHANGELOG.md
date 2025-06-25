# Changelog

All notable changes to pcode will be documented in this file.

## [0.2.0] - 2025-06-25

### Added
- Sandboxed Python execution tool with security isolation
- Sandboxed JavaScript/TypeScript execution (Node.js/Deno support)
- Real code coverage integration with cargo-tarpaulin
- AI-powered automatic refactoring tool
- Code fixing capabilities (format, lint, test fixes)
- One-liner installation script
- GitHub Actions workflows for releases and CI

### Changed
- Fixed admin privilege requirements in tests
- Updated documentation to reflect 13 built-in tools
- Improved test coverage to 80.9% (83 tests)

### Technical Details
- Binary size: 5.2MB (well under 12MB target)
- Performance: <200ms first-token latency
- Security: Platform-specific sandboxing without admin privileges
- Tools: 13 built-in tools for comprehensive development support

## [0.1.0] - 2025-06-24

### Initial Release
- Core MCP protocol implementation
- Basic file and process tools
- LLM integration with Google AI Studio
- PMAT code analysis integration
- Interactive chat mode
- Security sandboxing framework