.PHONY: build run debug fmt lint test test-cov bench clean

build:
	cargo build --release --all-features

debug: tmp/yc-companies.csv
	RUST_LOG=keyhunter=debug RUST_BACKTRACE=1 cargo run --features report --example yc_startups

fmt:
	taplo format
	cargo fmt

lint:
	taplo lint
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all-features

# run tests and collect coverage. Generates tarpaulin-report.html
test-cov:
	RUST_BACKTRACE=1 cargo tarpaulin --all-features --out Html --skip-clean

bench:
	cargo codspeed build
	cargo codspeed run

clean:
	rm -rf tmp tarpaulin-report.html target/sites

# ==============================================================================

.PHONY: yc yc-companies.csv

yc: tmp/yc-companies.csv
	RUST_BACKTRACE=1 cargo run --release --all-features --example yc_startups

yc-companies.csv: tmp/yc-companies.csv
tmp/yc-companies.csv:
	node ./tasks/get-yc-companies.js

src/config/gitleaks.toml:
	node ./tasks/update-gitleaks.js
