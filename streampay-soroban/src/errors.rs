// errors.rs — All contract error codes.
//
// In Soroban, errors are u32 codes returned in the Result.
// Keep them grouped and documented — clients use these to show
// meaningful messages to users.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    // ─── Initialization ───────────────────────────────────────────────────────
    AlreadyInitialized      = 1,
    NotInitialized          = 2,

    // ─── Input validation ─────────────────────────────────────────────────────
    InvalidAmount           = 10,
    NoMilestones            = 11,
    TooManyMilestones       = 12,  // Max 10 milestones per contract
    InvalidMilestoneBps     = 13,  // BPS must sum to exactly 10_000

    // ─── Escrow state ─────────────────────────────────────────────────────────
    EscrowCompleted         = 20,  // All milestones resolved — contract is done
    EscrowCancelled         = 21,  // Contract was cancelled

    // ─── Milestone state ──────────────────────────────────────────────────────
    MilestoneNotFound           = 30,
    MilestoneAlreadyCompleted   = 31,  // mark_complete() called twice
    MilestoneNotPendingRelease  = 32,  // approve/auto_release called on wrong state
    MilestoneNotCompleted       = 33,  // completed_at timestamp missing
    MilestoneNotDisputed        = 34,  // resolve_dispute called on non-disputed milestone

    // ─── Dispute ──────────────────────────────────────────────────────────────
    NoDisputeWindow         = 40,  // dispute_window is 0 — manual approval only
    DisputeWindowOpen       = 41,  // auto_release called before window expires
    DisputeWindowClosed     = 42,  // dispute() called after window expired

    // ─── Release ──────────────────────────────────────────────────────────────
    ManualApprovalRequired  = 50,  // dispute_window is 0 — must use approve()
    CannotCancelWithPendingMilestones = 51,

    // ─── Math ─────────────────────────────────────────────────────────────────
    Overflow                = 99,
}
