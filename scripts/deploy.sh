#!/bin/sh
# Deploy a treasury to testnet or mainnet and record it in deployments/<network>.json.
#
#   ./scripts/deploy.sh testnet [--dry-run] [--force]
#   ./scripts/deploy.sh mainnet [--dry-run] [--force]
#
# Settings come from the environment or a .env file (see .env.example).
#
# Setup runs in the contract constructor, in the same transaction as the deployment, so nobody can
# initialize the treasury before you.
#
# Mainnet moves real funds and cannot be undone, so it asks for two things first:
#   - AUDIT_REPORT_URL set to the audit report, or ACKNOWLEDGE_UNAUDITED=yes
#   - typing DEPLOY MAINNET at the prompt
set -eu

cd "$(dirname "$0")/.."

NETWORK=""
DRY_RUN=0
FORCE=0
for arg in "$@"; do
  case "$arg" in
    testnet | mainnet) NETWORK="$arg" ;;
    --dry-run) DRY_RUN=1 ;;
    --force) FORCE=1 ;;
    -h | --help)
      sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      exit 1
      ;;
  esac
done
[ -n "$NETWORK" ] || {
  echo "Usage: $0 <testnet|mainnet> [--dry-run] [--force]" >&2
  exit 1
}

if [ -f .env ]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env
  set +a
fi

case "$NETWORK" in
  testnet)
    PASSPHRASE="Test SDF Network ; September 2015"
    RPC_URL="${RPC_URL:-https://soroban-testnet.stellar.org}"
    EXPLORER="https://stellar.expert/explorer/testnet"
    ;;
  mainnet)
    PASSPHRASE="Public Global Stellar Network ; September 2015"
    RPC_URL="${RPC_URL:-https://mainnet.sorobanrpc.com}"
    EXPLORER="https://stellar.expert/explorer/public"
    ;;
esac

SOURCE="${SOURCE:-treasury-deployer}" # name of a stellar-cli identity
OWNERS="${OWNERS:-}"                  # comma-separated G... addresses
THRESHOLD="${THRESHOLD:-}"
LIMIT_AMOUNT="${LIMIT_AMOUNT:-}"      # in the asset's smallest unit (stroops for XLM)
LIMIT_PERIOD_SECS="${LIMIT_PERIOD_SECS:-86400}"
LIMIT_ASSET="${LIMIT_ASSET:-}"        # token contract id; default: the network's XLM

say() { printf '%s\n' "$*" >&2; }
step() { printf '\n==> %s\n' "$*" >&2; }
# Print a command instead of running it under --dry-run.
run() {
  if [ "$DRY_RUN" -eq 1 ]; then
    printf '  [dry-run] %s\n' "$*" >&2
    return 0
  fi
  "$@"
}

: "${OWNERS:?set OWNERS to a comma-separated list of owner G... addresses}"
: "${THRESHOLD:?set THRESHOLD to the number of approvals needed}"
: "${LIMIT_AMOUNT:?set LIMIT_AMOUNT to the spending limit per period (smallest unit)}"

DEPLOYMENTS="deployments/$NETWORK.json"
if [ -f "$DEPLOYMENTS" ] && [ "$FORCE" -eq 0 ] && [ "$DRY_RUN" -eq 0 ]; then
  say "$DEPLOYMENTS already exists. Pass --force to replace it (the old one stays in git history)."
  exit 1
fi

step "Network: $NETWORK ($RPC_URL)"
run stellar network add "$NETWORK" --rpc-url "$RPC_URL" --network-passphrase "$PASSPHRASE"

if [ -z "$LIMIT_ASSET" ]; then
  if [ "$DRY_RUN" -eq 1 ]; then
    LIMIT_ASSET="<native XLM contract id>"
  else
    LIMIT_ASSET=$(stellar contract id asset --asset native --network "$NETWORK")
  fi
fi

