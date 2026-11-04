# Security policy

This repository holds the Soroban treasury contract. A treasury holds funds for a team, so reports against this repository are taken seriously.

## Reporting a vulnerability

Please **do not open a public issue**. Open a private report with GitHub's "Report a vulnerability" button on the Security tab of this repository. Include the contract version, what you did, what happened, and what you expected.

You will get a reply in the report thread. Please give us time to fix the problem before you share details.

## Supported versions

| Contract version | Supported |
|---|---|
| 1 | yes |

`contract_version()` returns the version of the deployed code.

## There is no upgrade path

The contract has no upgrade function, by design: nobody, including the maintainers, can swap the code of a deployed treasury. The flip side is that a vulnerability in a deployed treasury cannot be patched in place. The response is a new contract version and a way for owners to move funds. That is a reason to report early.

## Deployed addresses in scope

Listed in [docs/DEPLOYMENTS.md](docs/DEPLOYMENTS.md) and `deployments/`. Mainnet addresses are added there when they exist.

## In scope

- The contract in `src/`: authorization, spending limits, proposal lifecycle, storage and TTL handling.
- Ways to move funds without the required approvals, to exceed a spending limit, to lock a treasury, or to make an executed or expired proposal usable again.

## Out of scope

- Token contracts that owners choose to use.
- Losing or leaking an owner's key (see the known limits in [docs/MAINNET_READINESS.md](docs/MAINNET_READINESS.md)).
- Findings already listed in [Audits/](Audits/) as accepted.
- The SDK, API and app; report those in their own repositories.

## Audit status

An internal review is in [Audits/](Audits/). It is not an independent audit. When an external audit is done, its report will be linked here.
