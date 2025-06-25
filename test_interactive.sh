#!/bin/bash
# Test interactive mode responses

echo "Testing pcode interactive mode..."
echo ""

# Test without API key
echo "=== Testing without API key ==="
unset AI_STUDIO_API_KEY
echo -e "tell me about this project\nexit" | ./target/release/pcode 2>/dev/null | grep -A20 "tell me about"

echo ""
echo "=== With API key, it would use the LLM for intelligent responses ==="
echo "Set AI_STUDIO_API_KEY to enable full AI features"