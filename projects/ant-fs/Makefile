PROJECT = ant-fs

-include ../../Common.mk

all: debug

debug: BUILD_MODE = debug
debug: $(BUILD_OUTPUT_DIR)/$(PROJECT)

release: BUILD_MODE = release
release: RUST_FLAGS += --release
release: $(BUILD_OUTPUT_DIR)/$(PROJECT)

clean:
	rm -rf build

$(BUILD_OUTPUT_DIR)/$(PROJECT): | $(BUILD_OUTPUT_DIR)
	@echo "BUILDING BINARY" >> /dev/stderr
	@cross build --target=$(RUST_TARGET) $(RUST_FLAGS) -p $(PROJECT)
	@cp ../../target/$(RUST_TARGET)/$(BUILD_MODE)/$(PROJECT) $(BUILD_OUTPUT_DIR)/$(PROJECT)

.PHONY: all release debug binary
