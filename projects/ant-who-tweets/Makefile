PROJECT_ROOT = $(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))
PROJECT = ant-who-tweets

-include $(PROJECT_ROOT)/../../Makefile.common

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

install: INSTALL_DIR = $(INSTALL_PREFIX)/$(PROJECT)/$(INSTALL_VERSION)/
install: release
	mkdir -p $(INSTALL_DIR)
	cp $(PROJECT_ROOT)/../../.env $(INSTALL_DIR)
	install -v -m 0755 $(BUILD_DIR)/release/$(PROJECT) $(INSTALL_DIR)
	HOME=$(HOME) VERSION=$(INSTALL_VERSION) mo $(PROJECT).service.mo > $(INSTALL_DIR)/$(PROJECT).service

$(BUILD_DIR)/%/$(PROJECT):
	cargo build $(RUST_FLAGS)
	mkdir -p $(@D)
	cp $(PROJECT_ROOT)/../../target/$(BUILD_MODE)/$(PROJECT) $@

.PHONY: all debug release clean
