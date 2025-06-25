#!/usr/bin/env -S deno run --allow-run

/**
 * Demo script for pcode interactive mode
 * Shows available features and commands
 */

const BOLD = "\x1b[1m";
const RESET = "\x1b[0m";
const BLUE = "\x1b[34m";
const GREEN = "\x1b[32m";
const YELLOW = "\x1b[33m";

function printHeader(text: string): void {
  console.log(`\n${BOLD}${BLUE}${text}${RESET}`);
  console.log("=".repeat(text.length));
}

function printSection(title: string, content: string[]): void {
  console.log(`\n${GREEN}${title}:${RESET}`);
  content.forEach((line) => console.log(`  ${line}`));
}

export function showDemo(): void {
  printHeader("🤖 pcode Interactive Mode Demo");

  console.log("\npcode now supports interactive chat mode similar to Claude!");

  printSection("To start interactive mode", [
    "./target/release/pcode",
    "./target/release/pcode --interactive",
  ]);

  printSection("Available commands in interactive mode", [
    `${YELLOW}help${RESET}                    - Show help`,
    `${YELLOW}tools${RESET}                   - List available tools`,
    `${YELLOW}/file_read <path>${RESET}       - Read a file`,
    `${YELLOW}/file_write <path> <content>${RESET} - Write to a file`,
    `${YELLOW}/process <command>${RESET}      - Execute a command`,
    `${YELLOW}/llm <prompt>${RESET}           - Query LLM (requires AI_STUDIO_API_KEY)`,
    `${YELLOW}/token_estimate <text>${RESET}  - Estimate tokens`,
    `${YELLOW}clear${RESET}                   - Clear screen`,
    `${YELLOW}exit${RESET}                    - Exit`,
  ]);

  printSection("Natural language queries", [
    "tell me about this project",
    "what tools are available?",
    "summarize my README.md",
    "how do I contribute?",
    "explain the architecture",
  ]);

  console.log(`\n${BOLD}Try it now!${RESET}`);
}

// Test function
export function testDemoOutput(): void {
  // This test verifies the demo can run without errors
  showDemo();
}

if (import.meta.main) {
  showDemo();
}
