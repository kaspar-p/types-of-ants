PROJECT = ant-on-the-web

-include ../../Makefile.common

# Default to debug builds
all: debug

debug: BUILD_MODE = debug
debug: binary

release: BUILD_MODE = release
release: RUST_FLAGS += --release
release: binary

clean:
	rm -rf build
	cd ./server && cargo clean
	$(MAKE) -C ./website clean

run: debug
	cd $(BUILD_DIR)/debug && ./$(PROJECT)

binary:
	@echo "BUILDING BINARY" >> /dev/stderr
	@cross build --target=$(TARGET) $(RUST_FLAGS) -p $(PROJECT)
	@mkdir -p $(BUILD_DIR)/$(BUILD_MODE)
	@cp ../../target/$(TARGET)/$(BUILD_MODE)/$(PROJECT) $(BUILD_DIR)/$(BUILD_MODE)/$(PROJECT)

.PHONY: all clean debug release install binary
