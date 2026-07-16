#!/bin/bash
echo "Archived v0.3 helper: use README.md for the v0.4 build and startup flow." >&2
exit 2

set -e

echo "🔒 WireGuard TUI Manager - Setup Script"
echo "========================================"
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install it first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✓ Rust found: $(rustc --version)"
echo ""

# Build the project
echo "📦 Building WireGuard TUI..."
cargo build --release
echo "✓ Build complete!"
echo ""

# Note about auto-config
echo "ℹ️  Note: The application will auto-generate config.toml on first run!"
echo ""

# Summary
echo "✨ Setup complete!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Next Steps:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "1️⃣  Run the application (config will be auto-generated):"
echo "   ./target/release/wireguard-tui"
echo ""
echo "2️⃣  The app will create: ~/.config/wireguard-tui/config.toml"
echo ""
echo "3️⃣  Edit the config with your StrongVPN credentials:"
echo "   nano ~/.config/wireguard-tui/config.toml"
echo ""
echo "4️⃣  Update these two lines:"
echo "   username = \"your-actual-username\""
echo "   password = \"your-actual-password\""
echo ""
echo "5️⃣  Run the app again and press 'r' to download configs!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Get Your Credentials:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "• Login: https://strongtech.org/account/"
echo "• Click: 'Account Setup Instructions'"
echo "• Find: 'VPN Account Information'"
echo "• Note: Username starts with 'a'"
echo ""
echo "💡 Tip: Use VPN credentials, NOT website login!"
echo ""
