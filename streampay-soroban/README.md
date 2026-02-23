# 💸 StreamEscrow — Soroban Contract

Milestone-based Git escrow on Stellar. Funds release when PRs merge.

---

## Architecture

```
StreamEscrow (one instance = one deal)
│
├── initialize()       → Lock USDC, define milestones
├── mark_complete()    → Backend confirms PR merged (starts dispute window)
├── approve()          → Client approves → instant release
├── auto_release()     → Anyone calls after 72h → trustless release
├── dispute()          → Client freezes funds within window
├── resolve_dispute()  → Backend arbitrates (V2: on-chain oracle)
└── cancel()           → Client cancels, gets unreleased funds back
```

**State machine:**
```
Milestone: Pending → PendingRelease → Released ✅
                                    → Disputed → Released / Refunded
Escrow:    Active  → Completed ✅
                   → Cancelled ↩️
```

---

## Prerequisites

```bash
# 1. Rust + wasm target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# 2. Stellar CLI
cargo install --locked stellar-cli --features opt

# 3. Generate keys
stellar keys generate deployer   --network testnet
stellar keys generate client     --network testnet
stellar keys generate developer  --network testnet
stellar keys generate backend    --network testnet

# 4. Fund with testnet XLM
stellar account fund $(stellar keys address deployer)  --network testnet
stellar account fund $(stellar keys address client)    --network testnet
```

---

## Build & Test

```bash
# Build
stellar contract build
# or: cargo build --target wasm32-unknown-unknown --release

# Test (runs all 15 tests in sandboxed Soroban env)
cargo test

# Test with output
cargo test -- --nocapture
```

---

## Deploy to Testnet

```bash
chmod +x scripts/deploy.sh
./scripts/deploy.sh

# Saves contract ID to deployments/testnet.json
```

---

## Initialize an Escrow

```bash
CONTRACT_ID=$(cat deployments/testnet.json | jq -r .contract_id)

stellar contract invoke \
  --id $CONTRACT_ID \
  --source client \
  --network testnet \
  -- initialize \
  --client    $(stellar keys address client) \
  --developer $(stellar keys address developer) \
  --backend   $(stellar keys address backend) \
  --token     <USDC_CONTRACT_ID> \
  --total_amount 1000_0000000 \
  --milestones '[
    {"title":"Design","trigger_keyword":"feat/design","bps":3000,"status":{"Pending":null},"pr_url":null,"completed_at":null},
    {"title":"Backend","trigger_keyword":"feat/backend","bps":4000,"status":{"Pending":null},"pr_url":null,"completed_at":null},
    {"title":"Delivery","trigger_keyword":"feat/delivery","bps":3000,"status":{"Pending":null},"pr_url":null,"completed_at":null}
  ]' \
  --dispute_window_secs 259200
```

---

## Invoke Contract Functions

```bash
# Backend marks PR merged
stellar contract invoke --id $CONTRACT_ID --source backend --network testnet \
  -- mark_complete --milestone_index 0 --pr_url "https://github.com/acme/api/pull/42"

# Client approves (instant release)
stellar contract invoke --id $CONTRACT_ID --source client --network testnet \
  -- approve --milestone_index 0

# Anyone triggers auto-release after 72h
stellar contract invoke --id $CONTRACT_ID --source deployer --network testnet \
  -- auto_release --milestone_index 0

# Client disputes within window
stellar contract invoke --id $CONTRACT_ID --source client --network testnet \
  -- dispute --milestone_index 0 --reason "Work doesn't match spec"

# Backend resolves dispute
stellar contract invoke --id $CONTRACT_ID --source backend --network testnet \
  -- resolve_dispute --milestone_index 0 --release_to_developer true

# Check status
stellar contract invoke --id $CONTRACT_ID --network testnet \
  -- get_balance

stellar contract invoke --id $CONTRACT_ID --network testnet \
  -- get_milestones
```

---

## Upgrade path

Soroban supports WASM upgrades — contract logic can be replaced while
all storage state (escrow data, funds) is preserved.

```bash
./scripts/upgrade.sh $CONTRACT_ID target/wasm32-unknown-unknown/release/stream_escrow.wasm
```

See `src/upgrade_stub.rs` for the full upgrade pattern and storage migration guide.

---

## Key design decisions

**Why one contract per escrow (not a factory)?**
Simpler to audit. Each deal is isolated. No shared state that can be drained.
When volume grows, a factory pattern can deploy instances cheaply.

**Why backend for mark_complete() instead of oracle?**
V1 pragmatism. GitHub doesn't have on-chain proofs yet.
V2 path: use a ZK proof of GitHub webhook signature, or an oracle network.

**Why BPS instead of fixed amounts?**
Proportional milestones survive renegotiation. If client and developer
agree to change the total, the proportions stay correct.

**Why persistent storage (not instance)?**
Escrow data must never expire mid-deal. Persistent storage costs more
rent but is appropriate for financial contracts.

---

## Roadmap

- [x] V1 — Stellar primitives (Python CLI)
- [x] V2 — Soroban contract (this)
- [ ] V3 — Python CLI updated to invoke Soroban contract
- [ ] V4 — GitHub App (no manual webhook setup)
- [ ] V5 — ZK proof of PR merge (remove backend trust)
- [ ] V6 — Factory contract (one deploy, many escrows)

---

## License

MIT
