# Deployments

Every deployment made with `scripts/deploy.sh` is recorded in `deployments/<network>.json`. This page explains them.

## Mainnet

`deployments/mainnet.json` is added when a treasury is deployed there. See [MAINNET_READINESS.md](MAINNET_READINESS.md) for what to complete first.

## Testnet

| Field | Value |
|---|---|
| Network | Testnet (`Test SDF Network ; September 2015`), RPC `https://soroban-testnet.stellar.org` |
| **Contract ID** | `CBK7HRYROYMGDFPN2ISHW7NHK7O4Z4TVGK4S3HJTKGZX6ZX5A7QFBWRI` |
| Explorer | https://stellar.expert/explorer/testnet/contract/CBK7HRYROYMGDFPN2ISHW7NHK7O4Z4TVGK4S3HJTKGZX6ZX5A7QFBWRI |
| Deploy tx | https://stellar.expert/explorer/testnet/tx/0799fbd5a806a7c1c663a0775d3b962286a6d7e710b7e09b4089c8c8ec094d33 |
| WASM hash | `1f6e5b53c3ad153ac813a0f15beeac352b7e72b8731737f69e50d2c7bf1b640b` |
| WASM size | 13,795 bytes (limit 131,072) |
| Contract version | 1 |
| Owners | `GCD5TVTFLXI22TPDR22P7IISDXR52FGO2IR657CXQMSYAJPTZ537SFB2`, `GA277IVPC3N2U7M7VAQ53ZE5XR73EP72QGWZ3FH52HE7QB57KL6KKMMD` |
| Approvals needed | 2 |
| Limit | 50 XLM per 24 hours (`CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`, the testnet XLM contract) |

This treasury is a smoke test. Its owner keys are throwaway testnet keys that hold no value. Testnet is reset from time to time, so the address may stop existing.

## The deployment record

```json
{
  "network": "testnet",
  "contract_id": "C...",
  "wasm_hash": "...",
  "deployer": "G...",
  "owners": ["G...", "G..."],
  "threshold": 2,
  "limits": [{ "asset": "C...", "limit": "500000000", "period": 86400 }],
  "deployed_at": "2026-11-04T12:27:30Z",
  "explorer": "https://stellar.expert/..."
}
```

`scripts/deploy.sh` refuses to replace an existing record unless you pass `--force`. The old record stays in git history.

## How a deployment works

1. `scripts/build.sh` builds and optimizes the wasm and prints its SHA-256.
2. The wasm is uploaded; its hash is the code identity.
3. The contract is created **with its constructor arguments** (owners, threshold, starting limits) in the same transaction. Nobody can initialize it before you.
4. The script reads back `contract_version` and `get_config` and writes the record.

## Reproduce

```sh
cp .env.example .env              # fill in OWNERS, THRESHOLD, LIMIT_AMOUNT
stellar keys generate treasury-deployer --network testnet --fund
./scripts/deploy.sh testnet --dry-run   # prints every step, sends nothing
./scripts/deploy.sh testnet
```

Anyone can check a deployment: build the wasm, compare its SHA-256 with `wasm_hash`, and read `get_config` from the contract.

## Using a deployment

- **App:** open the treasury by its contract id.
- **API:** add the id to `TREASURY_IDS`.
- **SDK:** `new TreasuryClient({ contractId, rpcUrl, networkPassphrase })`.
