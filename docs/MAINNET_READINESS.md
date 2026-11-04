# Mainnet readiness

A checklist to work through before a treasury holds real value. `scripts/deploy.sh mainnet` enforces the audit line and a typed confirmation.

## Code

- [ ] Independent audit by a third-party firm; report linked from the README Security section and from `AUDIT_REPORT_URL` when deploying
- [ ] Every finding in the audit fixed or accepted in writing; the fixes re-reviewed
- [x] Internal pre-audit done: [`Audits/pre-audit-2026-11-04.md`](../Audits/pre-audit-2026-11-04.md)
- [x] Tests for every error and every state change; clippy clean
- [x] Setup in a constructor (no front-running of initialization)
- [x] Bounded storage, checked arithmetic, no `unwrap` or `expect`
- [ ] Soak period on testnet with real owners using the app and API
- [ ] Reproducible build: build twice on clean machines and compare the wasm SHA-256 (`scripts/build.sh` prints it)

## Deployment

- [ ] Deployer key is a fresh key, funded with only enough XLM for fees
- [ ] Owner keys are on hardware wallets or otherwise held by separate people
- [ ] Threshold chosen so that up to `owners - threshold` lost keys still leave enough owners to approve, and `threshold - 1` stolen keys cannot spend
- [ ] Spending limit sized against the amount held, not above it
- [ ] RPC provider chosen (mainnet needs a provider; the public testnet URL does not serve mainnet)
- [ ] `scripts/deploy.sh mainnet --dry-run` reviewed, then run; `deployments/mainnet.json` committed
- [ ] Deployed wasm hash matches the audited build
- [ ] Explorer page checked: owners, threshold and limit are what you intended

## After deployment

- [ ] `treasury-api` indexing the treasury, webhook alerts on `proposal_created` and `proposal_executed`
- [ ] `docs/DEPLOYMENTS.md` and `SECURITY.md` list the mainnet address
- [ ] A test payment of a small amount made and checked end to end
- [ ] Someone owns incident response: who can be reached, and what the plan is if an owner key is lost or leaked

## Known limits to plan around

These are documented in the pre-audit report.

- Owners and threshold are fixed at deployment. A lost or leaked key cannot be rotated; the response is a new treasury and moving funds.
- The spending limit is a fixed window, so just before and just after a window boundary up to twice the limit can leave the treasury.
- There is no upgrade path. A vulnerability is fixed by deploying a new version and migrating.
