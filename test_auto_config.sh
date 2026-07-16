#!/bin/bash
echo "Archived test: v0.4 intentionally does not store provider credentials." >&2
exit 2

# Test script for auto-config generation

set -e

echo "🧪 Testing Auto-Config Generation"
echo "=================================="
echo ""

CONFIG_DIR="$HOME/.config/wireguard-tui"
CONFIG_FILE="$CONFIG_DIR/config.toml"
BACKUP_FILE="$CONFIG_DIR/config.toml.backup"

# Backup existing config if present
if [ -f "$CONFIG_FILE" ]; then
    echo "📦 Backing up existing config..."
    cp "$CONFIG_FILE" "$BACKUP_FILE"
    rm "$CONFIG_FILE"
    echo "✓ Backup created at: $BACKUP_FILE"
fi

echo ""
echo "🔍 Test 1: Config auto-generation"
echo "----------------------------------"

# Build the project first
echo "Building project..."
cargo build --release 2>&1 | tail -3

echo ""
echo "Starting application (will exit immediately)..."
timeout 2s ./target/release/wireguard-tui || true

# Check if config was created
if [ -f "$CONFIG_FILE" ]; then
    echo "✅ Test 1 PASSED: Config file auto-generated"
    echo ""
    echo "Generated config:"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    head -20 "$CONFIG_FILE"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
else
    echo "❌ Test 1 FAILED: Config file not created"
    exit 1
fi

echo ""
echo "🔍 Test 2: Config file format"
echo "------------------------------"

# Check if config contains required fields
if grep -q "username" "$CONFIG_FILE" && \
   grep -q "password" "$CONFIG_FILE" && \
   grep -q "auto_download" "$CONFIG_FILE"; then
    echo "✅ Test 2 PASSED: Config contains all required fields"
else
    echo "❌ Test 2 FAILED: Config missing required fields"
    exit 1
fi

echo ""
echo "🔍 Test 3: Config file has comments"
echo "------------------------------------"

# Check if config has helpful comments
if grep -q "# WireGuard TUI Configuration" "$CONFIG_FILE"; then
    echo "✅ Test 3 PASSED: Config has helpful comments"
else
    echo "❌ Test 3 FAILED: Config missing comments"
    exit 1
fi

echo ""
echo "🔍 Test 4: Template values present"
echo "-----------------------------------"

# Check if template values are present
if grep -q 'username = "your-vpn-username"' "$CONFIG_FILE" && \
   grep -q 'password = "your-vpn-password"' "$CONFIG_FILE"; then
    echo "✅ Test 4 PASSED: Template values present"
else
    echo "❌ Test 4 FAILED: Template values missing"
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ✅ All Tests Passed!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Cleanup - restore backup if exists
if [ -f "$BACKUP_FILE" ]; then
    echo "🔄 Restoring original config..."
    mv "$BACKUP_FILE" "$CONFIG_FILE"
    echo "✓ Original config restored"
else
    echo "⚠️  Test config left at: $CONFIG_FILE"
    echo "   (This is a new installation)"
fi

echo ""
echo "Summary:"
echo "• Config auto-generation: ✅ Working"
echo "• Template format: ✅ Correct"
echo "• Comments included: ✅ Yes"
echo "• Ready to use: ✅ Yes"
echo ""
echo "Next step: Edit $CONFIG_FILE with your credentials"
