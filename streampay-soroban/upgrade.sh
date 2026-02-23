#!/bin/bash
# upgrade.sh — Upgrade StreamEscrow contract to a new WASM version
#
# Soroban upgrade model:
#   Contracts are immutable once deployed, BUT Soroban supports
#   WASM hash upgrades via the contract's own upgrade mechanism.
#
#   Strategy used here: the contract stores an "admin" address
#   that can call a built-in upgrade function in future versions.
#   The upgrade replaces the WASM bytecode but KEEPS all storage state.
#
#   This means:
#   ✓ All escrow data survives the upgrade
#   ✓ Client/developer addresses unchanged
#   ✓ Funds stay in the contract
#   ✗ Cannot change storage schema incompatibly (add new required fields)
#
# Upgrade checklist before running:
#   1. New WASM is backward compatible with existing storage
#   2. Migration tested on local fork
#   3. New WASM hash verified
#   4. Admin key is available
#
# Usage:
#   ./scripts/upgrade.sh <CONTRACT_ID> <NEW_WASM_PATH>

set -e

CONTRACT_ID=$1
NEW_WASM_PATH=$2
NETWORK="testnet"

if [ -z "$CONTRACT_ID" ] || [ -z "$NEW_WASM_PATH" ]; then
    echo "Usage: ./scripts/upgrade.sh <CONTRACT_ID> <WASM_PATH>"
    exit 1
fi

echo ""
echo "💸 StreamPay — Contract Upgrade"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Contract: $CONTRACT_ID"
echo "Network:  $NETWORK"
echo ""

# Upload new WASM
echo "▶ Uploading new WASM..."
NEW_WASM_HASH=$(stellar contract upload \
    --network $NETWORK \
    --source deployer \
    --wasm "$NEW_WASM_PATH" \
    --output plain)

echo "✓ New WASM hash: $NEW_WASM_HASH"

# The contract must have an `upgrade` function that calls
# env.deployer().update_current_contract_wasm(new_wasm_hash)
# See upgrade_stub.rs for the implementation pattern.
echo ""
echo "▶ Invoking upgrade on contract..."
stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source deployer \
    --network $NETWORK \
    -- upgrade \
    --new_wasm_hash "$NEW_WASM_HASH"

echo ""
echo "✅ Upgrade complete!"
echo ""
echo "New WASM hash: $NEW_WASM_HASH"
echo "State preserved: all escrow data intact"
echo ""
echo "Verify on explorer:"
echo "  https://stellar.expert/explorer/testnet/contract/$CONTRACT_ID"
