#!/bin/bash
set -e

echo "=== Agora Development Setup ==="

if ! command -v rustc &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust already installed: $(rustc --version)"
fi

if ! command -v node &> /dev/null; then
    echo "WARNING: Node.js not found. Install Node.js 18+ for Tauri frontend."
else
    echo "Node.js: $(node --version)"
fi

echo ""
echo "Installing Rust targets for Tauri..."
rustup target add x86_64-pc-windows-msvc 2>/dev/null || true
rustup target add x86_64-apple-darwin 2>/dev/null || true
rustup target add aarch64-apple-darwin 2>/dev/null || true

echo ""
echo "Installing Tauri CLI..."
cargo install tauri-cli 2>/dev/null || echo "Tauri CLI already installed"

echo ""
echo "=== Building core library ==="
cargo build

echo ""
echo "=== Running tests ==="
cargo test

echo ""
echo "=== Setup complete! ==="
echo ""
echo "To run the desktop app:"
echo "  cd desktop && cargo tauri dev"
echo ""
echo "To build for production:"
echo "  cd desktop && cargo tauri build"
