build:
	cargo build

test: test-main test-output_stream

test-main:
	cargo test

test-output_stream:
	cd output_stream && cargo test
