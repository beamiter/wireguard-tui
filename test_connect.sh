#!/bin/bash
echo "Archived diagnostic: use the redacted troubleshooting flow in README.md/SECURITY.md." >&2
exit 2

# 测试 WireGuard 连接

echo "=== Testing WireGuard Connection ==="
echo ""

CONFIG_NAME="str-dub303"

echo "1. Checking if config exists..."
if sudo ls /etc/wireguard/$CONFIG_NAME.conf >/dev/null 2>&1; then
    echo "   ✓ Config exists: /etc/wireguard/$CONFIG_NAME.conf"
else
    echo "   ✗ Config not found!"
    echo "   Please import the config first using the TUI"
    exit 1
fi
echo ""

echo "2. Checking WireGuard installation..."
if command -v wg-quick >/dev/null 2>&1; then
    echo "   ✓ wg-quick found: $(which wg-quick)"
else
    echo "   ✗ WireGuard not installed"
    echo "   Install: sudo apt install wireguard"
    exit 1
fi
echo ""

echo "3. Checking WireGuard kernel module..."
if lsmod | grep -q wireguard; then
    echo "   ✓ WireGuard module loaded"
else
    echo "   ⚠ WireGuard module not loaded"
    echo "   Trying to load..."
    if sudo modprobe wireguard 2>&1; then
        echo "   ✓ Module loaded successfully"
    else
        echo "   ✗ Failed to load module"
        echo "   Your kernel may not support WireGuard"
    fi
fi
echo ""

echo "4. Testing connection..."
echo "   Running: sudo wg-quick up $CONFIG_NAME"
echo "   ─────────────────────────────────────"
echo ""

if sudo wg-quick up $CONFIG_NAME; then
    echo ""
    echo "   ✓ Connection successful!"
    echo ""
    echo "5. Checking status..."
    sudo wg show $CONFIG_NAME
    echo ""
    echo "6. Disconnecting..."
    sudo wg-quick down $CONFIG_NAME
else
    echo ""
    echo "   ✗ Connection failed!"
    echo ""
    echo "   Common issues:"
    echo "   1. Config file format error"
    echo "   2. Kernel doesn't support WireGuard"
    echo "   3. Network interface conflict"
    echo "   4. Firewall blocking"
    echo ""
    echo "   Check config file:"
    echo "   sudo cat /etc/wireguard/$CONFIG_NAME.conf"
fi

echo ""
echo "=== Test Complete ==="
