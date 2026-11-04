//! Policy-controlled team treasury.
//!
//! Owners hold a shared treasury. Spending follows `propose -> approve -> execute`:
//! an owner proposes a payment, `threshold` owners approve it, and an owner executes
//! it as long as it fits the per-asset spending limit for the current period.
//!
//! The contract is unaudited. Testnet only.
#![no_std]

mod events;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token, Address, Env, IntoVal, Val, Vec};

pub use types::*;

use events::{Approved, Executed, Initialized, LimitRemoved, LimitSet, Proposed};

#[contract]
pub struct TreasuryContract;

#[contractimpl]
impl TreasuryContract {
    /// Runs once, at deployment, in the same transaction that creates the contract. That leaves no
    /// moment where someone else could initialize an empty treasury first.
    ///
    /// Sets the owners, the approval threshold and the starting spending limits. Limits are set
    /// here because changing them later needs `threshold` owners to authorize in one call (see
    /// `set_spend_limit`). Assets without a limit cannot be spent.
    pub fn __constructor(
        env: Env,
        owners: Vec<Address>,
        threshold: u32,
        limits: Vec<LimitInit>,
    ) -> Result<(), Error> {
        if owners.is_empty() {
            return Err(Error::InvalidOwners);
        }
        if owners.len() > MAX_OWNERS {
            return Err(Error::TooManyOwners);
        }
        if threshold == 0 || threshold > owners.len() {
            return Err(Error::InvalidThreshold);
        }
        if has_duplicates(&owners) {
            return Err(Error::InvalidOwners);
        }

        let config = Config {
            owners: owners.clone(),
            threshold,
        };
        put(&env, &DataKey::Config, &config);
        put(&env, &DataKey::ProposalCount, &0u64);

        let mut assets = Vec::<Address>::new(&env);
        for init in limits.iter() {
            if assets.contains(&init.asset) {
                return Err(Error::InvalidPolicy);
            }
            apply_limit(&env, &mut assets, init.asset, init.limit, init.period)?;
        }
        put(&env, &DataKey::PolicyAssets, &assets);

        Initialized { owners, threshold }.publish(&env);
        Ok(())
    }

    /// Set or replace the spending limit for `asset`.
    ///
    /// Needs at least `threshold` distinct owners in `approvers`, each of whom must authorize.
    pub fn set_spend_limit(
        env: Env,
        approvers: Vec<Address>,
        asset: Address,
        limit: i128,
        period: u64,
    ) -> Result<(), Error> {
        let config = load_config(&env)?;
        require_owner_approvals(&config, &approvers)?;

        let mut assets = load_policy_assets(&env);
        apply_limit(&env, &mut assets, asset, limit, period)?;
        put(&env, &DataKey::PolicyAssets, &assets);
        Ok(())
    }

    /// Remove the spending limit for `asset`. With no limit, the asset can't be spent.
    pub fn remove_spend_limit(
        env: Env,
        approvers: Vec<Address>,
        asset: Address,
    ) -> Result<(), Error> {
        let config = load_config(&env)?;
        require_owner_approvals(&config, &approvers)?;

        let policy_key = DataKey::Policy(asset.clone());
        if !env.storage().persistent().has(&policy_key) {
            return Err(Error::NoPolicy);
        }
        env.storage().persistent().remove(&policy_key);
        env.storage()
            .persistent()
            .remove(&DataKey::Spent(asset.clone()));

        let mut assets = load_policy_assets(&env);
        if let Some(index) = assets.first_index_of(&asset) {
            assets.remove(index);
        }
        put(&env, &DataKey::PolicyAssets, &assets);

        LimitRemoved { asset }.publish(&env);
        Ok(())
    }

