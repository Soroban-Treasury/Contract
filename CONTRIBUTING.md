# Contributing

Thanks for helping. This is the contract repository of Stellar Treasury.

> Open pull requests here. Maintainers also work on this code in the `contract/` folder of the Soroban-Treasury monorepo, and the two are kept in sync automatically: changes merged here are pulled into the monorepo, and changes made there arrive here.

## Setup

You need [rustup](https://rustup.rs) and, to build the wasm, [stellar-cli](https://developers.stellar.org/docs/tools/cli) 25.2 or newer. The Rust version is pinned in `rust-toolchain.toml`, and rustup installs it on first use. To reproduce the exact wasm hash, use stellar-cli 30.0.1, the version CI uses.

```
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
sh scripts/build.sh
```

## Rules for contract changes

- **No `unwrap` or `expect`** in contract code. Return an `Error`.
- **Checked arithmetic** on anything that touches amounts or time.
- **Authorize first:** the owner who acts calls `require_auth`, after being checked against `Config`.
- **Update state before external calls** (the order in `execute`).
- **Bound everything** that can grow: owners, policies, proposal lifetime.
- **Every state change emits an event.** Add it to `docs/STORAGE.md`.
- **Every new error and every new branch needs a test.** See `docs/TESTING.md`.
- Changing stored types or behaviour: bump `CONTRACT_VERSION` and say why in the pull request.

## Open tasks

Marked in the code as `TODO(wave: ...)`:

| Marker | Task |
|---|---|
| `contracts-1` | Recipient allowlist policy |
| `contracts-2` | Timelock policy |
| `contracts-3` | Batch payment proposal |

Other good first issues: more attack-scenario tests (replay, policy bypass, griefing) and sharper docs.

## Pull requests

Describe what changed and why, list the tests you added, and say if storage or events changed. Keep a pull request to one change.

## Conduct

Be kind and be specific. See the [Code of Conduct](CODE_OF_CONDUCT.md).
