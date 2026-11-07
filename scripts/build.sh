#!/bin/sh
# Build and optimize the treasury contract.
#
#   ./scripts/build.sh
#
# Needs stellar-cli. Prints the size and SHA-256 of the optimized wasm, so a deployment can be
# checked against a build anyone can reproduce.
set -eu

cd "$(dirname "$0")/.."

NAME=treasury_contract
OUT=target/wasm32v1-none/release

stellar contract build
# The deployed code hash comes from this extra optimize pass. Keep it until the code is redeployed:
# `stellar contract build --optimize` alone gives a different file, and so a different hash.
stellar contract optimize --wasm "$OUT/$NAME.wasm"

WASM="$OUT/$NAME.optimized.wasm"
SIZE=$(wc -c <"$WASM" | tr -d ' ')
if command -v sha256sum >/dev/null 2>&1; then
  HASH=$(sha256sum "$WASM" | cut -d' ' -f1)
else
  HASH=$(shasum -a 256 "$WASM" | cut -d' ' -f1)
fi

echo "wasm:   $WASM"
echo "size:   $SIZE bytes (network limit 131072)"
echo "sha256: $HASH"
