PROJECT = ant-zookeeper

-include ../../Common.mk

all: debug

debug: BUILD_MODE = debug
debug: binary

release: BUILD_MODE = release
release: RUST_FLAGS += --release
release: binary

clean:
	rm -rf build

binary: | $(BUILD_OUTPUT_DIR)
	@echo "BUILDING BINARY" >> /dev/stderr
	@cross build --target=$(RUST_TARGET) $(RUST_FLAGS) -p $(PROJECT)
	@cp ../../target/$(RUST_TARGET)/$(BUILD_MODE)/$(PROJECT) $(BUILD_OUTPUT_DIR)

.PHONY: all release debug binary
