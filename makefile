SHELL=bash
build:
	cargo build

test: test-main test-output_stream test-sub_process

test-main:
	cargo test

test-output_stream:
	cd output_stream;cargo test

test-sub_process:
	cd sub_process;cargo test

strip:
	@./scripts/strip.sh $(TARGET)

