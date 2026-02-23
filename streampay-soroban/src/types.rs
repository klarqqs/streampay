// types.rs — All data types for the StreamEscrow contract.
//
// In Soroban, types stored on-chain must derive contracttype.
// This makes them serializable to XDR for ledger storage.

use soroban_sdk::{contracttype, String};

// ─── Escrow status ────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum EscrowStatus {
    Active,      // Escrow is live — milestones can be completed
    Completed,   // All milestones resolved — contract is done
    Cancelled,   // Client cancelled — remaining funds refunded
}

// ─── Milestone status ─────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum MilestoneStatus {
    Pending,         // Not started — waiting for work + PR merge
    PendingRelease,  // PR merged, backend called mark_complete() — awaiting approval/timeout
    Released,        // Funds sent to developer ✅
    Disputed,        // Client raised a dispute — funds frozen
    Refunded,        // Funds returned to client (dispute resolved in client's favor)
}

// ─── Milestone ────────────────────────────────────────────────────────────────

/// A single unit of work with a proportional payout.
///
/// `bps` (basis points) defines what % of total_amount this milestone pays.
/// All milestone bps must sum to exactly 10_000 (= 100%).
///
/// Example for a $1000 escrow:
///   Milestone 1: title="Design",   bps=2000  → $200
///   Milestone 2: title="Backend",  bps=5000  → $500
///   Milestone 3: title="Delivery", bps=3000  → $300
#[contracttype]
#[derive(Clone, Debug)]
pub struct Milestone {
    /// Human-readable title (stored on-chain for transparency)
    pub title: String,

    /// A keyword the backend checks in the PR title/branch
    /// e.g. "feat/design" or "milestone-1"
    pub trigger_keyword: String,

    /// Basis points — this milestone's share of total_amount
    /// Must be > 0. All milestones must sum to 10_000.
    pub bps: u32,

    /// Current status
    pub status: MilestoneStatus,

    /// GitHub PR URL — set by backend when mark_complete() is called
    pub pr_url: Option<String>,

    /// Ledger timestamp when mark_complete() was called
    pub completed_at: Option<u64>,
}
