.PHONY: build test docs deploy-testnet stake claim

build:
	cargo build --target wasm32-unknown-unknown --release

test:
	cargo test --features testutils

# Build rustdoc and list undocumented public items (#675).
docs:
	cargo doc --no-deps --lib 2>&1 | tee target/rustdoc.log
	@echo "Undocumented public items: $$(grep -c 'missing documentation' target/rustdoc.log || true)"

deploy-testnet:
	chmod +x scripts/deploy_testnet.sh
	./scripts/deploy_testnet.sh

stake:
	@chmod +x scripts/stake.sh
	@if [ -z "$(AMOUNT)" ]; then \
		echo "Error: AMOUNT is required. Usage: make stake AMOUNT=10000000"; \
		exit 1; \
	fi
	./scripts/stake.sh $(AMOUNT)

claim:
	chmod +x scripts/claim.sh
	./scripts/claim.sh
