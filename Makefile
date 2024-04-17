.PHONY: run debug fmt clean

run:
	RUST_LOG=key_hunter::extract=trace RUST_BACKTRACE=1 cargo run --release --example yc_startups
debug:
	RUST_LOG=key_hunter=trace RUST_BACKTRACE=1 cargo run --example yc_startups

fmt:
	taplo format
	cargo fmt

clean:
	rm -rf tmp
