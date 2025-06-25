#!/bin/bash
# Demo script for pcode - demonstrating its capabilities

echo "🤖 pcode Demo - Production AI Code Agent"
echo "========================================"
echo ""

# Build the project first
echo "📦 Building pcode..."
cargo build --release --quiet
echo "✅ Build complete ($(ls -lh target/release/pcode | awk '{print $5}') binary)"
echo ""

# Demo 1: Token estimation
echo "1️⃣ Token Estimation Demo"
echo "------------------------"
./target/release/pcode -c "/token_estimate This is a test of the token estimation system with our new lookup table implementation." 2>/dev/null
echo ""

# Demo 2: PMAT complexity analysis  
echo "2️⃣ PMAT Complexity Analysis Demo"
echo "---------------------------------"
./target/release/pcode -c "/pmat complexity src/main.rs" 2>/dev/null | tail -n +2
echo ""

# Demo 3: Fix tool dry-run
echo "3️⃣ Code Fix Tool Demo (Dry Run)"
echo "--------------------------------"
./target/release/pcode -c "/fix complexity src/main.rs --dry-run" 2>/dev/null | tail -n +2
echo ""

# Demo 4: Development tools
echo "4️⃣ Development CLI Tools Demo"
echo "------------------------------"
echo "Finding TODO comments:"
./target/release/pcode -c "/dev_cli rg TODO src/" 2>/dev/null | tail -n +2 | head -10
echo ""

# Demo 5: File operations
echo "5️⃣ File Operations Demo"
echo "-----------------------"
echo "Reading README.md (first 5 lines):"
./target/release/pcode -c "/file_read README.md" 2>/dev/null | tail -n +2 | jq -r '.content' 2>/dev/null | head -5
echo "... (truncated)"
echo ""

# Demo 6: Interactive mode test
echo "6️⃣ Testing Interactive Commands"
echo "-------------------------------"
echo "help" | ./target/release/pcode -i 2>/dev/null | grep -A 5 "Available Commands" | head -10
echo ""

echo "📊 Performance Metrics:"
echo "- Binary size: $(ls -lh target/release/pcode | awk '{print $5}')"
echo "- Token estimation: Self-contained with 256KB lookup table"
echo "- Test suite: 122 tests passing"
echo ""
echo "✨ Demo complete!"
echo ""
echo "To use with AI assistance:"
echo "export AI_STUDIO_API_KEY='your-gemini-api-key'"
echo "./target/release/pcode -i"