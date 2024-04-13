.PHONY: run debug fmt

run:
	RUST_BACKTRACE=1 cargo run --release --example yc_startups
debug:
	RUST_BACKTRACE=1 cargo run --example yc_startups

fmt:
	taplo format
	cargo fmt
