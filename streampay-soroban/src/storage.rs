// storage.rs — Typed storage helpers for StreamEscrow.
//
// Soroban uses a key-value store on the ledger.
// We wrap every read/write in a typed helper so the rest of
// the contract never touches raw storage directly.
//
// Storage keys are kept in the DataKey enum — Soroban serializes
// these to XDR automatically via #[contracttype].
//
// TTL note: Soroban storage has three tiers:
//   - Persistent: survives indefinitely (costs more rent)
//   - Temporary:  expires after N ledgers
//   - Instance:   tied to contract instance lifetime
//
// We use Persistent for escrow data (can't let it expire mid-deal).

use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::errors::Error;
use crate::types::{EscrowStatus, Milestone};

// ─── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Initialized,
    Client,
    Developer,
    Backend,
    Token,
    TotalAmount,
    ReleasedAmount,
    Status,
    DisputeWindow,
    Milestones,
}

// ─── Storage helpers ──────────────────────────────────────────────────────────

pub struct Storage;

impl Storage {

    // ─── Initialization guard ─────────────────────────────────────────────────

    pub fn is_initialized(env: &Env) -> bool {
        env.storage().persistent().has(&DataKey::Initialized)
    }

    pub fn set_initialized(env: &Env) {
        env.storage().persistent().set(&DataKey::Initialized, &true);
    }

    // ─── Addresses ────────────────────────────────────────────────────────────

    pub fn get_client(env: &Env) -> Result<Address, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Client)
            .ok_or(Error::NotInitialized)
    }

    pub fn set_client(env: &Env, client: &Address) {
        env.storage().persistent().set(&DataKey::Client, client);
    }

    pub fn get_developer(env: &Env) -> Result<Address, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Developer)
            .ok_or(Error::NotInitialized)
    }

    pub fn set_developer(env: &Env, developer: &Address) {
        env.storage().persistent().set(&DataKey::Developer, developer);
    }

    pub fn get_backend(env: &Env) -> Result<Address, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Backend)
            .ok_or(Error::NotInitialized)
    }

    pub fn set_backend(env: &Env, backend: &Address) {
        env.storage().persistent().set(&DataKey::Backend, backend);
    }

    pub fn get_token(env: &Env) -> Result<Address, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Token)
            .ok_or(Error::NotInitialized)
    }

    pub fn set_token(env: &Env, token: &Address) {
        env.storage().persistent().set(&DataKey::Token, token);
    }

    // ─── Amounts ──────────────────────────────────────────────────────────────

    pub fn get_total_amount(env: &Env) -> Result<i128, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::TotalAmount)
            .ok_or(Error::NotInitialized)
    }

    pub fn set_total_amount(env: &Env, amount: i128) {
        env.storage().persistent().set(&DataKey::TotalAmount, &amount);
    }

    pub fn get_released_amount(env: &Env) -> Result<i128, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::ReleasedAmount)
            .ok_or(Error::NotInitialized)
    }

    pub fn set_released_amount(env: &Env, amount: i128) {
        env.storage().persistent().set(&DataKey::ReleasedAmount, &amount);
    }

    // ─── Status ───────────────────────────────────────────────────────────────

    pub fn get_status(env: &Env) -> Result<EscrowStatus, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Status)
            .ok_or(Error::NotInitialized)
    }

    pub fn set_status(env: &Env, status: EscrowStatus) {
        env.storage().persistent().set(&DataKey::Status, &status);
    }

    // ─── Dispute window ───────────────────────────────────────────────────────

    pub fn get_dispute_window(env: &Env) -> Result<u64, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::DisputeWindow)
            .ok_or(Error::NotInitialized)
    }

    pub fn set_dispute_window(env: &Env, secs: u64) {
        env.storage().persistent().set(&DataKey::DisputeWindow, &secs);
    }

    // ─── Milestones ───────────────────────────────────────────────────────────

    pub fn get_milestones(env: &Env) -> Result<Vec<Milestone>, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestones)
            .ok_or(Error::NotInitialized)
    }

    pub fn set_milestones(env: &Env, milestones: &Vec<Milestone>) {
        env.storage().persistent().set(&DataKey::Milestones, milestones);
    }
}
