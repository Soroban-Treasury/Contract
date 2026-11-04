#![cfg(test)]

extern crate std;

use crate::{Error, LimitInit, TreasuryContract, TreasuryContractClient, MAX_OWNERS, MAX_POLICIES};
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    vec, Address, Env, Vec,
};

const START: u64 = 1_000;
const DAY: u64 = 86_400;
const LIMIT: i128 = 1_000;

struct Setup<'a> {
    client: TreasuryContractClient<'a>,
    token: TokenClient<'a>,
    asset: Address,
    treasury: Address,
    owners: [Address; 3],
    recipient: Address,
}

fn new_asset(env: &Env) -> Address {
    let admin = Address::generate(env);
    env.register_stellar_asset_contract_v2(admin).address()
}

/// Deploy a treasury. The constructor runs as part of registering the contract.
fn deploy(env: &Env, owners: &[Address], threshold: u32, limits: Vec<LimitInit>) -> Address {
    env.register(
        TreasuryContract,
        (Vec::from_slice(env, owners), threshold, limits),
    )
}

fn limit_init(asset: &Address, limit: i128, period: u64) -> LimitInit {
    LimitInit {
        asset: asset.clone(),
        limit,
        period,
    }
}

/// Treasury with 3 owners, threshold 2, a funded asset and a 1000/day limit on it.
fn setup(env: &Env) -> Setup<'_> {
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = START);

    let owners = [
        Address::generate(env),
        Address::generate(env),
        Address::generate(env),
    ];
    let treasury = deploy(env, &owners, 2, Vec::new(env));
    let client = TreasuryContractClient::new(env, &treasury);

    let asset = new_asset(env);
    StellarAssetClient::new(env, &asset).mint(&treasury, &1_000_000);
    client.set_spend_limit(
        &vec![env, owners[0].clone(), owners[1].clone()],
        &asset,
        &LIMIT,
        &DAY,
    );

    Setup {
        client,
        token: TokenClient::new(env, &asset),
        asset,
        treasury,
        owners,
        recipient: Address::generate(env),
    }
}

impl Setup<'_> {
    fn propose(&self, amount: i128) -> u64 {
        self.client
            .propose(&self.owners[0], &self.asset, &self.recipient, &amount, &DAY)
    }

    /// Propose, approve with a second owner, execute.
    fn pay(&self, amount: i128) -> u64 {
        let id = self.propose(amount);
        self.client.approve(&self.owners[1], &id);
        self.client.execute(&self.owners[0], &id);
        id
    }
}

#[test]
fn initialize_stores_config() {
    let env = Env::default();
    let s = setup(&env);
    let config = s.client.get_config();
    assert_eq!(config.threshold, 2);
    assert_eq!(config.owners.len(), 3);
    assert_eq!(s.client.proposal_count(), 0);
}

#[test]
fn happy_path_proposal_to_payment() {
    let env = Env::default();
    let s = setup(&env);

    let id = s.propose(400);
    let proposal = s.client.get_proposal(&id);
    assert_eq!(proposal.approvals.len(), 1);
    assert!(!proposal.executed);

    s.client.approve(&s.owners[1], &id);
    s.client.execute(&s.owners[2], &id);

    assert_eq!(s.token.balance(&s.recipient), 400);
    assert_eq!(s.token.balance(&s.treasury), 1_000_000 - 400);
    assert!(s.client.get_proposal(&id).executed);
    assert_eq!(s.client.get_spent(&s.asset), 400);
}

#[test]
fn proposal_ids_increase() {
    let env = Env::default();
    let s = setup(&env);
    assert_eq!(s.propose(1), 0);
    assert_eq!(s.propose(1), 1);
    assert_eq!(s.client.proposal_count(), 2);
}

#[test]
fn execute_below_threshold_fails() {
    let env = Env::default();
    let s = setup(&env);
    let id = s.propose(100);
    let res = s.client.try_execute(&s.owners[0], &id);
    assert_eq!(res, Err(Ok(Error::NotEnoughApprovals)));
    assert_eq!(s.token.balance(&s.recipient), 0);
}

