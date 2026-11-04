use soroban_sdk::{contracterror, contracttype, Address, Vec};

/// Version of the contract code. Bump it when behaviour or storage changes.
pub const CONTRACT_VERSION: u32 = 1;

/// Maximum number of owners a treasury can have.
pub const MAX_OWNERS: u32 = 10;
/// Maximum number of assets that can have a spending-limit policy.
pub const MAX_POLICIES: u32 = 10;
/// Longest a proposal can stay open, in seconds (30 days).
pub const MAX_PROPOSAL_TTL_SECS: u64 = 30 * 24 * 60 * 60;

/// Ledgers per day at ~5s per ledger.
const DAY_IN_LEDGERS: u32 = 17_280;
/// Extend an entry when its remaining TTL drops below this many ledgers (30 days).
pub const BUMP_THRESHOLD: u32 = 30 * DAY_IN_LEDGERS;
/// Extend an entry to this many ledgers (60 days).
pub const BUMP_EXTEND_TO: u32 = 60 * DAY_IN_LEDGERS;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Owners and approval threshold.
    Config,
    /// List of assets that have a spending-limit policy.
    PolicyAssets,
    /// `SpendLimit` for an asset.
    Policy(Address),
    /// `PeriodSpend` for an asset.
    Spent(Address),
    /// Number of proposals created so far; also the next proposal id.
    ProposalCount,
    /// A `Proposal` by id.
    Proposal(u64),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub owners: Vec<Address>,
    pub threshold: u32,
}

/// Spending limit for one asset: at most `limit` can leave the treasury per `period` seconds.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpendLimit {
    pub limit: i128,
    pub period: u64,
}

/// A spending limit to set when the treasury is created.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LimitInit {
    pub asset: Address,
    pub limit: i128,
    pub period: u64,
}

/// How much of an asset has been spent in the current period.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeriodSpend {
    pub start: u64,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub asset: Address,
    pub to: Address,
    pub amount: i128,
    pub approvals: Vec<Address>,
    pub executed: bool,
    /// Ledger timestamp (seconds) after which the proposal can no longer be approved or executed.
    pub expires_at: u64,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Reserved. Setup now happens in the constructor, which can only run once.
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidThreshold = 3,
    InvalidOwners = 4,
    TooManyOwners = 5,
    NotOwner = 6,
    NotEnoughApprovals = 7,
    InvalidAmount = 8,
    NoPolicy = 9,
    OverLimit = 10,
    ProposalNotFound = 11,
    AlreadyApproved = 12,
    AlreadyExecuted = 13,
    Expired = 14,
    InvalidTtl = 15,
    TooManyPolicies = 16,
    InvalidPolicy = 17,
    DuplicateApprover = 18,
    Overflow = 19,
}
