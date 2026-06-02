.PHONY: help test test-integration test-integration-docker

help:
	@echo "marketd - Available make targets:"
	@echo ""
	@echo "  test                       - Run the fast (ungated) tests"
	@echo "  test-integration           - Run the gated e2e test locally"
	@echo "                                requires BITCOIND_EXE to be set"
	@echo "  test-integration-docker    - Build and run the e2e test inside a"
	@echo "                                hermetic container (bundles bitcoind"
	@echo "                                and nostr-rs-relay)"

test:
	cargo test

# Local runner — pass through to cargo so flags like `-- --nocapture` work.
test-integration:
	cargo test --test e2e --features integration-test -- --nocapture

# Self-contained container with bitcoind + nostr-rs-relay pre-installed.
test-integration-docker:
	docker build -t marketd-integration-test -f docker/Dockerfile.integration-test .
	docker run --rm marketd-integration-test
