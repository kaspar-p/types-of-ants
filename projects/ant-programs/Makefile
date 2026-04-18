PROJECT = ant-programs

-include ../../Makefile.common

# Default to debug builds
all: debug

debug: BUILD_MODE = debug
debug: copy-files

release: BUILD_MODE = release
release: copy-files

clean:
	rm -rf build

copy-files:
	@echo "COPYING FILES" >> /dev/stderr
	@mkdir -p $(BUILD_DIR)/$(BUILD_MODE)
	@cp ./src/* $(BUILD_DIR)/$(BUILD_MODE)/

.PHONY: all clean debug release copy-files
