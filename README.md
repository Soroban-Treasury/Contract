# Treasury contract

Policy-controlled team treasury on Soroban (Rust, `#![no_std]`). Part of Stellar Treasury.

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

The owners, the threshold and the starting spending limits are set in the **constructor**, in the same transaction that deploys the contract, so nobody can set it up before you. Limits can be changed later with `set_spend_limit` / `remove_spend_limit`, which need `threshold` distinct owners to authorize in the same call, so set the limits you need at deployment.

The treasury is a normal contract that holds tokens. Execution is the contract transferring from its own address, so there is no `__check_auth` custom account in this MVP.

## Functions

| Function | Who | What |
|---|---|---|
| `__constructor(owners, threshold, limits)` | at deployment | Set owners (1 to 10), threshold and starting spending limits (up to 10, one per asset) |
| `set_spend_limit(approvers, asset, limit, period)` | threshold owners | Max `limit` per `period` seconds |
| `remove_spend_limit(approvers, asset)` | threshold owners | Asset can no longer be spent |
| `propose(proposer, asset, to, amount, ttl_secs)` | owner | Returns proposal id. TTL up to 30 days |
| `approve(owner, id)` | owner | Add an approval |
| `execute(executor, id)` | owner | Pay out if approved, unexpired and under the limit |
| `get_config`, `get_proposal`, `proposal_count`, `get_spend_limit`, `get_policy_assets`, `get_spent`, `contract_version` | anyone | Read state |

## Rules

- An asset with no policy cannot be spent (default deny).
- A proposal above the asset's limit cannot be created.
- The limit is checked again at execution, so lowering or removing a limit affects pending proposals.
- Proposals expire. Expired or executed proposals can't be approved or executed.
- At most 10 owners and 10 policies.

## Deploy

```
cp .env.example .env                      # OWNERS, THRESHOLD, LIMIT_AMOUNT
sh scripts/deploy.sh testnet --dry-run    # prints every step, sends nothing
sh scripts/deploy.sh testnet              # records deployments/testnet.json
sh scripts/deploy.sh mainnet              # asks for the audit link and a typed confirmation
```

Details, the recorded testnet deployment and how to verify a deployment: [docs/DEPLOYMENTS.md](docs/DEPLOYMENTS.md).

## Docs

| | |
|---|---|
| [docs/STORAGE.md](docs/STORAGE.md) | Storage keys, bounds, TTL, events |
| [docs/DEPLOYMENTS.md](docs/DEPLOYMENTS.md) | Deployments and how to reproduce one |
| [docs/TESTING.md](docs/TESTING.md) | What the tests cover |
| [docs/MAINNET_READINESS.md](docs/MAINNET_READINESS.md) | Checklist before real value |
| [Audits/](Audits/) | Internal pre-audit report |
| [SECURITY.md](SECURITY.md) | Reporting a vulnerability |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to contribute, open tasks |

## Build and test

```
cargo test
sh scripts/build.sh         # needs stellar-cli v25.2.0+
```

## Security

The contract holds funds, so treat it accordingly.

- **Audit status:** not yet independently audited. An audit by a third-party security firm is the step before holding significant value on mainnet. This section will link the report once it exists.
- **What is checked today:** the full test suite (including approval, limit, expiry and non-owner cases), clippy with warnings denied, bounded storage, checked arithmetic, an [internal pre-audit](Audits/pre-audit-2026-11-04.md), and a live run on Stellar testnet.
- **Report a vulnerability:** see [SECURITY.md](SECURITY.md).

## Not in the MVP

Timelock, recipient allowlist, batch payments, upgrades, changing owners.

Open issues: `TODO(wave: contracts-1)` allowlist policy, `contracts-2` timelock policy, `contracts-3` batch payment proposal.
