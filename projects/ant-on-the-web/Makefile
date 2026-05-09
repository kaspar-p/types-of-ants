PROJECT = ant-on-the-web

-include ../../Common.mk

# Default to debug builds
all: debug

debug: BUILD_MODE = debug
debug: $(BUILD_OUTPUT_DIR)/$(PROJECT)

release: BUILD_MODE = release
release: RUST_FLAGS += --release
release: $(BUILD_OUTPUT_DIR)/$(PROJECT)

clean:
	rm -rf build
	cd ./server && cargo clean
	$(MAKE) -C ./website clean

run: debug
	cd $(BUILD_DIR)/debug && ./$(PROJECT)

$(BUILD_OUTPUT_DIR)/$(PROJECT):
	@echo "BUILDING BINARY" >> /dev/stderr
	@cross build --target=$(RUST_TARGET) $(RUST_FLAGS) -p $(PROJECT)
	@cp ../../target/$(RUST_TARGET)/$(BUILD_MODE)/$(PROJECT) $(BUILD_OUTPUT_DIR)/$(PROJECT)

.PHONY: all clean debug release
