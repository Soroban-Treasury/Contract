# Storage and events

Reference for what the contract stores, how long it lives, and what it emits.

## Storage

Everything is in **persistent** storage, because treasury state must not disappear. Nothing uses temporary storage. The contract instance (which holds the code reference) is kept alive too.

| Key | Value | Written by |
|---|---|---|
| `Config` | `Config { owners, threshold }` | constructor |
| `PolicyAssets` | `Vec<Address>`: assets that have a limit | constructor, `set_spend_limit`, `remove_spend_limit` |
| `Policy(asset)` | `SpendLimit { limit, period }` | constructor, `set_spend_limit` |
| `Spent(asset)` | `PeriodSpend { start, amount }` | `execute` |
| `ProposalCount` | `u64`, also the next proposal id | constructor, `propose` |
| `Proposal(id)` | `Proposal { id, proposer, asset, to, amount, approvals, executed, expires_at }` | `propose`, `approve`, `execute` |

### Bounds

| Limit | Value | Why |
|---|---|---|
| Owners | 10 | `Config` and every `Proposal.approvals` stay small |
| Assets with a limit | 10 | `PolicyAssets` stays small |
| Proposal lifetime | 1 s to 30 days | proposals cannot stay open forever |

Each proposal is its own entry, so no single entry grows without bound as proposals accumulate.

### TTL

Every write and every read of a key extends its TTL (and the instance TTL) when it has less than the threshold left.

| Constant | Ledgers | About |
|---|---:|---|
| `BUMP_THRESHOLD` | 518,400 | 30 days |
| `BUMP_EXTEND_TO` | 1,036,800 | 60 days |

Ledgers per day is taken as 17,280 (about 5 seconds per ledger). A treasury nobody touches for more than 60 days has its entries archived. They can be restored with a restore transaction, so funds are not lost, but the first call needs the restore.

## Events

Events are published for every state change. Topics are `[name, ...topic fields]`. Data is a map of the remaining fields, with symbol keys in alphabetical order.

| Event | Topics | Data fields |
|---|---|---|
| `initialized` | name | `owners`, `threshold` |
| `limit_set` | name, `asset` | `limit`, `period` |
| `limit_removed` | name, `asset` | none |
| `proposed` | name, `id` | `proposer`, `asset`, `to`, `amount`, `expires_at` |
| `approved` | name, `id` | `owner`, `approvals` |
| `executed` | name, `id` | `executor`, `asset`, `to`, `amount` |

`execute` also causes the token contract to emit its own `transfer` event. `treasury-api` reads the events above to index proposals.

## Errors

Codes are stable and listed in [`src/types.rs`](../src/types.rs). Code `1` (`AlreadyInitialized`) is reserved: setup now happens in the constructor, which can only run once.