    /// Propose paying `amount` of `asset` to `to`. The proposer's approval is counted.
    /// The proposal stays open for `ttl_secs` seconds. Returns the proposal id.
    pub fn propose(
        env: Env,
        proposer: Address,
        asset: Address,
        to: Address,
        amount: i128,
        ttl_secs: u64,
    ) -> Result<u64, Error> {
        proposer.require_auth();
        let config = load_config(&env)?;
        require_owner(&config, &proposer)?;

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if ttl_secs == 0 || ttl_secs > MAX_PROPOSAL_TTL_SECS {
            return Err(Error::InvalidTtl);
        }
        let policy = load_policy(&env, &asset)?;
        if amount > policy.limit {
            // Could never be executed, so don't let it be created.
            return Err(Error::OverLimit);
        }

        let expires_at = env
            .ledger()
            .timestamp()
            .checked_add(ttl_secs)
            .ok_or(Error::Overflow)?;

        let id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0);
        let next = id.checked_add(1).ok_or(Error::Overflow)?;
        put(&env, &DataKey::ProposalCount, &next);

        let mut approvals = Vec::new(&env);
        approvals.push_back(proposer.clone());
        let proposal = Proposal {
            id,
            proposer: proposer.clone(),
            asset: asset.clone(),
            to: to.clone(),
            amount,
            approvals,
            executed: false,
            expires_at,
        };
        save_proposal(&env, &proposal);

        Proposed {
            id,
            proposer,
            asset,
            to,
            amount,
            expires_at,
        }
        .publish(&env);
        Ok(id)
    }

    /// Approve a proposal as an owner.
    pub fn approve(env: Env, owner: Address, id: u64) -> Result<(), Error> {
        owner.require_auth();
        let config = load_config(&env)?;
        require_owner(&config, &owner)?;

        let mut proposal = load_proposal(&env, id)?;
        if proposal.executed {
            return Err(Error::AlreadyExecuted);
        }
        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::Expired);
        }
        if proposal.approvals.contains(&owner) {
            return Err(Error::AlreadyApproved);
        }

        proposal.approvals.push_back(owner.clone());
        let approvals = proposal.approvals.len();
        save_proposal(&env, &proposal);

        Approved {
            id,
            owner,
            approvals,
        }
        .publish(&env);
        Ok(())
    }

    /// Execute an approved proposal: pay out of the treasury, counting against the spending limit.
    pub fn execute(env: Env, executor: Address, id: u64) -> Result<(), Error> {
        executor.require_auth();
        let config = load_config(&env)?;
        require_owner(&config, &executor)?;

        let mut proposal = load_proposal(&env, id)?;
        if proposal.executed {
            return Err(Error::AlreadyExecuted);
        }
        let now = env.ledger().timestamp();
        if now > proposal.expires_at {
            return Err(Error::Expired);
        }
        if proposal.approvals.len() < config.threshold {
            return Err(Error::NotEnoughApprovals);
        }

        // The policy may have changed since the proposal was made, so check it again.
        let policy = load_policy(&env, &proposal.asset)?;
        let spent_key = DataKey::Spent(proposal.asset.clone());
        let mut spend: PeriodSpend =
            env.storage()
                .persistent()
                .get(&spent_key)
                .unwrap_or(PeriodSpend {
                    start: now,
                    amount: 0,
                });
        let period_end = spend
            .start
            .checked_add(policy.period)
            .ok_or(Error::Overflow)?;
        if now >= period_end {
            spend = PeriodSpend {
                start: now,
                amount: 0,
            };
        }
        let new_amount = spend
            .amount
            .checked_add(proposal.amount)
            .ok_or(Error::Overflow)?;
        if new_amount > policy.limit {
            return Err(Error::OverLimit);
        }
        spend.amount = new_amount;

        // Update state before the external call.
        put(&env, &spent_key, &spend);
        proposal.executed = true;
        save_proposal(&env, &proposal);

        token::Client::new(&env, &proposal.asset).transfer(
            &env.current_contract_address(),
            &proposal.to,
            &proposal.amount,
        );

        Executed {
            id,
            executor,
            asset: proposal.asset,
            to: proposal.to,
            amount: proposal.amount,
        }
        .publish(&env);
        Ok(())
    }

    /// Version of the contract code, for clients and deployment records.
    pub fn contract_version(_env: Env) -> u32 {
        CONTRACT_VERSION
    }

    // TODO(wave: contracts-1): recipient allowlist policy
    // TODO(wave: contracts-2): timelock policy
    // TODO(wave: contracts-3): batch payment proposal

    pub fn get_config(env: Env) -> Result<Config, Error> {
        load_config(&env)
    }

    pub fn get_proposal(env: Env, id: u64) -> Result<Proposal, Error> {
        load_proposal(&env, id)
    }

    pub fn proposal_count(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0)
    }

    pub fn get_spend_limit(env: Env, asset: Address) -> Result<SpendLimit, Error> {
        load_policy(&env, &asset)
    }

    pub fn get_policy_assets(env: Env) -> Vec<Address> {
        load_policy_assets(&env)
    }

    /// Amount of `asset` spent in the current period (0 if the period has rolled over).
    pub fn get_spent(env: Env, asset: Address) -> Result<i128, Error> {
        let policy = load_policy(&env, &asset)?;
        let spend: Option<PeriodSpend> = env.storage().persistent().get(&DataKey::Spent(asset));
        Ok(match spend {
            Some(s) => {
                let end = s.start.saturating_add(policy.period);
                if env.ledger().timestamp() >= end {
                    0
                } else {
                    s.amount
                }
            }
            None => 0,
        })
    }
}

