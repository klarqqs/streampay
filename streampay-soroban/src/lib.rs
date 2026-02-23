// lib.rs — StreamEscrow Soroban Contract
//
// Architecture:
//   types.rs    → all data structures (Escrow, Milestone, Status enums)
//   storage.rs  → read/write helpers for contract storage
//   errors.rs   → all error codes
//   events.rs   → all emitted events
//   lib.rs      → contract functions (public API)
//
// One contract instance = one escrow agreement.
// Milestones define proportional fund releases tied to GitHub PRs.

#![no_std]

mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod tests;

use soroban_sdk::{
    contract, contractimpl, token, Address, Env, String, Vec,
};

use errors::Error;
use events::Events;
use storage::Storage;
use types::{EscrowStatus, Milestone, MilestoneStatus};

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct StreamEscrow;

#[contractimpl]
impl StreamEscrow {

    // ─── Initialize ───────────────────────────────────────────────────────────

    /// Initialize the escrow contract.
    ///
    /// Called once by the client when deploying.
    ///
    /// # Arguments
    /// * `client`         - Address that deposits funds and approves releases
    /// * `developer`      - Address that receives funds on milestone completion
    /// * `backend`        - Trusted server address allowed to call mark_complete()
    /// * `token`          - USDC token contract address
    /// * `total_amount`   - Total USDC locked (in stroops — 7 decimal places)
    /// * `milestones`     - Vec of (title, trigger_keyword, bps) — bps must sum to 10000
    /// * `dispute_window` - Seconds client has to dispute after mark_complete (0 = manual approval always)
    ///
    /// # Milestone BPS
    /// BPS = basis points. 10000 = 100%. Each milestone gets a % of total_amount.
    /// Example: [3000, 3000, 4000] = 30% + 30% + 40%
    pub fn initialize(
        env: Env,
        client: Address,
        developer: Address,
        backend: Address,
        token: Address,
        total_amount: i128,
        milestones: Vec<Milestone>,
        dispute_window_secs: u64,
    ) -> Result<(), Error> {
        // Prevent re-initialization
        if Storage::is_initialized(&env) {
            return Err(Error::AlreadyInitialized);
        }

        // Validate inputs
        if total_amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if milestones.is_empty() {
            return Err(Error::NoMilestones);
        }
        if milestones.len() > 10 {
            return Err(Error::TooManyMilestones);
        }

        // Validate milestone BPS sums to exactly 10000
        let total_bps: u32 = milestones.iter().map(|m| m.bps).sum();
        if total_bps != 10_000 {
            return Err(Error::InvalidMilestoneBps);
        }

        // Require client authorization
        client.require_auth();

        // Pull USDC from client into this contract
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&client, &env.current_contract_address(), &total_amount);

        // Store everything
        Storage::set_client(&env, &client);
        Storage::set_developer(&env, &developer);
        Storage::set_backend(&env, &backend);
        Storage::set_token(&env, &token);
        Storage::set_total_amount(&env, total_amount);
        Storage::set_released_amount(&env, 0i128);
        Storage::set_status(&env, EscrowStatus::Active);
        Storage::set_dispute_window(&env, dispute_window_secs);
        Storage::set_milestones(&env, &milestones);
        Storage::set_initialized(&env);

        Events::initialized(
            &env,
            &client,
            &developer,
            total_amount,
            milestones.len() as u32,
        );

