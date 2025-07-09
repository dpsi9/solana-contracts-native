#!/bin/bash

# Build script for Solana native contracts

echo "🔨 Building Solana Native Contracts..."

# Build all contracts
echo "Building all contracts..."
cargo build

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    
    echo "📁 Generated program files:"
    echo "  - target/debug/escrow.so"
    echo "  - target/debug/staking.so" 
    echo "  - target/debug/vault.so"
    echo "  - target/debug/marketplace.so"
    echo "  - target/debug/governance.so"
    
    echo ""
    echo "🚀 To deploy locally:"
    echo "  solana-test-validator"
    echo "  solana program deploy target/debug/escrow.so"
    
else
    echo "❌ Build failed!"
    exit 1
fi
