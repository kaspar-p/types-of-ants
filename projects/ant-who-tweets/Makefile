PROJECT = ant-who-tweets

-include ../../Makefile.common

# Default to debug builds
all: debug

debug: BUILD_MODE = debug
debug: $(BUILD_DIR)/debug/$(PROJECT)

release: BUILD_MODE = release
release: RUST_FLAGS += --release
release: $(BUILD_DIR)/release/$(PROJECT)

clean:
	rm -rf build
# cargo clean

$(BUILD_DIR)/%/$(PROJECT): src/** Cargo.toml
	cargo build $(RUST_FLAGS)
	mkdir -p $(@D)
	cp ../../target/$(BUILD_MODE)/$(PROJECT) $@

.PHONY: all debug release clean
