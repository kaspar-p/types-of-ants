PROJECT_ROOT = $(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))

RUST_BUILD = debug

BUILD_DIR = $(PROJECT_ROOT)/build/
BIN = $(BUILD_DIR)/ant-on-the-web
STATIC = $(BUILD_DIR)/static/

# Default to debug builds
all: debug

debug: BUILD_DIR := $(BUILD_DIR)/debug
debug: $(BIN)

release: BUILD_DIR := $(BUILD_DIR)/release
release: RUST_FLAGS += --release
release: RUST_BUILD = release
release: $(BIN)

remake: clean debug
.NOTPARALLEL: remake

clean:
	rm -rf build
	cd $(PROJECT_ROOT)/server && cargo clean
	cd $(PROJECT_ROOT)/website && make clean

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BIN): $(BUILD_DIR) $(STATIC)
	cd $(PROJECT_ROOT)/server && cargo build $(RUST_FLAGS)
	cp $(PROJECT_ROOT)/../../target/$(RUST_BUILD)/ant-on-the-web $(BIN)

$(STATIC): $(BUILD_DIR)
	cd $(PROJECT_ROOT)/website && $(MAKE)
	cp -R $(PROJECT_ROOT)/website/out $(STATIC)

.PHONY: all clean debug release remake
