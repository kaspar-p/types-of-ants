PROJECT = ant-host-agent

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
	cargo clean

run: debug
	cd $(BUILD_DIR)/debug && ./$(PROJECT)

$(BUILD_DIR)/%/$(PROJECT): src/** Cargo.toml
	@cross build --target=$(TARGET) $(RUST_FLAGS)
	@mkdir -p $(@D)
	@cp ../../target/$(TARGET)/$(BUILD_MODE)/$(PROJECT) $@

.PHONY: all clean debug release install
