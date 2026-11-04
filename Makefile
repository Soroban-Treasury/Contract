.PHONY: test lint fmt build deploy-testnet deploy-testnet-dry deploy-mainnet-dry

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

build:
	sh scripts/build.sh

deploy-testnet-dry:
	sh scripts/deploy.sh testnet --dry-run

deploy-testnet:
	sh scripts/deploy.sh testnet

deploy-mainnet-dry:
	sh scripts/deploy.sh mainnet --dry-run