#[test]
fn single_proposal_over_limit_is_rejected() {
    let env = Env::default();
    let s = setup(&env);
    let res = s
        .client
        .try_propose(&s.owners[0], &s.asset, &s.recipient, &(LIMIT + 1), &DAY);
    assert_eq!(res, Err(Ok(Error::OverLimit)));
}

#[test]
fn cumulative_spend_over_limit_fails_until_period_rolls_over() {
    let env = Env::default();
    let s = setup(&env);

    s.pay(600);

    let id = s.propose(600);
    s.client.approve(&s.owners[1], &id);
    let res = s.client.try_execute(&s.owners[0], &id);
    assert_eq!(res, Err(Ok(Error::OverLimit)));
    assert_eq!(s.token.balance(&s.recipient), 600);

    // A smaller payment that still fits goes through.
    s.pay(400);
    assert_eq!(s.client.get_spent(&s.asset), 1_000);

    // After the period ends the same proposal can run (if it hasn't expired).
    env.ledger().with_mut(|l| l.timestamp = START + DAY);
    assert_eq!(s.client.get_spent(&s.asset), 0);
    let id = s.propose(600);
    s.client.approve(&s.owners[1], &id);
    s.client.execute(&s.owners[0], &id);
    assert_eq!(s.token.balance(&s.recipient), 1_600);
}

#[test]
fn duplicate_approval_fails() {
    let env = Env::default();
    let s = setup(&env);
    let id = s.propose(100);

    // The proposer already approved by proposing.
    let res = s.client.try_approve(&s.owners[0], &id);
    assert_eq!(res, Err(Ok(Error::AlreadyApproved)));

    s.client.approve(&s.owners[1], &id);
    let res = s.client.try_approve(&s.owners[1], &id);
    assert_eq!(res, Err(Ok(Error::AlreadyApproved)));
}

#[test]
fn non_owner_cannot_propose_approve_or_execute() {
    let env = Env::default();
    let s = setup(&env);
    let stranger = Address::generate(&env);
    let id = s.propose(100);

    let res = s
        .client
        .try_propose(&stranger, &s.asset, &s.recipient, &100, &DAY);
    assert_eq!(res, Err(Ok(Error::NotOwner)));
    assert_eq!(
        s.client.try_approve(&stranger, &id),
        Err(Ok(Error::NotOwner))
    );
    s.client.approve(&s.owners[1], &id);
    assert_eq!(
        s.client.try_execute(&stranger, &id),
        Err(Ok(Error::NotOwner))
    );
}

#[test]
fn expired_proposal_cannot_be_approved_or_executed() {
    let env = Env::default();
    let s = setup(&env);
    let id = s.propose(100);
    s.client.approve(&s.owners[1], &id);
    let other = s.propose(100);

    env.ledger().with_mut(|l| l.timestamp = START + DAY + 1);

    assert_eq!(
        s.client.try_execute(&s.owners[0], &id),
        Err(Ok(Error::Expired))
    );
    assert_eq!(
        s.client.try_approve(&s.owners[2], &other),
        Err(Ok(Error::Expired))
    );
    assert_eq!(s.token.balance(&s.recipient), 0);
}

#[test]
fn executed_proposal_cannot_run_or_be_approved_again() {
    let env = Env::default();
    let s = setup(&env);
    let id = s.pay(100);
    assert_eq!(
        s.client.try_execute(&s.owners[0], &id),
        Err(Ok(Error::AlreadyExecuted))
    );
    assert_eq!(
        s.client.try_approve(&s.owners[2], &id),
        Err(Ok(Error::AlreadyExecuted))
    );
    assert_eq!(s.token.balance(&s.recipient), 100);
}

#[test]
fn asset_without_policy_cannot_be_spent() {
    let env = Env::default();
    let s = setup(&env);
    let other_asset = new_asset(&env);
    let res = s
        .client
        .try_propose(&s.owners[0], &other_asset, &s.recipient, &10, &DAY);
    assert_eq!(res, Err(Ok(Error::NoPolicy)));
}

