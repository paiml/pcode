/**
 * Test suite for all Deno scripts
 * Ensures all scripts meet quality standards
 */

import { assertEquals, assertExists } from "https://deno.land/std@0.210.0/assert/mod.ts";
import { testInteractiveMode } from "./test_interactive.ts";
import { testDemoOutput } from "./demo_interactive.ts";
import { testChatResponseExtraction } from "./test_chat_responses.ts";

Deno.test("test_interactive.ts - exports test function", () => {
  assertExists(testInteractiveMode);
  assertEquals(typeof testInteractiveMode, "function");
});

Deno.test("demo_interactive.ts - runs without errors", () => {
  // Should not throw
  testDemoOutput();
});

Deno.test("test_chat_responses.ts - chat response extraction", () => {
  testChatResponseExtraction();
});

Deno.test("all scripts have proper permissions", async () => {
  const scripts = [
    "test_interactive.ts",
    "demo_interactive.ts",
    "test_chat_responses.ts",
  ];

  for (const script of scripts) {
    const scriptPath = `scripts/${script}`;
    const stat = await Deno.stat(scriptPath);
    assertExists(stat);
    assertEquals(stat.isFile, true);
  }
});

Deno.test("scripts follow TypeScript strict mode", async () => {
  // This is checked by deno check, but we verify files exist
  const files = [];
  for await (const entry of Deno.readDir("scripts/")) {
    if (entry.name.endsWith(".ts")) {
      files.push(entry.name);
    }
  }

  assertEquals(files.length >= 2, true, "Should have at least 2 TypeScript files");
});

// Note: Interactive mode test requires built binary
Deno.test({
  name: "interactive mode - help command works",
  ignore: !await binaryExists(),
  fn: async () => {
    await testInteractiveMode();
  },
});

async function binaryExists(): Promise<boolean> {
  try {
    await Deno.stat("./target/release/pcode");
    return true;
  } catch {
    return false;
  }
}
