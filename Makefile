.PHONY: build run debug fmt lint clean

build:
	cargo build --release

debug:
	RUST_LOG=keyhunter=trace RUST_BACKTRACE=1 cargo run --example yc_startups

fmt:
	taplo format
	cargo fmt

lint:
	taplo lint
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

clean:
	rm -rf tmp

# ==============================================================================

.PHONY: yc yc-companies.csv

yc: tmp/yc-companies.csv
	RUST_LOG=keyhunter=debug RUST_BACKTRACE=1 cargo run --release --example yc_startups

yc-companies.csv: tmp/yc-companies.csv
tmp/yc-companies.csv:
	node ./tasks/get-yc-companies.js

src/config/gitleaks.toml:
	node ./tasks/update-gitleaks.js
