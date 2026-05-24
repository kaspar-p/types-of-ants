PROJECT = ant-looking-pretty

-include ../../Common.mk

# Default to debug builds
all: debug

debug: BUILD_MODE = debug
debug: out

release: BUILD_MODE = release
release: out

out: | build/.cache/$(ARCH)/ $(BUILD_OUTPUT_DIR)
	@echo "installing..." >> /dev/stderr
	@npm ci >> /dev/stderr

	@echo "building..." >> /dev/stderr
	@npm run build >> /dev/stderr

	@echo "copying..." >> /dev/stderr
	@cp -r .next/standalone/ $(BUILD_OUTPUT_DIR)/.
	@cp -r public $(BUILD_OUTPUT_DIR)/
	@cp -r .next/static $(BUILD_OUTPUT_DIR)/.next

clean:
	@npm run clean && rm -rf build

.PHONY: all debug release clean out