# JSON arguments for the constructor.
OWNERS_JSON=$(printf '%s' "$OWNERS" | sed 's/[[:space:]]//g; s/,/","/g; s/^/["/; s/$/"]/')
LIMITS_JSON=$(printf '[{"asset":"%s","limit":"%s","period":%s}]' "$LIMIT_ASSET" "$LIMIT_AMOUNT" "$LIMIT_PERIOD_SECS")

step "Settings"
say "  deployer identity : $SOURCE"
say "  owners            : $OWNERS"
say "  approvals needed  : $THRESHOLD"
say "  limit             : $LIMIT_AMOUNT per $LIMIT_PERIOD_SECS s of $LIMIT_ASSET"

if [ "$NETWORK" = mainnet ] && [ "$DRY_RUN" -eq 0 ]; then
  if [ -z "${AUDIT_REPORT_URL:-}" ] && [ "${ACKNOWLEDGE_UNAUDITED:-}" != "yes" ]; then
    say ""
    say "Mainnet needs a link to the audit report (AUDIT_REPORT_URL=...)."
    say "To deploy without one, set ACKNOWLEDGE_UNAUDITED=yes and accept the risk."
    exit 1
  fi
  say ""
  say "This deploys to MAINNET and spends real XLM from the '$SOURCE' identity."
  printf 'Type exactly "DEPLOY MAINNET" to continue: ' >&2
  read -r reply
  [ "$reply" = "DEPLOY MAINNET" ] || {
    say "Aborted."
    exit 1
  }
fi

step "Building"
if [ "$DRY_RUN" -eq 1 ]; then
  say "  [dry-run] sh scripts/build.sh"
  WASM="target/wasm32v1-none/release/treasury_contract.optimized.wasm"
else
  sh scripts/build.sh >&2
  WASM="target/wasm32v1-none/release/treasury_contract.optimized.wasm"
fi

step "Uploading wasm"
if [ "$DRY_RUN" -eq 1 ]; then
  say "  [dry-run] stellar contract upload --wasm $WASM --source $SOURCE --network $NETWORK"
  WASM_HASH="<wasm hash>"
else
  WASM_HASH=$(stellar contract upload --wasm "$WASM" --source "$SOURCE" --network "$NETWORK")
fi
say "  wasm hash: $WASM_HASH"

step "Deploying (the constructor sets owners, threshold and limits)"
if [ "$DRY_RUN" -eq 1 ]; then
  say "  [dry-run] stellar contract deploy --wasm-hash $WASM_HASH --source $SOURCE --network $NETWORK -- --owners '$OWNERS_JSON' --threshold $THRESHOLD --limits '$LIMITS_JSON'"
  CONTRACT_ID="<contract id>"
else
  CONTRACT_ID=$(stellar contract deploy --wasm-hash "$WASM_HASH" --source "$SOURCE" --network "$NETWORK" \
    -- --owners "$OWNERS_JSON" --threshold "$THRESHOLD" --limits "$LIMITS_JSON")
fi
say "  contract id: $CONTRACT_ID"

step "Checking the deployed treasury"
if [ "$DRY_RUN" -eq 0 ]; then
  VERSION=$(stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" -- contract_version)
  say "  contract_version: $VERSION"
  stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" -- get_config >&2
fi

if [ "$DRY_RUN" -eq 1 ]; then
  step "Dry run finished. Nothing was deployed or written."
  exit 0
fi

step "Recording $DEPLOYMENTS"
mkdir -p deployments
cat >"$DEPLOYMENTS" <<EOF
{
  "network": "$NETWORK",
  "contract_id": "$CONTRACT_ID",
  "wasm_hash": "$WASM_HASH",
  "deployer": "$(stellar keys address "$SOURCE")",
  "owners": $OWNERS_JSON,
  "threshold": $THRESHOLD,
  "limits": $LIMITS_JSON,
  "deployed_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "explorer": "$EXPLORER/contract/$CONTRACT_ID"
}
EOF
cat "$DEPLOYMENTS"
say ""
say "Done. Add the contract id to TREASURY_IDS for the backend, and open it in the app."
