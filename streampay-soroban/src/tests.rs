#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env, String, Vec,
};

use crate::{
    types::{EscrowStatus, Milestone, MilestoneStatus},
    StreamEscrow, StreamEscrowClient,
};

// ─── Test setup ───────────────────────────────────────────────────────────────

struct TestEnv {
    env:       Env,
    token:     Address,
    client:    Address,
    developer: Address,
    backend:   Address,
    contract_id: soroban_sdk::Address,
}

fn setup() -> TestEnv {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    let token_client = token::StellarAssetClient::new(&env, &token);

    let client    = Address::generate(&env);
    let developer = Address::generate(&env);
    let backend   = Address::generate(&env);

    token_client.mint(&client, &1_000_000_0000000i128);

    let contract_id = env.register_contract(None, StreamEscrow);

    TestEnv { env, token, client, developer, backend, contract_id }
}

fn client<'a>(t: &'a TestEnv) -> StreamEscrowClient<'a> {
    StreamEscrowClient::new(&t.env, &t.contract_id)
}

fn make_milestones(env: &Env) -> Vec<Milestone> {
    let mut ms = Vec::new(env);
    ms.push_back(Milestone {
        title:        String::from_str(env, "Design"),
        trigger_keyword:   String::from_str(env, "feat/design"),
        bps:          3000,
        status:       MilestoneStatus::Pending,
        pr_url:       None,
        completed_at: None,
    });
    ms.push_back(Milestone {
        title:        String::from_str(env, "Backend"),
        trigger_keyword:   String::from_str(env, "feat/backend"),
        bps:          4000,
        status:       MilestoneStatus::Pending,
        pr_url:       None,
        completed_at: None,
    });
    ms.push_back(Milestone {
        title:        String::from_str(env, "Delivery"),
        trigger_keyword:   String::from_str(env, "feat/delivery"),
        bps:          3000,
        status:       MilestoneStatus::Pending,
        pr_url:       None,
        completed_at: None,
    });
    ms
}

fn pr_url(env: &Env) -> String {
    String::from_str(env, "https://github.com/acme/api/pull/42")
}

fn advance_time(env: &Env, secs: u64) {
    env.ledger().set(LedgerInfo {
        timestamp:                env.ledger().timestamp() + secs,
        protocol_version:         22,
        sequence_number:          env.ledger().sequence(),
        network_id:               Default::default(),
        base_reserve:             10,
        min_temp_entry_ttl:       10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl:            3_110_400,
    });
}

fn token_balance(env: &Env, token: &Address, addr: &Address) -> i128 {
    token::Client::new(env, token).balance(addr)
}

fn do_initialize(t: &TestEnv, dispute_window: u64) {
    let c  = client(t);
    let ms = make_milestones(&t.env);
    c.initialize(
        &t.client, &t.developer, &t.backend,
        &t.token, &1_000_0000000i128, &ms, &dispute_window,
    );
}

// ─── Initialize ───────────────────────────────────────────────────────────────

#[test]
fn test_initialize_success() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);

    assert_eq!(c.get_status(), EscrowStatus::Active);
    let (total, released, remaining) = c.get_balance();
    assert_eq!(total,     1_000_0000000);
    assert_eq!(released,  0);
    assert_eq!(remaining, 1_000_0000000);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_initialize_wrong_bps() {
    let t  = setup();
    let c  = client(&t);
    let mut ms = make_milestones(&t.env);
    let mut m = ms.get(0).unwrap();
    m.bps = 9999;
    ms.set(0, m);
    c.initialize(&t.client, &t.developer, &t.backend,
                 &t.token, &1_000_0000000i128, &ms, &259200u64);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_initialize_twice_fails() {
    let t = setup();
    do_initialize(&t, 259200);
    do_initialize(&t, 259200);
}

// ─── Happy path ───────────────────────────────────────────────────────────────

#[test]
fn test_full_happy_path_client_approves() {
    let t  = setup();
    do_initialize(&t, 259200);
    let c  = client(&t);

    let dev_before = token_balance(&t.env, &t.token, &t.developer);

    for i in 0u32..3 {
        c.mark_complete(&i, &pr_url(&t.env));
        assert_eq!(c.get_milestone(&i).status, MilestoneStatus::PendingRelease);
        c.approve(&i);
        assert_eq!(c.get_milestone(&i).status, MilestoneStatus::Released);
    }

    assert_eq!(c.get_status(), EscrowStatus::Completed);

    let dev_after = token_balance(&t.env, &t.token, &t.developer);
    let expected  = 1_000_0000000i128 * 99 / 100;
    assert_eq!(dev_after - dev_before, expected);
}

// ─── Auto-release ─────────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #41)")]
fn test_auto_release_too_early_fails() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);
    c.mark_complete(&0, &pr_url(&t.env));
    c.auto_release(&0);
}

