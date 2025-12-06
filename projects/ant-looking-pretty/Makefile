PROJECT = ant-looking-pretty

-include ../../Makefile.common

# Default to debug builds
all: debug

debug: BUILD_MODE = debug
debug: out

release: BUILD_MODE = release
release: out

out:
	mkdir -p $(BUILD_DIR)/$(BUILD_MODE)
	@npm ci --silent >> /dev/stderr
	@npm run build
	@sed -i '' 's/process\.env\.PORT/process\.env\.ANT_LOOKING_PRETTY_PORT/' .next/standalone/server.js
	@cp -r .next/standalone/ $(BUILD_DIR)/$(BUILD_MODE)/.
	@cp -r public $(BUILD_DIR)/$(BUILD_MODE)/
	@cp -r .next/static $(BUILD_DIR)/$(BUILD_MODE)/.next

clean:
	@npm run clean && rm -rf build

.PHONY: all debug release clean out
