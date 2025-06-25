/// Context and system prompts for pcode
pub const SYSTEM_PROMPT: &str = r#"You are pcode, a production-grade AI code agent with extreme performance and security requirements. You help users with software engineering tasks.

Key information about yourself:
- You are built in Rust for extreme performance (<200ms latency, <12MB binary)
- You use security sandboxing (Landlock on Linux, platform-specific on other OS)
- You have tools for file operations, process execution, and token estimation
- You follow PMAT quality standards: low complexity, high test coverage, zero technical debt
- You support interactive chat and tool execution

When users ask about the project or codebase, you can reference:
- The main specification is in docs/v1-spec.md
- Quality metrics are tracked in QUALITY.md
- The project follows extreme quality standards (80% test coverage, complexity <20)
- You are designed to be a Claude Code clone with similar functionality

Be helpful, concise, and technical. Focus on practical assistance with coding tasks."#;

pub const PROJECT_CONTEXT: &str = r#"pcode Project Information:

Architecture:
- Built with Rust and Tokio async runtime
- Uses MCP (Model Context Protocol) for tool communication
- Cap'n Proto for efficient serialization
- Platform-specific security sandboxing
- Token estimation with perfect hash tables

Available Tools:
- file_read: Read file contents with optional offset/limit
- file_write: Write content to files
- process: Execute system commands with timeout
- llm: Query language models (requires API key)
- token_estimate: Estimate token count for text

Quality Standards (PMAT):
- Cyclomatic complexity: ≤20 per function
- Test coverage: ≥80% (currently ~80.9%)
- Technical debt: Zero tolerance
- Algorithm complexity: O(n) or better
- Binary size: <12MB target

The project is open source (MIT license) and designed for production use with extreme performance and security requirements."#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_contains_key_information() {
        // Verify SYSTEM_PROMPT contains essential information
        assert!(SYSTEM_PROMPT.contains("pcode"));
        assert!(SYSTEM_PROMPT.contains("production-grade AI code agent"));
        assert!(SYSTEM_PROMPT.contains("Rust"));
        assert!(SYSTEM_PROMPT.contains("<200ms latency"));
        assert!(SYSTEM_PROMPT.contains("<12MB binary"));
        assert!(SYSTEM_PROMPT.contains("security sandboxing"));
        assert!(SYSTEM_PROMPT.contains("Landlock"));
        assert!(SYSTEM_PROMPT.contains("PMAT quality standards"));
        assert!(SYSTEM_PROMPT.contains("80% test coverage"));
        assert!(SYSTEM_PROMPT.contains("docs/v1-spec.md"));
        assert!(SYSTEM_PROMPT.contains("QUALITY.md"));
    }

    #[test]
    fn test_project_context_contains_architecture_info() {
        // Verify PROJECT_CONTEXT contains architecture details
        assert!(PROJECT_CONTEXT.contains("pcode Project Information"));
        assert!(PROJECT_CONTEXT.contains("Rust and Tokio"));
        assert!(PROJECT_CONTEXT.contains("MCP (Model Context Protocol)"));
        assert!(PROJECT_CONTEXT.contains("Cap'n Proto"));
        assert!(PROJECT_CONTEXT.contains("security sandboxing"));
        assert!(PROJECT_CONTEXT.contains("perfect hash tables"));
    }

    #[test]
    fn test_project_context_lists_all_tools() {
        // Verify all tools are documented
        assert!(PROJECT_CONTEXT.contains("file_read"));
        assert!(PROJECT_CONTEXT.contains("file_write"));
        assert!(PROJECT_CONTEXT.contains("process"));
        assert!(PROJECT_CONTEXT.contains("llm"));
        assert!(PROJECT_CONTEXT.contains("token_estimate"));
        assert!(PROJECT_CONTEXT.contains("optional offset/limit"));
        assert!(PROJECT_CONTEXT.contains("Execute system commands"));
        assert!(PROJECT_CONTEXT.contains("requires API key"));
    }

    #[test]
    fn test_project_context_quality_standards() {
        // Verify PMAT standards are documented
        assert!(PROJECT_CONTEXT.contains("Quality Standards (PMAT)"));
        assert!(PROJECT_CONTEXT.contains("Cyclomatic complexity: ≤20"));
        assert!(PROJECT_CONTEXT.contains("Test coverage: ≥80%"));
        assert!(PROJECT_CONTEXT.contains("currently ~80.9%"));
        assert!(PROJECT_CONTEXT.contains("Technical debt: Zero tolerance"));
        assert!(PROJECT_CONTEXT.contains("O(n) or better"));
        assert!(PROJECT_CONTEXT.contains("<12MB target"));
    }

    #[test]
    fn test_project_context_license_info() {
        // Verify license and usage info
        assert!(PROJECT_CONTEXT.contains("open source"));
        assert!(PROJECT_CONTEXT.contains("MIT license"));
        assert!(PROJECT_CONTEXT.contains("production use"));
        assert!(PROJECT_CONTEXT.contains("extreme performance"));
    }

    #[test]
    fn test_prompts_are_not_empty() {
        // Basic sanity checks
        assert!(SYSTEM_PROMPT.len() > 0);
        assert!(PROJECT_CONTEXT.len() > 0);
        assert!(SYSTEM_PROMPT.len() > 100);
        assert!(PROJECT_CONTEXT.len() > 100);
    }

    #[test]
    fn test_prompts_formatting() {
        // Verify prompts are properly formatted
        assert!(SYSTEM_PROMPT.starts_with("You are pcode"));
        assert!(PROJECT_CONTEXT.starts_with("pcode Project Information:"));

        // Check for proper line breaks and formatting
        assert!(SYSTEM_PROMPT.contains('\n'));
        assert!(PROJECT_CONTEXT.contains('\n'));
        assert!(PROJECT_CONTEXT.contains("- "));
    }
}