        Ok(())
    }

    // ─── Mark complete ────────────────────────────────────────────────────────

    /// Mark a milestone as complete after verifying a GitHub PR merge.
    ///
    /// Only callable by the backend address (set at initialization).
    /// This starts the dispute window timer if dispute_window > 0.
    /// If dispute_window == 0, client must call approve() manually.
    ///
    /// # Arguments
    /// * `milestone_index` - 0-based index of the milestone
    /// * `pr_url`          - GitHub PR URL (stored on-chain for transparency)
    pub fn mark_complete(
        env: Env,
        milestone_index: u32,
        pr_url: String,
    ) -> Result<(), Error> {
        // Only the backend can call this
        let backend = Storage::get_backend(&env)?;
        backend.require_auth();

        Self::assert_active(&env)?;

        let mut milestones = Storage::get_milestones(&env)?;
        let milestone = milestones
            .get(milestone_index)
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::Pending {
            return Err(Error::MilestoneAlreadyCompleted);
        }

        // Update milestone state
        let completed_at = env.ledger().timestamp();
        let updated = Milestone {
            status:       MilestoneStatus::PendingRelease,
            pr_url:       Some(pr_url.clone()),
            completed_at: Some(completed_at),
            ..milestone
        };

        milestones.set(milestone_index, updated);
        Storage::set_milestones(&env, &milestones);

        Events::milestone_completed(&env, milestone_index, &pr_url, completed_at);

        Ok(())
    }

    // ─── Approve ──────────────────────────────────────────────────────────────

    /// Client explicitly approves a milestone — releases funds immediately.
    ///
    /// Can be called any time after mark_complete(), even during dispute window.
    /// This is the fast path — no waiting needed if client is happy.
    pub fn approve(env: Env, milestone_index: u32) -> Result<(), Error> {
        let client = Storage::get_client(&env)?;
        client.require_auth();

        Self::assert_active(&env)?;
        Self::release_milestone(&env, milestone_index, false)?;

        Ok(())
    }

    // ─── Auto release ─────────────────────────────────────────────────────────

    /// Release funds for a milestone after the dispute window has expired.
    ///
    /// Callable by anyone — no authorization required.
    /// This makes the contract truly trustless after the window expires.
    pub fn auto_release(env: Env, milestone_index: u32) -> Result<(), Error> {
        Self::assert_active(&env)?;

        let milestones = Storage::get_milestones(&env)?;
        let milestone = milestones
            .get(milestone_index)
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::PendingRelease {
            return Err(Error::MilestoneNotPendingRelease);
        }

        let dispute_window = Storage::get_dispute_window(&env)?;
        if dispute_window == 0 {
            return Err(Error::ManualApprovalRequired);
        }

        let completed_at = milestone.completed_at.ok_or(Error::MilestoneNotCompleted)?;
        let now          = env.ledger().timestamp();
        let deadline     = completed_at
            .checked_add(dispute_window)
            .ok_or(Error::Overflow)?;

        if now < deadline {
            return Err(Error::DisputeWindowOpen);
        }

        Self::release_milestone(&env, milestone_index, true)?;

        Ok(())
    }

    // ─── Dispute ──────────────────────────────────────────────────────────────

    /// Client disputes a milestone within the dispute window.
    ///
    /// Freezes the milestone funds. Contract owner must resolve via resolve_dispute().
    pub fn dispute(env: Env, milestone_index: u32, reason: String) -> Result<(), Error> {
        let client = Storage::get_client(&env)?;
        client.require_auth();

        Self::assert_active(&env)?;

        let dispute_window = Storage::get_dispute_window(&env)?;
        if dispute_window == 0 {
            return Err(Error::NoDisputeWindow);
        }

        let mut milestones = Storage::get_milestones(&env)?;
        let milestone = milestones
            .get(milestone_index)
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::PendingRelease {
            return Err(Error::MilestoneNotPendingRelease);
        }

        // Check window is still open
        let completed_at = milestone.completed_at.ok_or(Error::MilestoneNotCompleted)?;
        let now          = env.ledger().timestamp();
        let deadline     = completed_at
            .checked_add(dispute_window)
            .ok_or(Error::Overflow)?;

        if now >= deadline {
            return Err(Error::DisputeWindowClosed);
        }

        let disputed = Milestone {
            status: MilestoneStatus::Disputed,
            ..milestone
        };
        milestones.set(milestone_index, disputed);
        Storage::set_milestones(&env, &milestones);

        Events::dispute_opened(&env, milestone_index, &reason, now);

        Ok(())
    }

    // ─── Resolve dispute ──────────────────────────────────────────────────────

    /// Resolve a disputed milestone — send funds to developer or refund client.
    ///
    /// Only callable by the backend (arbitrator for V1).
    /// V2: replace with decentralized arbitration oracle.
    pub fn resolve_dispute(
        env: Env,
        milestone_index: u32,
        release_to_developer: bool,
    ) -> Result<(), Error> {
        let backend = Storage::get_backend(&env)?;
        backend.require_auth();

        Self::assert_active(&env)?;

        let mut milestones = Storage::get_milestones(&env)?;
        let milestone = milestones
            .get(milestone_index)
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::Disputed {
            return Err(Error::MilestoneNotDisputed);
        }

        if release_to_developer {
            // Developer wins — same as normal release
            milestones.set(milestone_index, Milestone {
                status: MilestoneStatus::PendingRelease,
                ..milestone
            });
            Storage::set_milestones(&env, &milestones);
            Self::release_milestone(&env, milestone_index, false)?;
        } else {
            // Client wins — refund this milestone's amount
            let total_amount = Storage::get_total_amount(&env)?;
            let refund_amount = Self::milestone_amount(total_amount, milestone.bps);

            let token   = Storage::get_token(&env)?;
            let client  = Storage::get_client(&env)?;
            let token_client = token::Client::new(&env, &token);
            token_client.transfer(
                &env.current_contract_address(),
                &client,
                &refund_amount,
            );

            milestones.set(milestone_index, Milestone {
                status: MilestoneStatus::Refunded,
                ..milestone
            });
            Storage::set_milestones(&env, &milestones);

            let released = Storage::get_released_amount(&env)?;
            Storage::set_released_amount(&env, released + refund_amount);

            Events::dispute_resolved(&env, milestone_index, &client, refund_amount);
        }

        Self::check_completion(&env)?;

        Ok(())
    }

    // ─── Cancel ───────────────────────────────────────────────────────────────

    /// Cancel the escrow and refund all unreleased funds to client.
    ///
    /// Only callable by client, and only if no milestones are PendingRelease or Disputed.
    /// (Can't cancel mid-review — prevents client from cancelling to avoid payment.)
    pub fn cancel(env: Env) -> Result<(), Error> {
        let client = Storage::get_client(&env)?;
        client.require_auth();

        Self::assert_active(&env)?;

        let milestones = Storage::get_milestones(&env)?;

        // Ensure no milestone is in a "limbo" state
        for milestone in milestones.iter() {
            if matches!(
                milestone.status,
                MilestoneStatus::PendingRelease | MilestoneStatus::Disputed
            ) {
                return Err(Error::CannotCancelWithPendingMilestones);
            }
        }

        // Refund unreleased amount
        let total_amount    = Storage::get_total_amount(&env)?;
        let released_amount = Storage::get_released_amount(&env)?;
        let remaining       = total_amount
            .checked_sub(released_amount)
            .ok_or(Error::Overflow)?;

        if remaining > 0 {
            let token = Storage::get_token(&env)?;
            let token_client = token::Client::new(&env, &token);
            token_client.transfer(
                &env.current_contract_address(),
                &client,
                &remaining,
            );
        }

        Storage::set_status(&env, EscrowStatus::Cancelled);
        Events::cancelled(&env, &client, remaining);

        Ok(())
    }

    // ─── Views ────────────────────────────────────────────────────────────────

    /// Get the full milestone list.
    pub fn get_milestones(env: Env) -> Result<Vec<Milestone>, Error> {
        Storage::get_milestones(&env)
    }

    /// Get a single milestone.
    pub fn get_milestone(env: Env, index: u32) -> Result<Milestone, Error> {
        let milestones = Storage::get_milestones(&env)?;
        milestones.get(index).ok_or(Error::MilestoneNotFound)
    }

    /// Get overall escrow status.
    pub fn get_status(env: Env) -> Result<EscrowStatus, Error> {
        Storage::get_status(&env)
    }

    /// Get total and released amounts.
    pub fn get_balance(env: Env) -> Result<(i128, i128, i128), Error> {
        let total    = Storage::get_total_amount(&env)?;
        let released = Storage::get_released_amount(&env)?;
        let remaining = total.checked_sub(released).ok_or(Error::Overflow)?;
        Ok((total, released, remaining))
    }

    /// How many seconds remain in the dispute window for a milestone.
    pub fn time_until_auto_release(env: Env, milestone_index: u32) -> Result<u64, Error> {
        let milestones      = Storage::get_milestones(&env)?;
        let milestone       = milestones.get(milestone_index).ok_or(Error::MilestoneNotFound)?;
        let dispute_window  = Storage::get_dispute_window(&env)?;

        if milestone.status != MilestoneStatus::PendingRelease || dispute_window == 0 {
            return Ok(0);
        }

        let completed_at = milestone.completed_at.ok_or(Error::MilestoneNotCompleted)?;
        let now          = env.ledger().timestamp();
        let deadline     = completed_at.checked_add(dispute_window).ok_or(Error::Overflow)?;

        if now >= deadline {
            Ok(0)
        } else {
            Ok(deadline - now)
        }
    }

    // ─── Internal ─────────────────────────────────────────────────────────────

    fn assert_active(env: &Env) -> Result<(), Error> {
        match Storage::get_status(env)? {
            EscrowStatus::Active => Ok(()),
            EscrowStatus::Completed  => Err(Error::EscrowCompleted),
            EscrowStatus::Cancelled  => Err(Error::EscrowCancelled),
        }
    }

    fn milestone_amount(total: i128, bps: u32) -> i128 {
        total * bps as i128 / 10_000
    }

    fn release_milestone(env: &Env, milestone_index: u32, auto: bool) -> Result<(), Error> {
        let mut milestones = Storage::get_milestones(env)?;
        let milestone = milestones
            .get(milestone_index)
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::PendingRelease {
            return Err(Error::MilestoneNotPendingRelease);
        }

        // Calculate this milestone's share
        let total_amount  = Storage::get_total_amount(env)?;
        let release_amount = Self::milestone_amount(total_amount, milestone.bps);

        // Platform fee: 1%
        let fee_amount = release_amount / 100;
        let dev_amount = release_amount - fee_amount;

        let token        = Storage::get_token(env)?;
        let developer    = Storage::get_developer(env)?;
        let backend      = Storage::get_backend(env)?;  // fee goes to backend/platform
        let token_client = token::Client::new(env, &token);

        token_client.transfer(
            &env.current_contract_address(),
            &developer,
            &dev_amount,
        );
        token_client.transfer(
            &env.current_contract_address(),
            &backend,
            &fee_amount,
        );

        // Update released total
        let released = Storage::get_released_amount(env)?;
        Storage::set_released_amount(env, released + release_amount);

        // Mark milestone released
        milestones.set(milestone_index, Milestone {
            status: MilestoneStatus::Released,
            ..milestone
        });
        Storage::set_milestones(env, &milestones);

        Events::funds_released(env, milestone_index, &developer, dev_amount, auto);

        Self::check_completion(env)?;

        Ok(())
    }

    fn check_completion(env: &Env) -> Result<(), Error> {
        let milestones = Storage::get_milestones(env)?;

        // Completed when every milestone is Released or Refunded (no Pending/Disputed left)
        let all_done = milestones.iter().all(|m| {
            matches!(m.status, MilestoneStatus::Released | MilestoneStatus::Refunded)
        });

        if all_done {
            Storage::set_status(env, EscrowStatus::Completed);
            Events::completed(env);
        }

        Ok(())
    }
}
