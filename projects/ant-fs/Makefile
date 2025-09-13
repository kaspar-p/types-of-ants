PROJECT = ant-fs

-include ../../Makefile.common

all: debug

debug: BUILD_MODE = debug
debug: binary

release: BUILD_MODE = release
release: RUST_FLAGS += --release
release: binary

clean:
	rm -rf build

binary:
	echo "BUILDING BINARY"
	cross build --target=$(TARGET) $(RUST_FLAGS) -p $(PROJECT)
	mkdir -p $(BUILD_DIR)/$(BUILD_MODE)
	mkdir $(BUILD_DIR)/$(BUILD_MODE)/fs
	touch $(BUILD_DIR)/$(BUILD_MODE)/fs/.keep
	cp ../../target/$(TARGET)/$(BUILD_MODE)/$(PROJECT) $(BUILD_DIR)/$(BUILD_MODE)/$(PROJECT)

.PHONY: all release debug binary
