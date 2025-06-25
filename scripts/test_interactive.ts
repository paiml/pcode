#!/usr/bin/env -S deno run --allow-run --allow-env

/**
 * Test pcode interactive mode
 * Demonstrates chat functionality with and without API key
 */

interface TestResult {
  success: boolean;
  output: string;
  error?: string;
}

async function runPcode(input: string, withApiKey = false): Promise<TestResult> {
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
  await writer.write(new TextEncoder().encode(input + "\nexit\n"));
  await writer.close();

  const { success, stdout, stderr } = await process.output();

  return {
    success,
    output: new TextDecoder().decode(stdout),
    error: stderr.length > 0 ? new TextDecoder().decode(stderr) : undefined,
  };
}

function extractChatResponse(output: string, _query: string): string {
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
  return responseLines.join("\n");
}

async function main() {
  console.log("🧪 Testing pcode interactive mode");
  console.log("=".repeat(50));

  // Test without API key
  console.log("\n📍 Test 1: Without API key (offline mode)");
  console.log("-".repeat(40));

  const result1 = await runPcode("tell me about this project");
  if (result1.success) {
    const response = extractChatResponse(result1.output, "tell me about this project");
    console.log(response);
  } else {
    console.error("❌ Test failed:", result1.error);
  }

  // Show API key instructions
  console.log("\n📍 Test 2: With API key");
  console.log("-".repeat(40));
  console.log("When AI_STUDIO_API_KEY is set, pcode uses Google AI Studio (Gemini).");
  console.log("The chat provides intelligent, context-aware responses.\n");
  console.log("To test with API key:");
  console.log("  export AI_STUDIO_API_KEY='your-key-here'");
  console.log("  ./target/release/pcode\n");
  console.log("Example queries:");
  console.log("  - tell me about this project");
  console.log("  - which agent SDK are you using?");
  console.log("  - summarize my README.md");
  console.log("  - how do I run tests?");
  console.log("  - what's the architecture?");
}

// Test function for Deno test
export async function testInteractiveMode(): Promise<void> {
  const result = await runPcode("help");
  if (!result.success) {
    throw new Error(`Interactive mode test failed: ${result.error}`);
  }

  if (!result.output.includes("Available Commands")) {
    throw new Error("Help command did not return expected output");
  }
}

// Run main if this is the main module
if (import.meta.main) {
  await main();
}