#[test]
fn test_auto_release_after_window() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);
    c.mark_complete(&0, &pr_url(&t.env));
    advance_time(&t.env, 259201);

    let dev_before = token_balance(&t.env, &t.token, &t.developer);
    c.auto_release(&0);
    let dev_after  = token_balance(&t.env, &t.token, &t.developer);

    let expected = 300_0000000i128 * 99 / 100;
    assert_eq!(dev_after - dev_before, expected);
}

#[test]
#[should_panic(expected = "Error(Contract, #50)")]
fn test_auto_release_no_window_fails() {
    let t = setup();
    do_initialize(&t, 0);
    let c = client(&t);
    c.mark_complete(&0, &pr_url(&t.env));
    advance_time(&t.env, 999999);
    c.auto_release(&0);
}

// ─── Dispute ──────────────────────────────────────────────────────────────────

#[test]
fn test_dispute_within_window() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);
    c.mark_complete(&0, &pr_url(&t.env));

    c.dispute(&0, &String::from_str(&t.env, "Does not match spec"));
    assert_eq!(c.get_milestone(&0).status, MilestoneStatus::Disputed);
}

#[test]
#[should_panic(expected = "Error(Contract, #42)")]
fn test_dispute_after_window_fails() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);
    c.mark_complete(&0, &pr_url(&t.env));
    advance_time(&t.env, 259201);
    c.dispute(&0, &String::from_str(&t.env, "Too late"));
}

#[test]
fn test_dispute_resolved_dev_wins() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);
    c.mark_complete(&0, &pr_url(&t.env));
    c.dispute(&0, &String::from_str(&t.env, "Dispute"));

    let dev_before = token_balance(&t.env, &t.token, &t.developer);
    c.resolve_dispute(&0, &true);
    let dev_after  = token_balance(&t.env, &t.token, &t.developer);
    assert!(dev_after > dev_before);
}

#[test]
fn test_dispute_resolved_client_wins() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);
    c.mark_complete(&0, &pr_url(&t.env));
    c.dispute(&0, &String::from_str(&t.env, "Not delivered"));

    let client_before = token_balance(&t.env, &t.token, &t.client);
    c.resolve_dispute(&0, &false);
    let client_after  = token_balance(&t.env, &t.token, &t.client);
    assert_eq!(client_after - client_before, 300_0000000i128);
}

// ─── Cancel ───────────────────────────────────────────────────────────────────

#[test]
fn test_cancel_full_refund() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);

    let before = token_balance(&t.env, &t.token, &t.client);
    c.cancel();
    let after  = token_balance(&t.env, &t.token, &t.client);

    assert_eq!(after - before, 1_000_0000000i128);
    assert_eq!(c.get_status(), EscrowStatus::Cancelled);
}

#[test]
fn test_cancel_partial_after_one_release() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);

    c.mark_complete(&0, &pr_url(&t.env));
    c.approve(&0);

    let before = token_balance(&t.env, &t.token, &t.client);
    c.cancel();
    let after  = token_balance(&t.env, &t.token, &t.client);
    assert_eq!(after - before, 700_0000000i128);
}

#[test]
#[should_panic(expected = "Error(Contract, #51)")]
fn test_cancel_blocked_pending() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);
    c.mark_complete(&0, &pr_url(&t.env));
    c.cancel();
}

// ─── Time remaining ───────────────────────────────────────────────────────────

#[test]
fn test_time_until_auto_release() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);
    c.mark_complete(&0, &pr_url(&t.env));

    let t1 = c.time_until_auto_release(&0);
    assert!(t1 > 0 && t1 <= 259200);

    advance_time(&t.env, 100);
    let t2 = c.time_until_auto_release(&0);
    assert!(t2 < t1);

    advance_time(&t.env, 259200);
    let t3 = c.time_until_auto_release(&0);
    assert_eq!(t3, 0);
}

// ─── Edge cases ───────────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #31)")]
fn test_mark_complete_twice_fails() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);
    c.mark_complete(&0, &pr_url(&t.env));
    c.mark_complete(&0, &pr_url(&t.env));
}

#[test]
#[should_panic(expected = "Error(Contract, #21)")]
fn test_ops_fail_after_cancel() {
    let t = setup();
    do_initialize(&t, 259200);
    let c = client(&t);
    c.cancel();
    c.mark_complete(&0, &pr_url(&t.env));
}

#[test]
#[should_panic(expected = "Error(Contract, #20)")]
fn test_ops_fail_after_completion() {
    let t = setup();
    do_initialize(&t, 0);
    let c = client(&t);

    for i in 0u32..3 {
        c.mark_complete(&i, &pr_url(&t.env));
        c.approve(&i);
    }
    assert_eq!(c.get_status(), EscrowStatus::Completed);
    c.mark_complete(&0, &pr_url(&t.env));
}