#[test]
fn removing_policy_blocks_pending_proposals() {
    let env = Env::default();
    let s = setup(&env);
    let id = s.propose(100);
    s.client.approve(&s.owners[1], &id);

    s.client.remove_spend_limit(
        &vec![&env, s.owners[0].clone(), s.owners[2].clone()],
        &s.asset,
    );
    assert_eq!(s.client.get_policy_assets().len(), 0);

    assert_eq!(
        s.client.try_execute(&s.owners[0], &id),
        Err(Ok(Error::NoPolicy))
    );
}

#[test]
fn lowering_limit_applies_to_pending_proposals() {
    let env = Env::default();
    let s = setup(&env);
    let id = s.propose(800);
    s.client.approve(&s.owners[1], &id);

    s.client.set_spend_limit(
        &vec![&env, s.owners[0].clone(), s.owners[1].clone()],
        &s.asset,
        &500,
        &DAY,
    );
    assert_eq!(
        s.client.try_execute(&s.owners[0], &id),
        Err(Ok(Error::OverLimit))
    );
}

#[test]
fn invalid_amount_and_ttl_are_rejected() {
    let env = Env::default();
    let s = setup(&env);
    let try_propose = |amount: i128, ttl: u64| {
        s.client
            .try_propose(&s.owners[0], &s.asset, &s.recipient, &amount, &ttl)
    };
    assert_eq!(try_propose(0, DAY), Err(Ok(Error::InvalidAmount)));
    assert_eq!(try_propose(-5, DAY), Err(Ok(Error::InvalidAmount)));
    assert_eq!(try_propose(10, 0), Err(Ok(Error::InvalidTtl)));
    assert_eq!(try_propose(10, 31 * DAY), Err(Ok(Error::InvalidTtl)));
}

#[test]
fn policy_changes_need_threshold_distinct_owner_approvals() {
    let env = Env::default();
    let s = setup(&env);
    let stranger = Address::generate(&env);
    let set = |approvers: Vec<Address>| {
        s.client
            .try_set_spend_limit(&approvers, &s.asset, &500, &DAY)
    };

    assert_eq!(
        set(vec![&env, s.owners[0].clone()]),
        Err(Ok(Error::NotEnoughApprovals))
    );
    assert_eq!(
        set(vec![&env, s.owners[0].clone(), s.owners[0].clone()]),
        Err(Ok(Error::DuplicateApprover))
    );
    assert_eq!(
        set(vec![&env, s.owners[0].clone(), stranger]),
        Err(Ok(Error::NotOwner))
    );
    // Nothing changed.
    assert_eq!(s.client.get_spend_limit(&s.asset).limit, LIMIT);
}

#[test]
fn invalid_policy_values_are_rejected() {
    let env = Env::default();
    let s = setup(&env);
    let approvers = vec![&env, s.owners[0].clone(), s.owners[1].clone()];
    assert_eq!(
        s.client.try_set_spend_limit(&approvers, &s.asset, &0, &DAY),
        Err(Ok(Error::InvalidPolicy))
    );
    assert_eq!(
        s.client.try_set_spend_limit(&approvers, &s.asset, &10, &0),
        Err(Ok(Error::InvalidPolicy))
    );
}

#[test]
fn policy_count_is_bounded() {
    let env = Env::default();
    let s = setup(&env);
    let approvers = vec![&env, s.owners[0].clone(), s.owners[1].clone()];
    // One policy already exists from setup.
    for _ in 1..MAX_POLICIES {
        s.client
            .set_spend_limit(&approvers, &new_asset(&env), &10, &DAY);
    }
    assert_eq!(s.client.get_policy_assets().len(), MAX_POLICIES);
    assert_eq!(
        s.client
            .try_set_spend_limit(&approvers, &new_asset(&env), &10, &DAY),
        Err(Ok(Error::TooManyPolicies))
    );
    // Updating an existing policy is still allowed.
    s.client.set_spend_limit(&approvers, &s.asset, &20, &DAY);
}

#[test]
fn missing_proposal_is_reported() {
    let env = Env::default();
    let s = setup(&env);
    assert_eq!(
        s.client.try_approve(&s.owners[0], &99),
        Err(Ok(Error::ProposalNotFound))
    );
}

