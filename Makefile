.PHONY: run debug fmt yc clean

run:
	RUST_LOG=keyhunter::extract=trace RUST_BACKTRACE=1 cargo run --release --example yc_startups
debug:
	RUST_LOG=keyhunter=trace RUST_BACKTRACE=1 cargo run --example yc_startups

fmt:
	taplo format
	cargo fmt


yc: tmp/yc-companies.csv
tmp/yc-companies.csv:
	node ./tasks/get-yc-companies.js

clean:
	rm -rf tmp
