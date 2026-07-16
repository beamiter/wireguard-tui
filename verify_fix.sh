#!/bin/bash
echo "Archived sudo-era helper: v0.4 must run as a normal user; see README.md." >&2
exit 2

# 验证 sudo 环境变量修复

echo "=== Verifying sudo HOME fix ==="
echo ""

echo "1. Current environment:"
echo "   USER: $USER"
echo "   HOME: $HOME"
echo ""

echo "2. Simulating sudo environment:"
echo "   When you run 'sudo ./app', these variables are set:"
echo ""

# 模拟 sudo 环境
export SUDO_USER="$USER"
export ORIGINAL_HOME="$HOME"

# 模拟 sudo 后的 HOME
TEST_HOME="/root"

echo "   SUDO_USER=$SUDO_USER"
echo "   HOME=$TEST_HOME (root's home)"
echo ""

echo "3. Testing get_real_home() logic:"
echo ""

# 模拟我们的逻辑
if [ -n "$SUDO_USER" ]; then
    REAL_HOME="/home/$SUDO_USER"
    echo "   ✓ Detected sudo, using: $REAL_HOME"
else
    REAL_HOME="$HOME"
    echo "   ✗ No sudo detected, using: $REAL_HOME"
fi

echo "   Downloads path: $REAL_HOME/Downloads"
echo ""

echo "4. Checking if path exists:"
if [ -d "$REAL_HOME/Downloads" ]; then
    echo "   ✓ Directory exists"

    CONF_COUNT=$(ls "$REAL_HOME/Downloads/"*.conf 2>/dev/null | wc -l)
    if [ $CONF_COUNT -gt 0 ]; then
        echo "   ✓ Found $CONF_COUNT .conf file(s)"
        ls "$REAL_HOME/Downloads/"*.conf 2>/dev/null | while read file; do
            echo "     - $(basename "$file")"
        done
    else
        echo "   ⚠ No .conf files in Downloads"
    fi
else
    echo "   ✗ Directory does not exist: $REAL_HOME/Downloads"
fi

echo ""
echo "=== Summary ==="
echo ""
echo "The fix should work when the TUI runs as the normal user:"
echo "  1. Run sudo -v only to refresh the credential cache"
echo "  2. Start the TUI without sudo so HOME remains /home/$USER"
echo "  3. The app finds Downloads at /home/$USER/Downloads"
echo "  4. It scans for .conf files successfully"
echo ""
echo "✓ Ready to test with: sudo -v"
echo "                      ./target/debug/wireguard-tui"
echo ""
