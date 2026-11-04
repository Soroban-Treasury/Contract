# Testing

```
cargo test                                   # unit tests, no network
cargo clippy --all-targets -- -D warnings    # lints, warnings are errors
cargo fmt --check
```

CI runs all three on every push.

## What the tests cover

Tests live in [`src/test.rs`](../src/test.rs) and run against Soroban's in-process test environment with real token (Stellar Asset) contracts.

| Area | Cases |
|---|---|
| Deployment | owners and limits stored; threshold above owner count, zero threshold, no owners, duplicate owners, too many owners; zero limit, zero period, the same asset twice, too many limits (each aborts deployment) |
| Happy path | propose, approve, execute; payout reaches the recipient; a threshold-2 treasury spends without any policy change |
| Approval | execute below threshold; duplicate approval; non-owner propose, approve and execute; approving or executing an executed proposal |
| Limits | one proposal over the limit; cumulative spend over the limit in one period; the period rolling over; lowering a limit affects pending proposals; removing a limit blocks pending proposals |
| Expiry | approve and execute after a proposal expired |
| Inputs | zero and negative amounts; zero and too-long lifetimes; asset without a limit; missing proposal |
| Policy changes | needs `threshold` distinct owners; invalid values; the 10-limit cap |
| Events | each state change emits an event |

## Testing against a network

[`sdk/scripts/e2e-testnet.mjs`](https://github.com/Soroban-Treasury/SDK/blob/main/scripts/e2e-testnet.mjs) deploys this contract on testnet and runs a 2-of-2 flow through the SDK. `scripts/deploy.sh testnet --dry-run` prints a deployment without sending anything.

## Adding a test

Use `setup(&env)`: a treasury with three owners, threshold 2, a funded asset and a 1000-per-day limit. Check errors with the generated `try_*` client methods and compare to `Err(Ok(Error::...))`. Constructor failures are tested with `#[should_panic(expected = "Error(Contract, #N)")]`.
