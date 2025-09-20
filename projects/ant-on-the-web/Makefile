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

binary: $(BUILD_DIR)/%/$(STATIC)/
	@echo "BUILDING BINARY" >> /dev/stderr
	@cd ./server && cross build --target=$(TARGET) $(RUST_FLAGS) -p $(PROJECT)
	@mkdir -p $(@D)
	@cp ../../target/$(TARGET)/$(BUILD_MODE)/$(PROJECT) $(BUILD_DIR)/$(BUILD_MODE)/$(PROJECT)

WEBSITE_SOURCES := $(shell find ./website)
$(BUILD_DIR)/%/$(STATIC)/: $(WEBSITE_SOURCES)
	@echo "BUILDING STATIC CONTENT" >> /dev/stderr
	@$(MAKE) -C ./website
	@mkdir -p $(BUILD_DIR)/$(BUILD_MODE)/$(STATIC)
	@cp -R ./website/out/* $(BUILD_DIR)/$(BUILD_MODE)/$(STATIC)

.PHONY: all clean debug release install binary
