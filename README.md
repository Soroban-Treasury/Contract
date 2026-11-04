# Treasury contract

Policy-controlled team treasury on Soroban (Rust, `#![no_std]`). Part of Stellar Treasury.

> **Unaudited. Testnet only.** This contract holds funds. Do not use it on mainnet.

## How it differs from other wallets

- **Stellar-Agent-Guard** limits what an AI agent can spend. This contract is for **human teams**: several owners share one treasury and must agree before money moves.
- **Boundless-SmartWallet** is a single-owner smart wallet. This contract has N owners and an approval threshold.
- Plain **multisig** only checks signatures. Here the treasury also enforces a **spending limit per asset per period**, even when every owner agrees.

## How it works

```
propose  ->  approve (x N)  ->  execute
```

1. An owner calls `propose(asset, to, amount, ttl_secs)`. Their approval counts.
2. Other owners call `approve(id)` until `threshold` owners have approved.
3. An owner calls `execute(id)`. The treasury pays out only if the amount fits the asset's limit for the current period.

Policies (spending limits) are changed with `set_spend_limit` / `remove_spend_limit`. These need `threshold` distinct owners to authorize in the same call.

The treasury is a normal contract that holds tokens. Execution is the contract transferring from its own address, so there is no `__check_auth` custom account in this MVP.

## Functions

| Function | Who | What |
|---|---|---|
| `initialize(owners, threshold)` | anyone, once | Set owners (1 to 10) and threshold |
| `set_spend_limit(approvers, asset, limit, period)` | threshold owners | Max `limit` per `period` seconds |
| `remove_spend_limit(approvers, asset)` | threshold owners | Asset can no longer be spent |
| `propose(proposer, asset, to, amount, ttl_secs)` | owner | Returns proposal id. TTL up to 30 days |
| `approve(owner, id)` | owner | Add an approval |
| `execute(executor, id)` | owner | Pay out if approved, unexpired and under the limit |
| `get_config`, `get_proposal`, `proposal_count`, `get_spend_limit`, `get_policy_assets`, `get_spent` | anyone | Read state |

## Rules

- An asset with no policy cannot be spent (default deny).
- A proposal above the asset's limit cannot be created.
- The limit is checked again at execution, so lowering or removing a limit affects pending proposals.
- Proposals expire. Expired or executed proposals can't be approved or executed.
- At most 10 owners and 10 policies.

## Storage

All state is in persistent storage. Every write extends the entry TTL (and the contract instance TTL) to about 60 days.

| Key | Value |
|---|---|
| `Config` | owners, threshold |
| `PolicyAssets` | assets that have a limit |
| `Policy(asset)` | limit, period |
| `Spent(asset)` | start of current period, amount spent |
| `ProposalCount` | next proposal id |
| `Proposal(id)` | proposer, asset, to, amount, approvals, executed, expires_at |

## Events

`initialized`, `limit_set`, `limit_removed`, `proposed`, `approved`, `executed`. Proposal events carry the proposal `id` as a topic, policy events carry the `asset`.

## Errors

See [`src/types.rs`](src/types.rs) (`Error`, codes 1 to 19).

## Build and test

```
cargo test
stellar contract build      # needs stellar-cli v25.2.0+
```

Tests cover the happy path, below-threshold, over-limit (single and cumulative), duplicate approval, non-owner calls, expired proposals, policy changes and bounds.

## Not in the MVP

Timelock, recipient allowlist, batch payments, upgrades, changing owners.

Open issues: `TODO(wave: contracts-1)` allowlist policy, `contracts-2` timelock policy, `contracts-3` batch payment proposal.
