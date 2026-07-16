#!/bin/bash
echo "Archived interactive capture helper: use cargo test and do not record raw TUI output." >&2
exit 2

# 测试导入功能的 UI 交互

echo "=== Testing Import UI ==="
echo ""
echo "This will launch the TUI in debug mode."
echo "Press 'i' to test import, then 'q' to quit."
echo ""
echo "Starting in 2 seconds..."
sleep 2

# 运行应用并捕获 stderr（调试输出）
./target/debug/wireguard-tui 2>&1 &
APP_PID=$!

echo ""
echo "App started with PID: $APP_PID"
echo ""
echo "Instructions:"
echo "  1. Press 'i' to test import"
echo "  2. Check if str-dub303.conf is listed"
echo "  3. Press 'q' to quit"
echo ""
echo "Debug output will show below:"
echo "─────────────────────────────────────"

# 等待用户操作
wait $APP_PID
