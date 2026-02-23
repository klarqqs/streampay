#!/bin/bash
# deploy.sh — Deploy StreamEscrow to Stellar testnet
#
# Prerequisites:
#   stellar keys generate deployer --network testnet
#   stellar account fund <DEPLOYER_PUBLIC> --network testnet
#   cargo install --locked stellar-cli
#
# Usage:
#   chmod +x scripts/deploy.sh
#   ./scripts/deploy.sh

set -e

NETWORK="testnet"
CONTRACT_NAME="stream-escrow"
WASM_PATH="target/wasm32v1-none/release/stream_escrow.wasm"

echo ""
echo "💸 StreamPay — Soroban Deploy"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Network: $NETWORK"
echo ""

# ─── Step 1: Build ────────────────────────────────────────────────────────────

echo "▶ Step 1/4 — Building contract..."
stellar contract build

if [ ! -f "$WASM_PATH" ]; then
    echo "❌ WASM not found at $WASM_PATH"
    echo "   Run: cargo build --target wasm32-unknown-unknown --release"
    exit 1
fi

WASM_SIZE=$(wc -c < "$WASM_PATH")
echo "✓ Built. WASM size: ${WASM_SIZE} bytes"

# ─── Step 2: Upload WASM ─────────────────────────────────────────────────────

echo ""
echo "▶ Step 2/4 — Uploading WASM to testnet..."
WASM_HASH=$(stellar contract upload \
    --network $NETWORK \
    --source deployer \
    --wasm "$WASM_PATH")

echo "✓ WASM hash: $WASM_HASH"

# ─── Step 3: Deploy contract ──────────────────────────────────────────────────

echo ""
echo "▶ Step 3/4 — Deploying contract instance..."
CONTRACT_ID=$(stellar contract deploy \
    --network $NETWORK \
    --source deployer \
    --wasm-hash "$WASM_HASH")

echo "✓ Contract ID: $CONTRACT_ID"

# ─── Step 4: Save deployment info ────────────────────────────────────────────

echo ""
echo "▶ Step 4/4 — Saving deployment info..."

mkdir -p deployments
cat > deployments/testnet.json << EOF
{
  "network": "$NETWORK",
  "contract_id": "$CONTRACT_ID",
  "wasm_hash": "$WASM_HASH",
  "deployed_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "deployer": "$(stellar keys address deployer)"
}
EOF

echo "✓ Saved to deployments/testnet.json"

# ─── Done ─────────────────────────────────────────────────────────────────────

echo ""
echo "✅ Deployment complete!"
echo ""
echo "Contract ID: $CONTRACT_ID"
echo ""
echo "── Next: Initialize the escrow ──────────────────────────────────────────"
echo ""
echo "stellar contract invoke \\"
echo "  --id $CONTRACT_ID \\"
echo "  --source client \\"
echo "  --network $NETWORK \\"
echo "  -- initialize \\"
echo "  --client  \$(stellar keys address client) \\"
echo "  --developer \$(stellar keys address developer) \\"
echo "  --backend \$(stellar keys address backend) \\"
echo "  --token  <USDC_CONTRACT_ID> \\"
echo "  --total_amount 100_0000000 \\"
echo "  --milestones '[{...}]' \\"
echo "  --dispute_window_secs 259200"
echo ""
echo "── Testnet USDC ─────────────────────────────────────────────────────────"
echo "   Deploy a mock token or use the Stellar lab token:"
echo "   https://laboratory.stellar.org"
echo ""
echo "── Explorer ─────────────────────────────────────────────────────────────"
echo "   https://stellar.expert/explorer/testnet/contract/$CONTRACT_ID"
echo ""
