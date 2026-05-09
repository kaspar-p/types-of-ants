PROJECT = ant-looking-pretty

-include ../../Common.mk

# Default to debug builds
all: debug

debug: BUILD_MODE = debug
debug: out

release: BUILD_MODE = release
release: out

out: | $(BUILD_OUTPUT_DIR)
	@npm ci --silent >> /dev/stderr
	@npm run build
# 	@sed -i '' 's/process\.env\.PORT/process\.env\.ANT_LOOKING_PRETTY_PORT/' .next/standalone/server.js
	@cp -r .next/standalone/ $(BUILD_OUTPUT_DIR)/.
	@cp -r public $(BUILD_OUTPUT_DIR)/
	@cp -r .next/static $(BUILD_OUTPUT_DIR)/.next

clean:
	@npm run clean && rm -rf build

.PHONY: all debug release clean out
