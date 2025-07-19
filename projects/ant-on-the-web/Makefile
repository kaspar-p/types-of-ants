PROJECT = ant-on-the-web

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
	cd ./server && cargo clean
	$(MAKE) -C ./website clean

run: debug
	cd $(BUILD_DIR)/debug && ./$(PROJECT)

SERVER_SOURCES := $(shell find ./server)
$(BUILD_DIR)/%/$(PROJECT): $(BUILD_DIR)/%/$(STATIC)/ $(SERVER_SOURCES)
	cd ./server && cross build --target=$(TARGET) $(RUST_FLAGS)
	mkdir -p $(@D)
	cp ../../target/$(TARGET)/$(BUILD_MODE)/$(PROJECT) $@

WEBSITE_SOURCES := $(shell find ./website)
$(BUILD_DIR)/%/$(STATIC)/: $(WEBSITE_SOURCES)
	$(MAKE) -C ./website
	mkdir -p $@
	cp -R ./website/out/* $@

.PHONY: all clean debug release install
