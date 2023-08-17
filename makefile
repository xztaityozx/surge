SHELL=bash
build:
	cargo build

test:
	cargo test

strip:
	@./scripts/strip.sh $(TARGET)

build-release:
	@cargo build --release --target $(TARGET) --locked

archive: build-release
	@./scripts/archive.sh $(TARGET) $(VERSION) $(BIN_PATH)
