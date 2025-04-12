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
# cd ./server && cargo clean
# $(MAKE) -C ./website clean

install: INSTALL_DIR = $(INSTALL_PREFIX)/$(PROJECT)/$(INSTALL_VERSION)/
install: release
	echo $(BUILD_DIR) $(BUILD_MODE) $(RUST_FLAGS) $(INSTALL_DIR)
	mkdir -p $(INSTALL_DIR)/$(STATIC)
	cp -R $(BUILD_DIR)/release/$(STATIC) $(INSTALL_DIR)/$(STATIC)
	cp ../../.env $(INSTALL_DIR)
	install -v -m 0755 $(BUILD_DIR)/release/$(PROJECT) $(INSTALL_DIR)
	HOME=$(HOME) VERSION=$(INSTALL_VERSION) mo $(PROJECT).service.mo > $(INSTALL_DIR)/$(PROJECT).service

$(BUILD_DIR)/%/$(PROJECT): $(BUILD_DIR)/%/$(STATIC)/ ./server/**
	cd ./server && cargo build $(RUST_FLAGS)
	mkdir -p $(@D)
	cp ../../target/$(BUILD_MODE)/$(PROJECT) $@

$(BUILD_DIR)/%/$(STATIC)/:
	$(MAKE) -C ./website
	mkdir -p $(@D)
	cp -R ./website/out $@

.PHONY: all clean debug release install