/// Validate and store a limit, adding the asset to `assets` if it is new. Emits `LimitSet`.
/// The caller saves `assets`.
fn apply_limit(
    env: &Env,
    assets: &mut Vec<Address>,
    asset: Address,
    limit: i128,
    period: u64,
) -> Result<(), Error> {
    if limit <= 0 || period == 0 {
        return Err(Error::InvalidPolicy);
    }
    if !assets.contains(&asset) {
        if assets.len() >= MAX_POLICIES {
            return Err(Error::TooManyPolicies);
        }
        assets.push_back(asset.clone());
    }
    put(
        env,
        &DataKey::Policy(asset.clone()),
        &SpendLimit { limit, period },
    );
    LimitSet {
        asset,
        limit,
        period,
    }
    .publish(env);
    Ok(())
}

fn has_duplicates(owners: &Vec<Address>) -> bool {
    for i in 0..owners.len() {
        for j in (i + 1)..owners.len() {
            if owners.get(i) == owners.get(j) {
                return true;
            }
        }
    }
    false
}

fn load_config(env: &Env) -> Result<Config, Error> {
    let key = DataKey::Config;
    let config: Config = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::NotInitialized)?;
    extend(env, &key);
    Ok(config)
}

fn load_policy(env: &Env, asset: &Address) -> Result<SpendLimit, Error> {
    let key = DataKey::Policy(asset.clone());
    let policy: SpendLimit = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::NoPolicy)?;
    extend(env, &key);
    Ok(policy)
}

fn load_policy_assets(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::PolicyAssets)
        .unwrap_or(Vec::new(env))
}

fn load_proposal(env: &Env, id: u64) -> Result<Proposal, Error> {
    let key = DataKey::Proposal(id);
    let proposal: Proposal = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::ProposalNotFound)?;
    extend(env, &key);
    Ok(proposal)
}

fn save_proposal(env: &Env, proposal: &Proposal) {
    put(env, &DataKey::Proposal(proposal.id), proposal);
}

fn require_owner(config: &Config, who: &Address) -> Result<(), Error> {
    if config.owners.contains(who) {
        Ok(())
    } else {
        Err(Error::NotOwner)
    }
}

/// Check `approvers` are distinct owners, at least `threshold` of them, and have all authorized.
fn require_owner_approvals(config: &Config, approvers: &Vec<Address>) -> Result<(), Error> {
    if approvers.len() < config.threshold {
        return Err(Error::NotEnoughApprovals);
    }
    if has_duplicates(approvers) {
        return Err(Error::DuplicateApprover);
    }
    for approver in approvers.iter() {
        require_owner(config, &approver)?;
        approver.require_auth();
    }
    Ok(())
}

/// Write a persistent entry and extend its TTL, and keep the contract instance alive.
fn put<V: IntoVal<Env, Val>>(env: &Env, key: &DataKey, value: &V) {
    env.storage().persistent().set(key, value);
    extend(env, key);
}

fn extend(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, BUMP_THRESHOLD, BUMP_EXTEND_TO);
    env.storage()
        .instance()
        .extend_ttl(BUMP_THRESHOLD, BUMP_EXTEND_TO);
}
