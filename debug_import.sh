#!/bin/bash
# 调试导入功能

echo "=== Debug Import Functionality ==="
echo ""

echo "1. Checking environment..."
echo "   HOME: $HOME"
echo "   Downloads: $HOME/Downloads"
echo ""

echo "2. Checking .conf files..."
if ls "$HOME/Downloads/"*.conf 2>/dev/null; then
    COUNT=$(ls "$HOME/Downloads/"*.conf 2>/dev/null | wc -l)
    echo ""
    echo "   Found $COUNT .conf file(s)"
else
    echo "   No .conf files found"
fi
echo ""

echo "3. Running scan test..."
if [ -f "./test_scan" ]; then
    ./test_scan
else
    echo "   Compiling test..."
    rustc examples/test_scan.rs -o test_scan
    ./test_scan
fi
echo ""

echo "4. Testing TUI import (with debug output)..."
echo "   Run: RUST_LOG=debug sudo ./target/debug/wireguard-tui"
echo "   Then press 'i' to see debug output"
echo ""

echo "=== Suggested Actions ==="
echo ""
echo "If scan test found files but TUI doesn't:"
echo "  1. Run: sudo ./target/debug/wireguard-tui 2>&1 | tee tui-debug.log"
echo "  2. Press 'i'"
echo "  3. Check tui-debug.log for DEBUG messages"
echo ""
