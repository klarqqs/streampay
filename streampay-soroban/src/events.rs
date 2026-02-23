// events.rs — Contract events.
//
// Soroban events are indexed by Horizon and can be consumed by
// your Python CLI, frontend, or any event listener.
//
// Event structure: env.events().publish((topic1, topic2, ...), data)
// Topics are used for filtering. Data is the payload.

use soroban_sdk::{Address, Env, String, symbol_short};

pub struct Events;

impl Events {

    pub fn initialized(
        env: &Env,
        client: &Address,
        developer: &Address,
        total_amount: i128,
        milestone_count: u32,
    ) {
        env.events().publish(
            (symbol_short!("init"),),
            (client, developer, total_amount, milestone_count),
        );
    }

    pub fn milestone_completed(
        env: &Env,
        milestone_index: u32,
        pr_url: &String,
        completed_at: u64,
    ) {
        env.events().publish(
            (symbol_short!("complete"), milestone_index),
            (pr_url.clone(), completed_at),
        );
    }

    pub fn funds_released(
        env: &Env,
        milestone_index: u32,
        developer: &Address,
        amount: i128,
        auto_released: bool,
    ) {
        env.events().publish(
            (symbol_short!("release"), milestone_index),
            (developer, amount, auto_released),
        );
    }

    pub fn dispute_opened(
        env: &Env,
        milestone_index: u32,
        reason: &String,
        opened_at: u64,
    ) {
        env.events().publish(
            (symbol_short!("dispute"), milestone_index),
            (reason.clone(), opened_at),
        );
    }

    pub fn dispute_resolved(
        env: &Env,
        milestone_index: u32,
        recipient: &Address,
        amount: i128,
    ) {
        env.events().publish(
            (symbol_short!("resolved"), milestone_index),
            (recipient, amount),
        );
    }

    pub fn cancelled(env: &Env, client: &Address, refunded: i128) {
        env.events().publish(
            (symbol_short!("cancel"),),
            (client, refunded),
        );
    }

    pub fn completed(env: &Env) {
        env.events().publish(
            (symbol_short!("done"),),
            (),
        );
    }
}
