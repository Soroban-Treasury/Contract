use soroban_sdk::{contractevent, Address, Vec};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    pub owners: Vec<Address>,
    pub threshold: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LimitSet {
    #[topic]
    pub asset: Address,
    pub limit: i128,
    pub period: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LimitRemoved {
    #[topic]
    pub asset: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposed {
    #[topic]
    pub id: u64,
    pub proposer: Address,
    pub asset: Address,
    pub to: Address,
    pub amount: i128,
    pub expires_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Approved {
    #[topic]
    pub id: u64,
    pub owner: Address,
    pub approvals: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Executed {
    #[topic]
    pub id: u64,
    pub executor: Address,
    pub asset: Address,
    pub to: Address,
    pub amount: i128,
}
