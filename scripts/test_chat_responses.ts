#!/usr/bin/env -S deno run --allow-run --allow-env

/**
 * Test chat responses with and without API key
 * Demonstrates pcode's intelligent chat capabilities
 */

const BLUE = "\x1b[34m";
const GREEN = "\x1b[32m";
const YELLOW = "\x1b[33m";
const RESET = "\x1b[0m";
const BOLD = "\x1b[1m";

interface TestCase {
  query: string;
  description: string;
}

async function testChatResponse(query: string, withApiKey: boolean): Promise<string> {
  const env = { ...Deno.env.toObject() };
  if (!withApiKey) {
    delete env.AI_STUDIO_API_KEY;
  }

  const cmd = new Deno.Command("./target/release/pcode", {
    args: ["--no-sandbox"],
    stdin: "piped",
    stdout: "piped",
    stderr: "piped",
    env,
  });

  const process = cmd.spawn();
  const writer = process.stdin.getWriter();
  const encoder = new TextEncoder();

  await writer.write(encoder.encode(`${query}\nexit\n`));
  await writer.close();

  const { stdout } = await process.output();
  const output = new TextDecoder().decode(stdout);

  return extractResponse(output, query);
}

function extractResponse(output: string, _query: string): string {
  const lines = output.split("\n");

  // Find where the welcome message ends
  const welcomeEnd = lines.findIndex((line) => line.includes("Type 'help' for available commands"));
  if (welcomeEnd === -1) return "Welcome message not found";

  // Extract all content after the welcome message and before "Goodbye"
  const responseLines: string[] = [];
  let foundContent = false;

  for (let i = welcomeEnd + 2; i < lines.length; i++) {
    const line = lines[i];
    if (line.includes("Goodbye")) break;
    if (line.trim()) {
      responseLines.push(line);
      foundContent = true;
    }
  }

  if (!foundContent) return "No response found";
  return responseLines.join("\n").trimEnd();
}

export async function runTests(): Promise<void> {
  console.log(`${BOLD}${BLUE}=== Testing pcode chat responses ===${RESET}`);
  console.log("");

  // Test without API key
  console.log(`${GREEN}1. Without API key (offline mode):${RESET}`);
  console.log("-".repeat(35));

  try {
    const offlineResponse = await testChatResponse("tell me about this project", false);
    console.log(offlineResponse);
  } catch (error) {
    console.error(
      `${YELLOW}Error testing offline mode:${RESET}`,
      error instanceof Error ? error.message : String(error),
    );
  }

  console.log("");
  console.log(`${GREEN}2. With API key set:${RESET}`);
  console.log("-".repeat(19));
  console.log(
    "If AI_STUDIO_API_KEY is set, pcode will use Google AI Studio (Gemini) for responses.",
  );
  console.log("The chat will provide intelligent, context-aware responses about the project.");
  console.log("");
  console.log("To test with API key:");
  console.log("  export AI_STUDIO_API_KEY='your-key-here'");
  console.log("  ./target/release/pcode");
  console.log("");
  console.log("Example queries:");
  console.log("  - tell me about this project");
  console.log("  - what tools are available?");
  console.log("  - summarize my README.md");
  console.log("  - which agent SDK are you using?");
  console.log("  - how do I run tests?");
}

// Test function for test suite
export function testChatResponseExtraction(): void {
  const sampleOutput = `
Welcome to pcode interactive mode!
Type 'help' for available commands, 'exit' to quit.

pcode> tell me about this project
pcode is a production-grade AI code agent...

pcode> exit
Goodbye!`;

  const extracted = extractResponse(sampleOutput, "tell me about this project");
  if (!extracted.includes("pcode is a production-grade AI code agent")) {
    throw new Error("Failed to extract chat response correctly");
  }
}

if (import.meta.main) {
  await runTests();
}
