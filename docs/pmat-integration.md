# PMAT Integration Design

## Overview

This document outlines the design for integrating PMAT (Pragmatic Metrics for Agile Teams) capabilities into pcode through the MCP (Model Context Protocol) tool system.

## Why PMAT Integration is Critical

Currently, pcode can read and write files but cannot execute code or run analysis tools. This severely limits its usefulness as an AI code agent. PMAT integration would enable:

1. **Code Quality Analysis**: Measure cyclomatic complexity, test coverage, technical debt
2. **Automated Refactoring**: Identify and fix code smells based on metrics
3. **Test Generation**: Create tests for uncovered code paths
4. **Architecture Analysis**: Visualize dependencies and identify coupling issues
5. **Continuous Improvement**: Track quality metrics over time

## Implementation Plan

### Phase 1: MCP Tool for PMAT (MVP)

Create a new MCP tool that can execute PMAT analysis:

```rust
pub struct PmatTool {
    workspace: PathBuf,
    python_path: PathBuf,
}

impl Tool for PmatTool {
    fn name(&self) -> &str { "pmat" }
    
    async fn execute(&self, params: Value) -> Result<Value, ToolError> {
        // Parameters:
        // - command: "complexity" | "coverage" | "tdg" | "satd" | "all"
        // - path: file or directory to analyze
        // - options: language-specific options
        
        // Execute PMAT in sandboxed environment
        // Return structured metrics data
    }
}
```

### Phase 2: Sandboxed Python Execution

PMAT tools are typically written in Python. We need safe execution:

1. **Option A: Embedded Python**
   - Use PyO3 to embed Python interpreter
   - Better control but increases binary size
   - Can pre-compile PMAT tools

2. **Option B: Subprocess with Sandbox**
   - Use existing Python installation
   - Apply platform-specific sandboxing
   - More flexible but requires Python on system

### Phase 3: Metrics Integration

```rust
#[derive(Serialize, Deserialize)]
pub struct PmatMetrics {
    pub complexity: ComplexityReport,
    pub coverage: CoverageReport,
    pub technical_debt: Vec<SatdItem>,
    pub test_dependency_graph: TdgReport,
}

#[derive(Serialize, Deserialize)]
pub struct ComplexityReport {
    pub functions: Vec<FunctionComplexity>,
    pub max_complexity: u32,
    pub average_complexity: f64,
    pub violations: Vec<ComplexityViolation>,
}
```

### Phase 4: AI-Powered Improvements

Once PMAT is integrated, pcode can:

1. **Analyze code** → Identify issues
2. **Generate fixes** → Use LLM to refactor
3. **Verify improvements** → Re-run PMAT
4. **Create tests** → Improve coverage
5. **Document changes** → Explain improvements

## Security Considerations

### Sandboxing Requirements

1. **File System**: Read-only access to source, write to temp only
2. **Network**: Completely disabled
3. **Process**: No spawning of child processes
4. **Memory**: Limited to 256MB for analysis
5. **CPU**: 30-second timeout for analysis

### Platform-Specific Implementation

```rust
// Linux: Landlock + seccomp
pub fn sandbox_pmat_linux(config: &PmatConfig) -> Result<()> {
    // Apply Landlock rules
    // Set up seccomp filters
    // Create namespace
}

// macOS: Sandbox profile
pub fn sandbox_pmat_macos(config: &PmatConfig) -> Result<()> {
    // Apply sandbox profile
    // Set entitlements
}

// Windows: Job Objects
pub fn sandbox_pmat_windows(config: &PmatConfig) -> Result<()> {
    // Create job object
    // Set process limits
}
```

## Example Usage

```bash
pcode> /pmat complexity src/
🔧 Executing PMAT analysis...
📊 Complexity Report:
  - Maximum complexity: 18 (src/chat.rs:handle_command)
  - Average complexity: 4.2
  - Functions exceeding limit (>20): 0
  ✅ All functions within complexity limits

pcode> /pmat coverage
🔧 Running coverage analysis...
📊 Coverage Report:
  - Line coverage: 75.2%
  - Branch coverage: 68.4%
  - Uncovered files: 3
  ⚠️  Below target of 80%

pcode> generate tests for uncovered code
🤖 Analyzing uncovered code paths...
📝 Generating test cases...
✅ Created 5 new test cases
🔧 Re-running coverage...
📊 New coverage: 82.1% ✅
```

## Benefits

1. **Immediate Value**: PMAT provides actionable metrics
2. **Quality Gates**: Enforce standards automatically
3. **Learning Tool**: AI learns from metrics to improve code
4. **Productivity**: Automate routine quality checks
5. **Proof of Concept**: Demonstrates safe code execution

## Next Steps

1. Create proof-of-concept PMAT tool
2. Implement basic sandboxing
3. Test with real projects
4. Extend to other analysis tools
5. Build general code execution framework

## Success Criteria

- [ ] Can analyze Python, JavaScript, and Rust code
- [ ] Metrics accurate within 5% of native tools
- [ ] Zero security escapes in sandbox
- [ ] Analysis completes in <5 seconds for average project
- [ ] Integration adds <500KB to binary size