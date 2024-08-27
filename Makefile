.PHONY: init check build run debug fmt lint fix test test-cov bench clean purge

init:
	cargo binstall cargo-nextest -y

check:
	cargo check --all-features --all-targets

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

fix:
	cargo clippy --fix --allow-staged --no-deps --all-targets --all-features
	cargo fmt
	taplo fmt
	git status

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

purge:
	make clean
	cargo clean

# ==============================================================================

.PHONY: yc yc-companies.csv

yc: tmp/yc-companies.csv
	RUST_BACKTRACE=1 cargo run --release --all-features --example yc_startups

yc-companies.csv: tmp/yc-companies.csv
tmp/yc-companies.csv:
	node ./tasks/get-yc-companies.js

src/config/gitleaks.toml:
	node ./tasks/update-gitleaks.js