#[test]
fn state_changes_emit_events() {
    let env = Env::default();
    let s = setup(&env);
    // `all()` returns the events of the most recent invocation.
    s.propose(100);
    assert_eq!(env.events().all().events().len(), 1);
    s.client.approve(&s.owners[1], &0);
    assert_eq!(env.events().all().events().len(), 1);
    s.client.execute(&s.owners[0], &0);
    // `execute` emits the token transfer event and the treasury's own event.
    assert!(env.events().all().events().len() >= 2);
}

#[test]
fn constructor_stores_owners_and_starting_limits() {
    let env = Env::default();
    env.mock_all_auths();
    let owners = [Address::generate(&env), Address::generate(&env)];
    let asset = new_asset(&env);
    let treasury = deploy(&env, &owners, 2, vec![&env, limit_init(&asset, 500, DAY)]);
    let client = TreasuryContractClient::new(&env, &treasury);

    let config = client.get_config();
    assert_eq!(config.threshold, 2);
    assert_eq!(config.owners.len(), 2);
    assert_eq!(client.get_spend_limit(&asset).limit, 500);
    assert_eq!(client.get_policy_assets().len(), 1);
    assert_eq!(client.contract_version(), 1);
}

#[test]
fn threshold_two_treasury_can_spend_without_changing_policy() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = START);
    let owners = [Address::generate(&env), Address::generate(&env)];
    let asset = new_asset(&env);
    let treasury = deploy(&env, &owners, 2, vec![&env, limit_init(&asset, 500, DAY)]);
    let client = TreasuryContractClient::new(&env, &treasury);
    StellarAssetClient::new(&env, &asset).mint(&treasury, &10_000);

    let to = Address::generate(&env);
    let id = client.propose(&owners[0], &asset, &to, &300, &DAY);
    client.approve(&owners[1], &id);
    client.execute(&owners[0], &id);
    assert_eq!(TokenClient::new(&env, &asset).balance(&to), 300);
}

// A constructor that fails aborts the deployment, so a bad treasury can never exist.

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn constructor_rejects_no_owners() {
    let env = Env::default();
    deploy(&env, &[], 1, Vec::new(&env));
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn constructor_rejects_zero_threshold() {
    let env = Env::default();
    deploy(&env, &[Address::generate(&env)], 0, Vec::new(&env));
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn constructor_rejects_threshold_above_owner_count() {
    let env = Env::default();
    let owners = [Address::generate(&env), Address::generate(&env)];
    deploy(&env, &owners, 3, Vec::new(&env));
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn constructor_rejects_duplicate_owners() {
    let env = Env::default();
    let a = Address::generate(&env);
    deploy(&env, &[a.clone(), a], 1, Vec::new(&env));
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn constructor_rejects_too_many_owners() {
    let env = Env::default();
    let owners: std::vec::Vec<Address> =
        (0..=MAX_OWNERS).map(|_| Address::generate(&env)).collect();
    deploy(&env, &owners, 1, Vec::new(&env));
}

#[test]
#[should_panic(expected = "Error(Contract, #17)")]
fn constructor_rejects_zero_limit() {
    let env = Env::default();
    let asset = new_asset(&env);
    deploy(
        &env,
        &[Address::generate(&env)],
        1,
        vec![&env, limit_init(&asset, 0, DAY)],
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #17)")]
fn constructor_rejects_zero_period() {
    let env = Env::default();
    let asset = new_asset(&env);
    deploy(
        &env,
        &[Address::generate(&env)],
        1,
        vec![&env, limit_init(&asset, 10, 0)],
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #17)")]
fn constructor_rejects_the_same_asset_twice() {
    let env = Env::default();
    let asset = new_asset(&env);
    deploy(
        &env,
        &[Address::generate(&env)],
        1,
        vec![
            &env,
            limit_init(&asset, 10, DAY),
            limit_init(&asset, 20, DAY),
        ],
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn constructor_rejects_too_many_limits() {
    let env = Env::default();
    let mut limits = Vec::new(&env);
    for _ in 0..=MAX_POLICIES {
        limits.push_back(limit_init(&new_asset(&env), 10, DAY));
    }
    deploy(&env, &[Address::generate(&env)], 1, limits);
}
